package store

import (
	"context"
	"errors"
	"strconv"
	"time"

	"github.com/jackc/pgx/v5"
)

// ErrInsufficientCredits is returned when a charge would overdraw the balance.
var ErrInsufficientCredits = errors.New("insufficient credits")

// ReserveGeneration atomically:
//  1. locks the user row and deducts `cost` credits (checked, no overdraft),
//  2. writes a `consume` transaction,
//  3. inserts a generation row in `running` state.
//
// cost == 0 (admin exemption) skips the ledger write but still records the
// generation. Returns the generation id and the user's balance after charge.
func (s *Store) ReserveGeneration(ctx context.Context, userID string, cost int64, kind, model string, n int, paramsJSON []byte) (genID string, balance int64, err error) {
	tx, err := s.Pool.Begin(ctx)
	if err != nil {
		return "", 0, err
	}
	defer tx.Rollback(ctx)

	err = tx.QueryRow(ctx, `SELECT credits FROM users WHERE id = $1 FOR UPDATE`, userID).Scan(&balance)
	if errors.Is(err, pgx.ErrNoRows) {
		return "", 0, ErrNotFound
	}
	if err != nil {
		return "", 0, err
	}
	if cost > 0 {
		if balance < cost {
			return "", balance, ErrInsufficientCredits
		}
		balance -= cost
		if _, err := tx.Exec(ctx, `UPDATE users SET credits = $2, updated_at = now() WHERE id = $1`, userID, balance); err != nil {
			return "", 0, err
		}
		if _, err := tx.Exec(ctx, `
			INSERT INTO transactions (user_id, kind, amount, balance_after, note)
			VALUES ($1, 'consume', $2, $3, $4)
		`, userID, -cost, balance, "生图消耗 "+kind); err != nil {
			return "", 0, err
		}
	}
	err = tx.QueryRow(ctx, `
		INSERT INTO generations (user_id, kind, model, n, cost, status, params)
		VALUES ($1, $2, $3, $4, $5, 'running', $6)
		RETURNING id
	`, userID, kind, model, n, cost, paramsJSON).Scan(&genID)
	if err != nil {
		return "", 0, err
	}
	if err := tx.Commit(ctx); err != nil {
		return "", 0, err
	}
	return genID, balance, nil
}

// CompleteGeneration marks a generation as succeeded.
func (s *Store) CompleteGeneration(ctx context.Context, generationID string) error {
	_, err := s.Pool.Exec(ctx, `UPDATE generations SET status = 'ok' WHERE id = $1`, generationID)
	return err
}

// FailGeneration records a failure without touching the balance (used when
// nothing was charged, e.g. admin exemption).
func (s *Store) FailGeneration(ctx context.Context, generationID string, upstreamStatus int, errMsg string) error {
	_, err := s.Pool.Exec(ctx, `
		UPDATE generations SET status = 'failed_refunded', upstream_status = $2, error = $3 WHERE id = $1
	`, generationID, upstreamStatus, errMsg)
	return err
}

// RefundGeneration rolls back a failed charge: restores the credits, writes a
// `refund` transaction and marks the generation failed_refunded. Idempotent
// per generation row (status guard).
func (s *Store) RefundGeneration(ctx context.Context, generationID string, upstreamStatus int, errMsg string) error {
	tx, err := s.Pool.Begin(ctx)
	if err != nil {
		return err
	}
	defer tx.Rollback(ctx)

	var userID string
	var cost int64
	var status string
	err = tx.QueryRow(ctx, `SELECT user_id, cost, status FROM generations WHERE id = $1 FOR UPDATE`, generationID).
		Scan(&userID, &cost, &status)
	if errors.Is(err, pgx.ErrNoRows) {
		return ErrNotFound
	}
	if err != nil {
		return err
	}
	if status != "running" {
		return nil // already settled
	}
	if cost > 0 {
		var balance int64
		if err := tx.QueryRow(ctx, `UPDATE users SET credits = credits + $2, updated_at = now() WHERE id = $1 RETURNING credits`, userID, cost).Scan(&balance); err != nil {
			return err
		}
		if _, err := tx.Exec(ctx, `
			INSERT INTO transactions (user_id, kind, amount, balance_after, generation_id, note)
			VALUES ($1, 'refund', $2, $3, $4, $5)
		`, userID, cost, balance, generationID, "生成失败退还"); err != nil {
			return err
		}
	}
	if _, err := tx.Exec(ctx, `
		UPDATE generations SET status = 'failed_refunded', upstream_status = $2, error = $3 WHERE id = $1
	`, generationID, upstreamStatus, errMsg); err != nil {
		return err
	}
	return tx.Commit(ctx)
}

// AdjustCredits applies a signed admin adjustment (refuses to overdraw below
// zero) and records the ledger entry.
func (s *Store) AdjustCredits(ctx context.Context, userID string, amount int64, note string) (*User, error) {
	tx, err := s.Pool.Begin(ctx)
	if err != nil {
		return nil, err
	}
	defer tx.Rollback(ctx)

	var balance int64
	err = tx.QueryRow(ctx, `SELECT credits FROM users WHERE id = $1 FOR UPDATE`, userID).Scan(&balance)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	if balance+amount < 0 {
		return nil, ErrInsufficientCredits
	}
	var u User
	err = tx.QueryRow(ctx, `
		UPDATE users SET credits = credits + $2, updated_at = now() WHERE id = $1
		RETURNING `+userColumns, userID, amount,
	).Scan(&u.ID, &u.Username, &u.Email, &u.PasswordHash, &u.Role, &u.Status,
		&u.Credits, &u.CreatedAt, &u.UpdatedAt, &u.LastLoginAt)
	if err != nil {
		return nil, err
	}
	if _, err := tx.Exec(ctx, `
		INSERT INTO transactions (user_id, kind, amount, balance_after, note)
		VALUES ($1, 'admin_adjust', $2, $3, $4)
	`, userID, amount, u.Credits, note); err != nil {
		return nil, err
	}
	if err := tx.Commit(ctx); err != nil {
		return nil, err
	}
	return &u, nil
}

// Transaction is one ledger entry.
type Transaction struct {
	ID           string    `json:"id"`
	Kind         string    `json:"kind"`
	Amount       int64     `json:"amount"`
	BalanceAfter int64     `json:"balanceAfter"`
	Note         *string   `json:"note,omitempty"`
	CreatedAt    time.Time `json:"createdAt"`
}

// ListTransactions returns the newest ledger entries for a user.
func (s *Store) ListTransactions(ctx context.Context, userID string, limit int) ([]Transaction, error) {
	if limit <= 0 || limit > 200 {
		limit = 50
	}
	rows, err := s.Pool.Query(ctx, `
		SELECT id, kind, amount, balance_after, note, created_at
		FROM transactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2
	`, userID, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	out := []Transaction{}
	for rows.Next() {
		var t Transaction
		if err := rows.Scan(&t.ID, &t.Kind, &t.Amount, &t.BalanceAfter, &t.Note, &t.CreatedAt); err != nil {
			return nil, err
		}
		out = append(out, t)
	}
	return out, rows.Err()
}

// GenerationRecord is one generation row (audit / usage view).
type GenerationRecord struct {
	ID             string    `json:"id"`
	UserID         string    `json:"userId"`
	Username       string    `json:"username"`
	Kind           string    `json:"kind"`
	Model          string    `json:"model"`
	N              int       `json:"n"`
	Cost           int64     `json:"cost"`
	Status         string    `json:"status"`
	UpstreamStatus *int      `json:"upstreamStatus,omitempty"`
	Error          *string   `json:"error,omitempty"`
	CreatedAt      time.Time `json:"createdAt"`
}

// ListGenerations returns recent generations; userID "" means all users
// (admin view).
func (s *Store) ListGenerations(ctx context.Context, userID string, limit int) ([]GenerationRecord, error) {
	if limit <= 0 || limit > 200 {
		limit = 50
	}
	base := `
		SELECT g.id, g.user_id, u.username, g.kind, g.model, g.n, g.cost, g.status, g.upstream_status, g.error, g.created_at
		FROM generations g JOIN users u ON u.id = g.user_id `
	args := []any{}
	if userID != "" {
		base += `WHERE g.user_id = $1 `
		args = append(args, userID)
	}
	base += `ORDER BY g.created_at DESC LIMIT $` + strconv.Itoa(len(args)+1)
	args = append(args, limit)
	rows, err := s.Pool.Query(ctx, base, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	out := []GenerationRecord{}
	for rows.Next() {
		var g GenerationRecord
		if err := rows.Scan(&g.ID, &g.UserID, &g.Username, &g.Kind, &g.Model, &g.N, &g.Cost,
			&g.Status, &g.UpstreamStatus, &g.Error, &g.CreatedAt); err != nil {
			return nil, err
		}
		out = append(out, g)
	}
	return out, rows.Err()
}
