// Package config parses environment configuration for rudder-server.
//
// Secrets (upstream key, admin password) are only ever read from the
// environment or the settings table — never logged.
package config

import (
	"os"
	"strconv"
)

// Config is the process-level configuration resolved from the environment.
type Config struct {
	// ListenAddr is the HTTP listen address. Bind loopback only; nginx
	// terminates TLS and proxies /api/v1/.
	ListenAddr string
	// DatabaseURL is the Postgres connection string (required).
	DatabaseURL string

	// AdminUsername / AdminPassword seed the built-in admin account on first
	// boot (when the users table is empty). If AdminPassword is empty a
	// random password is generated and written to stdout once.
	AdminUsername string
	AdminPassword string

	// Upstream seed values: applied to the settings table on first boot only,
	// so the admin console stays the source of truth afterwards.
	UpstreamBaseURL string
	UpstreamAPIKey  string
	UpstreamModel   string

	// GenTimeoutSec bounds a single upstream generation attempt.
	GenTimeoutSec int
}

func env(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// Load reads configuration from the environment with sane defaults.
func Load() Config {
	return Config{
		ListenAddr:      env("LISTEN_ADDR", "127.0.0.1:8799"),
		DatabaseURL:     env("DATABASE_URL", ""),
		AdminUsername:   env("ADMIN_USERNAME", "admin"),
		AdminPassword:   env("ADMIN_PASSWORD", ""),
		UpstreamBaseURL: env("UPSTREAM_BASE_URL", ""),
		UpstreamAPIKey:  env("UPSTREAM_API_KEY", ""),
		UpstreamModel:   env("UPSTREAM_MODEL", "gpt-image-2"),
		GenTimeoutSec:   envInt("GEN_TIMEOUT_SEC", 300),
	}
}

func envInt(key string, fallback int) int {
	v := os.Getenv(key)
	if v == "" {
		return fallback
	}
	n, err := strconv.Atoi(v)
	if err != nil || n <= 0 {
		return fallback
	}
	return n
}
