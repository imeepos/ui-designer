// rudder-server: 舵 Rudder 的 Go 后端。
//
// 职责：用户系统（注册/登录）、按次计费（积分）、gpt-image-2 上游代理
// （客户端 0 配置，上游地址与密钥由管理员配置）、管理 API + Web 控制台。
// 存储为 Postgres；所有路由挂在 /api/v1 下，由 nginx 反代 + TLS。
package main

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"rudder-server/internal/api"
	"rudder-server/internal/auth"
	"rudder-server/internal/config"
	"rudder-server/internal/store"
)

func main() {
	cfg := config.Load()
	if cfg.DatabaseURL == "" {
		log.Fatal("DATABASE_URL is required (e.g. postgres://user:pass@127.0.0.1:5432/rudder?sslmode=disable)")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	st, err := store.Open(ctx, cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("open store: %v", err)
	}
	defer st.Close()

	// First-boot defaults + env seeds. Existing settings always win so the
	// admin console stays the source of truth.
	if err := st.SeedDefaults(ctx, store.Settings{
		CreditsPerImage: 10,
		UpstreamModel:   cfg.UpstreamModel,
		UpstreamBaseURL: cfg.UpstreamBaseURL,
		UpstreamAPIKey:  cfg.UpstreamAPIKey,
	}); err != nil {
		log.Fatalf("seed settings: %v", err)
	}

	if err := ensureAdmin(ctx, st, cfg); err != nil {
		log.Fatalf("ensure admin: %v", err)
	}

	secret, err := st.EnsureJWTSecret(ctx)
	if err != nil {
		log.Fatalf("ensure jwt secret: %v", err)
	}

	upstreamTimeout := time.Duration(cfg.GenTimeoutSec) * time.Second
	srv := &http.Server{
		// 对外经 nginx 代理；仅监听环回地址，数据库/上游错误不暴露。
		Addr:              cfg.ListenAddr,
		Handler:           api.New(st, secret, upstreamTimeout, 1).Router(),
		ReadHeaderTimeout: 15 * time.Second,
		IdleTimeout:       120 * time.Second,
	}

	go func() {
		log.Printf("rudder-server listening on %s (api base /api/v1, upstream timeout %s)", cfg.ListenAddr, upstreamTimeout)
		if err := srv.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			log.Fatalf("listen: %v", err)
		}
	}()

	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)
	<-stop
	log.Printf("shutting down…")
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer shutdownCancel()
	_ = srv.Shutdown(shutdownCtx)
}

// ensureAdmin guarantees the built-in admin account exists. Password source:
// ADMIN_PASSWORD env; when unset (first boot of a keyless deployment) a
// random password is generated and printed exactly once.
func ensureAdmin(ctx context.Context, st *store.Store, cfg config.Config) error {
	if _, err := st.GetUserByUsername(ctx, cfg.AdminUsername); err == nil {
		return nil // already provisioned
	}
	var password string
	if cfg.AdminPassword != "" {
		password = cfg.AdminPassword
	} else {
		buf := make([]byte, 12)
		if _, err := rand.Read(buf); err != nil {
			return err
		}
		password = hex.EncodeToString(buf)
		log.Printf("ADMIN_PASSWORD 未设置：已为内置管理员 %q 生成随机密码（仅此一次打印）：%s", cfg.AdminUsername, password)
	}
	hash, err := auth.HashPassword(password)
	if err != nil {
		return err
	}
	// 内置管理员自带 0 积分；管理员生图免计费。
	if _, err := st.CreateUser(ctx, cfg.AdminUsername, nil, hash, "admin", 0); err != nil {
		// 并发启动等场景下已存在即视为成功。
		if _, getErr := st.GetUserByUsername(ctx, cfg.AdminUsername); getErr == nil {
			return nil
		}
		return err
	}
	log.Printf("built-in admin %q provisioned", cfg.AdminUsername)
	return nil
}
