// Package store owns the Postgres connection, schema migrations and all
// persistence. API handlers never write SQL directly.
package store

import (
	"context"
	"crypto/rand"
	"embed"
	"encoding/hex"
	"errors"
	"fmt"
	"sort"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

//go:embed migrations/*.sql
var migrationsFS embed.FS

// Store wraps the connection pool.
type Store struct {
	Pool *pgxpool.Pool
}

// Open connects and applies pending migrations.
func Open(ctx context.Context, databaseURL string) (*Store, error) {
	pool, err := pgxpool.New(ctx, databaseURL)
	if err != nil {
		return nil, fmt.Errorf("connect database: %w", err)
	}
	if err := pool.Ping(ctx); err != nil {
		pool.Close()
		return nil, fmt.Errorf("ping database: %w", err)
	}
	s := &Store{Pool: pool}
	if err := s.migrate(ctx); err != nil {
		pool.Close()
		return nil, err
	}
	return s, nil
}

// migrate applies embedded *.sql files in filename order. Files are
// idempotent (IF NOT EXISTS), so re-running is always safe.
func (s *Store) migrate(ctx context.Context) error {
	entries, err := migrationsFS.ReadDir("migrations")
	if err != nil {
		return fmt.Errorf("read embedded migrations: %w", err)
	}
	names := make([]string, 0, len(entries))
	for _, e := range entries {
		names = append(names, e.Name())
	}
	sort.Strings(names)
	for _, name := range names {
		sqlBytes, err := migrationsFS.ReadFile("migrations/" + name)
		if err != nil {
			return fmt.Errorf("read migration %s: %w", name, err)
		}
		if _, err := s.Pool.Exec(ctx, string(sqlBytes)); err != nil {
			return fmt.Errorf("apply migration %s: %w", name, err)
		}
	}
	return nil
}

// Close releases the pool.
func (s *Store) Close() { s.Pool.Close() }

// ---------------------------------------------------------------------------
// Settings (key/value; drives price + upstream address + JWT secret)
// ---------------------------------------------------------------------------

// Setting keys.
const (
	KeyCreditsPerImage     = "credits_per_image"
	KeySignupBonusCredits  = "signup_bonus_credits"
	KeyRegistrationOpen    = "registration_open"
	KeyUpstreamBaseURL     = "upstream_base_url"
	KeyUpstreamAPIKey      = "upstream_api_key"
	KeyUpstreamModel       = "upstream_model"
	KeyJWTSecret           = "jwt_secret"
	DefaultCreditsPerImage = "10"
	DefaultModel           = "gpt-image-2"
)

// ErrNotFound is returned when a row does not exist.
var ErrNotFound = errors.New("not found")

// GetSetting returns one setting value or ErrNotFound.
func (s *Store) GetSetting(ctx context.Context, key string) (string, error) {
	var value string
	err := s.Pool.QueryRow(ctx, `SELECT value FROM settings WHERE key = $1`, key).Scan(&value)
	if errors.Is(err, pgx.ErrNoRows) {
		return "", ErrNotFound
	}
	return value, err
}

// SetSetting upserts one setting.
func (s *Store) SetSetting(ctx context.Context, key, value string) error {
	_, err := s.Pool.Exec(ctx, `
		INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, now())
		ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = now()
	`, key, value)
	return err
}

// Settings is the cached view the handlers use. The upstream key is never
// exposed through JSON — only its tail.
type Settings struct {
	CreditsPerImage    int64
	SignupBonusCredits int64
	RegistrationOpen   bool
	UpstreamBaseURL    string
	UpstreamAPIKey     string
	UpstreamModel      string
}

// defaults applied when a key is missing (fresh database).
func (s *Store) seedSettings(ctx context.Context, seed Settings) error {
	pairs := map[string]string{
		KeyCreditsPerImage:    fmt.Sprintf("%d", seed.CreditsPerImage),
		KeySignupBonusCredits: fmt.Sprintf("%d", seed.SignupBonusCredits),
		KeyRegistrationOpen:   "true",
		KeyUpstreamModel:      seed.UpstreamModel,
	}
	if seed.UpstreamBaseURL != "" {
		pairs[KeyUpstreamBaseURL] = seed.UpstreamBaseURL
	}
	if seed.UpstreamAPIKey != "" {
		pairs[KeyUpstreamAPIKey] = seed.UpstreamAPIKey
	}
	for k, v := range pairs {
		exists, err := s.HasSetting(ctx, k)
		if err != nil {
			return err
		}
		if !exists {
			if err := s.SetSetting(ctx, k, v); err != nil {
				return err
			}
		}
	}
	return nil
}

// HasSetting reports whether a key exists.
func (s *Store) HasSetting(ctx context.Context, key string) (bool, error) {
	var one bool
	err := s.Pool.QueryRow(ctx, `SELECT true FROM settings WHERE key = $1`, key).Scan(&one)
	if errors.Is(err, pgx.ErrNoRows) {
		return false, nil
	}
	return one, err
}

// LoadSettings reads every known key with fallbacks. Callers should treat the
// result as a snapshot (valid for one request).
func (s *Store) LoadSettings(ctx context.Context) (Settings, error) {
	out := Settings{
		CreditsPerImage: 10,
		UpstreamModel:   DefaultModel,
		RegistrationOpen: true,
	}
	rows, err := s.Pool.Query(ctx, `SELECT key, value FROM settings`)
	if err != nil {
		return out, err
	}
	defer rows.Close()
	values := map[string]string{}
	for rows.Next() {
		var k, v string
		if err := rows.Scan(&k, &v); err != nil {
			return out, err
		}
		values[k] = v
	}
	if err := rows.Err(); err != nil {
		return out, err
	}
	if v, ok := values[KeyCreditsPerImage]; ok {
		if n, err := parseI64(v); err == nil && n >= 0 {
			out.CreditsPerImage = n
		}
	}
	if v, ok := values[KeySignupBonusCredits]; ok {
		if n, err := parseI64(v); err == nil && n >= 0 {
			out.SignupBonusCredits = n
		}
	}
	if v, ok := values[KeyRegistrationOpen]; ok {
		out.RegistrationOpen = v == "true" || v == "1"
	}
	if v, ok := values[KeyUpstreamBaseURL]; ok {
		out.UpstreamBaseURL = v
	}
	if v, ok := values[KeyUpstreamAPIKey]; ok {
		out.UpstreamAPIKey = v
	}
	if v, ok := values[KeyUpstreamModel]; ok && v != "" {
		out.UpstreamModel = v
	}
	return out, nil
}

// EnsureJWTSecret returns the persisted JWT signing secret, creating a random
// one on first use so tokens survive restarts.
func (s *Store) EnsureJWTSecret(ctx context.Context) (string, error) {
	secret, err := s.GetSetting(ctx, KeyJWTSecret)
	if err == nil && len(secret) >= 32 {
		return secret, nil
	}
	if err != nil && !errors.Is(err, ErrNotFound) {
		return "", err
	}
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		return "", err
	}
	secret = hex.EncodeToString(buf)
	if err := s.SetSetting(ctx, KeyJWTSecret, secret); err != nil {
		return "", err
	}
	return secret, nil
}

// SeedDefaults applies first-boot defaults + env seeds. Existing keys win.
func (s *Store) SeedDefaults(ctx context.Context, seed Settings) error {
	return s.seedSettings(ctx, seed)
}

func parseI64(v string) (int64, error) {
	var n int64
	_, err := fmt.Sscanf(v, "%d", &n)
	return n, err
}

// Tail4 returns the display-safe last 4 characters of a secret.
func Tail4(s string) string {
	if s == "" {
		return ""
	}
	r := []rune(s)
	if len(r) <= 4 {
		return string(r)
	}
	return string(r[len(r)-4:])
}
