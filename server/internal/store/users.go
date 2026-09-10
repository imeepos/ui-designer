package store

import (
	"context"
	"errors"
	"strings"
	"time"

	"github.com/jackc/pgx/v5"
)

// User is the persisted account row. PasswordHash never leaves this package.
type User struct {
	ID           string
	Username     string
	Email        *string
	PasswordHash string
	Role         string // user | admin
	Status       string // active | disabled
	Credits      int64
	CreatedAt    time.Time
	UpdatedAt    time.Time
	LastLoginAt  *time.Time
}

// IsAdmin reports whether the account carries the admin role.
func (u *User) IsAdmin() bool { return u.Role == "admin" }

// IsActive reports whether the account may authenticate and generate.
func (u *User) IsActive() bool { return u.Status == "active" }

const userColumns = `id, username, email, password_hash, role, status, credits, created_at, updated_at, last_login_at`

func scanUser(row pgx.Row) (*User, error) {
	var u User
	err := row.Scan(&u.ID, &u.Username, &u.Email, &u.PasswordHash, &u.Role, &u.Status,
		&u.Credits, &u.CreatedAt, &u.UpdatedAt, &u.LastLoginAt)
	if errors.Is(err, pgx.ErrNoRows) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	return &u, nil
}

// CreateUser inserts an account (password already hashed) and, when bonus > 0,
// records the signup-bonus transaction in the same transaction.
func (s *Store) CreateUser(ctx context.Context, username string, email *string, passwordHash string, role string, bonus int64) (*User, error) {
	tx, err := s.Pool.Begin(ctx)
	if err != nil {
		return nil, err
	}
	defer tx.Rollback(ctx)

	var u User
	err = tx.QueryRow(ctx, `
		INSERT INTO users (username, email, password_hash, role, credits)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING `+userColumns,
		username, email, passwordHash, role, bonus,
	).Scan(&u.ID, &u.Username, &u.Email, &u.PasswordHash, &u.Role, &u.Status,
		&u.Credits, &u.CreatedAt, &u.UpdatedAt, &u.LastLoginAt)
	if err != nil {
		if strings.Contains(err.Error(), "users_username_key") {
			return nil, ErrUsernameTaken
		}
		if strings.Contains(err.Error(), "users_email_key") {
			return nil, ErrEmailTaken
		}
		return nil, err
	}
	if bonus > 0 {
		if _, err := tx.Exec(ctx, `
			INSERT INTO transactions (user_id, kind, amount, balance_after, note)
			VALUES ($1, 'signup_bonus', $2, $3, $4)
		`, u.ID, bonus, bonus, "注册赠送"); err != nil {
			return nil, err
		}
	}
	if err := tx.Commit(ctx); err != nil {
		return nil, err
	}
	return &u, nil
}

// GetUserByID fetches one account.
func (s *Store) GetUserByID(ctx context.Context, id string) (*User, error) {
	return scanUser(s.Pool.QueryRow(ctx, `SELECT `+userColumns+` FROM users WHERE id = $1`, id))
}

// GetUserByUsername fetches one account by username (login path).
func (s *Store) GetUserByUsername(ctx context.Context, username string) (*User, error) {
	return scanUser(s.Pool.QueryRow(ctx, `SELECT `+userColumns+` FROM users WHERE username = $1`, username))
}

// TouchLastLogin records a successful login.
func (s *Store) TouchLastLogin(ctx context.Context, id string) error {
	_, err := s.Pool.Exec(ctx, `UPDATE users SET last_login_at = now() WHERE id = $1`, id)
	return err
}

// UpdatePassword replaces the stored hash.
func (s *Store) UpdatePassword(ctx context.Context, id, passwordHash string) error {
	_, err := s.Pool.Exec(ctx, `UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1`, id, passwordHash)
	return err
}

// UpdateProfile lets an admin flip status and role (nil = unchanged).
func (s *Store) UpdateProfile(ctx context.Context, id string, status, role *string) (*User, error) {
	row := s.Pool.QueryRow(ctx, `
		UPDATE users SET
			status = COALESCE($2, status),
			role   = COALESCE($3, role),
			updated_at = now()
		WHERE id = $1
		RETURNING `+userColumns, id, status, role)
	return scanUser(row)
}

// UserList is one page of accounts plus the total for pagination.
type UserList struct {
	Total int64
	Users []User
}

// ListUsers returns accounts ordered by creation, newest first, optionally
// filtered by a username/email substring.
func (s *Store) ListUsers(ctx context.Context, query string, limit, offset int) (*UserList, error) {
	if limit <= 0 || limit > 200 {
		limit = 50
	}
	if offset < 0 {
		offset = 0
	}
	pattern := "%" + strings.ToLower(strings.TrimSpace(query)) + "%"
	list := &UserList{Users: []User{}}
	err := s.Pool.QueryRow(ctx,
		`SELECT count(*) FROM users WHERE lower(username) LIKE $1 OR lower(coalesce(email,'')) LIKE $1`,
		pattern).Scan(&list.Total)
	if err != nil {
		return nil, err
	}
	rows, err := s.Pool.Query(ctx, `
		SELECT `+userColumns+` FROM users
		WHERE lower(username) LIKE $1 OR lower(coalesce(email,'')) LIKE $1
		ORDER BY created_at DESC
		LIMIT $2 OFFSET $3`, pattern, limit, offset)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	for rows.Next() {
		var u User
		if err := rows.Scan(&u.ID, &u.Username, &u.Email, &u.PasswordHash, &u.Role, &u.Status,
			&u.Credits, &u.CreatedAt, &u.UpdatedAt, &u.LastLoginAt); err != nil {
			return nil, err
		}
		list.Users = append(list.Users, u)
	}
	return list, rows.Err()
}

// Stats aggregates the admin overview numbers.
type Stats struct {
	Users              int64 `json:"users"`
	ActiveUsers        int64 `json:"activeUsers"`
	Generations        int64 `json:"generations"`
	ImagesGenerated    int64 `json:"imagesGenerated"`
	ImagesFailed       int64 `json:"imagesFailed"`
	CreditsConsumed    int64 `json:"creditsConsumed"`
	CreditsGranted     int64 `json:"creditsGranted"`
	CreditsOutstanding int64 `json:"creditsOutstanding"`
}

// LoadStats computes the dashboard counters.
func (s *Store) LoadStats(ctx context.Context) (*Stats, error) {
	st := &Stats{}
	err := s.Pool.QueryRow(ctx, `
		SELECT
			(SELECT count(*) FROM users),
			(SELECT count(*) FROM users WHERE status = 'active'),
			(SELECT count(*) FROM generations),
			(SELECT coalesce(sum(n),0) FROM generations WHERE status = 'ok'),
			(SELECT count(*) FROM generations WHERE status <> 'ok'),
			(SELECT coalesce(sum(-amount),0) FROM transactions WHERE kind = 'consume'),
			(SELECT coalesce(sum(amount),0) FROM transactions WHERE kind IN ('signup_bonus','admin_adjust','refund')),
			(SELECT coalesce(sum(credits),0) FROM users)
	`).Scan(&st.Users, &st.ActiveUsers, &st.Generations, &st.ImagesGenerated,
		&st.ImagesFailed, &st.CreditsConsumed, &st.CreditsGranted, &st.CreditsOutstanding)
	if err != nil {
		return nil, err
	}
	return st, nil
}

// Sentinel user errors mapped to HTTP codes by the api package.
var (
	ErrUsernameTaken = errors.New("username already taken")
	ErrEmailTaken    = errors.New("email already taken")
)
