// Package api wires the HTTP surface of rudder-server: auth, user billing,
// image proxying and the admin API, all under /api/v1.
package api

import (
	"context"
	"encoding/json"
	"log"
	"net"
	"net/http"
	"strings"
	"sync"
	"time"

	"rudder-server/internal/auth"
	"rudder-server/internal/store"
	"rudder-server/internal/upstream"
)

// Server carries the wired dependencies.
type Server struct {
	Store  *store.Store
	Secret string
	// UpstreamTimeout bounds ONE upstream attempt. Retry budget keeps the
	// worst case under nginx's proxy_read_timeout (330s).
	UpstreamTimeout time.Duration
	UpstreamRetries int

	limiter *rateLimiter
}

// New builds the API server.
func New(st *store.Store, secret string, upstreamTimeout time.Duration, upstreamRetries int) *Server {
	return &Server{
		Store:           st,
		Secret:          secret,
		UpstreamTimeout: upstreamTimeout,
		UpstreamRetries: upstreamRetries,
		limiter:         newRateLimiter(60, time.Minute), // per-IP, per-minute
	}
}

// Router assembles all routes. Go 1.22 pattern routing: METHOD /path/{id}.
func (s *Server) Router() http.Handler {
	mux := http.NewServeMux()

	// --- public ---
	mux.HandleFunc("GET /api/v1/health", s.handleHealth)
	mux.HandleFunc("POST /api/v1/auth/register", s.limit(s.handleRegister))
	mux.HandleFunc("POST /api/v1/auth/login", s.limit(s.handleLogin))

	// --- authenticated user ---
	mux.Handle("GET /api/v1/auth/me", s.auth(false, http.HandlerFunc(s.handleMe)))
	mux.Handle("POST /api/v1/auth/change-password", s.auth(false, http.HandlerFunc(s.handleChangePassword)))
	mux.Handle("GET /api/v1/models", s.auth(false, http.HandlerFunc(s.handleModels)))
	mux.Handle("POST /api/v1/images/generations", s.auth(false, http.HandlerFunc(s.handleGenerations)))
	mux.Handle("POST /api/v1/images/edits", s.auth(false, http.HandlerFunc(s.handleEdits)))
	mux.Handle("GET /api/v1/usage", s.auth(false, http.HandlerFunc(s.handleUsage)))

	// --- admin ---
	mux.Handle("GET /api/v1/admin/stats", s.auth(true, http.HandlerFunc(s.handleAdminStats)))
	mux.Handle("GET /api/v1/admin/users", s.auth(true, http.HandlerFunc(s.handleAdminUsers)))
	mux.Handle("PATCH /api/v1/admin/users/{id}", s.auth(true, http.HandlerFunc(s.handleAdminUserPatch)))
	mux.Handle("POST /api/v1/admin/users/{id}/credits", s.auth(true, http.HandlerFunc(s.handleAdminCredits)))
	mux.Handle("POST /api/v1/admin/users/{id}/reset-password", s.auth(true, http.HandlerFunc(s.handleAdminResetPassword)))
	mux.Handle("GET /api/v1/admin/settings", s.auth(true, http.HandlerFunc(s.handleAdminGetSettings)))
	mux.Handle("PUT /api/v1/admin/settings", s.auth(true, http.HandlerFunc(s.handleAdminPutSettings)))
	mux.Handle("GET /api/v1/admin/generations", s.auth(true, http.HandlerFunc(s.handleAdminGenerations)))
	mux.HandleFunc("GET /api/v1/admin/console", s.handleAdminConsole)

	// fallthrough: unknown route
	return s.withMiddleware(mux)
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

type statusRecorder struct {
	http.ResponseWriter
	status int
}

func (r *statusRecorder) WriteHeader(code int) {
	r.status = code
	r.ResponseWriter.WriteHeader(code)
}

// withMiddleware applies panic recovery, CORS and one-line request logging.
// Bodies and Authorization headers are never logged.
func (s *Server) withMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		rec := &statusRecorder{ResponseWriter: w, status: 200}

		defer func() {
			if p := recover(); p != nil {
				log.Printf("panic %s %s: %v", r.Method, r.URL.Path, p)
				writeError(rec, http.StatusInternalServerError, CodeInternal, "internal error", "")
			}
			log.Printf("%s %s -> %d (%dms)", r.Method, r.URL.Path, rec.status, time.Since(start).Milliseconds())
		}()

		if r.Method == http.MethodOptions {
			setCORS(rec.Header())
			rec.WriteHeader(http.StatusNoContent)
			return
		}
		setCORS(rec.Header())
		next.ServeHTTP(rec, r)
	})
}

func setCORS(h http.Header) {
	h.Set("Access-Control-Allow-Origin", "*")
	h.Set("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")
	h.Set("Access-Control-Allow-Headers", "Authorization, Content-Type")
	h.Set("Access-Control-Max-Age", "86400")
}

// ctxUser is the authenticated account attached to a request.
type ctxUser struct{}

func userFrom(ctx context.Context) *store.User {
	u, _ := ctx.Value(ctxUser{}).(*store.User)
	return u
}

// auth wraps a handler requiring a Bearer session token; admin also demands
// the admin role.
func (s *Server) auth(admin bool, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		header := r.Header.Get("Authorization")
		token, ok := strings.CutPrefix(header, "Bearer ")
		if !ok || strings.TrimSpace(token) == "" {
			writeError(w, http.StatusUnauthorized, CodeUnauthorized, "缺少登录凭证", "请先登录获取 token")
			return
		}
		userID, _, err := auth.VerifyToken(s.Secret, strings.TrimSpace(token))
		if err != nil {
			writeError(w, http.StatusUnauthorized, CodeUnauthorized, "登录已过期或无效", "请重新登录")
			return
		}
		user, err := s.Store.GetUserByID(r.Context(), userID)
		if err == store.ErrNotFound {
			writeError(w, http.StatusUnauthorized, CodeUnauthorized, "账号不存在", "")
			return
		}
		if err != nil {
			writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
			return
		}
		if !user.IsActive() {
			writeError(w, http.StatusForbidden, CodeUserDisabled, "账号已被禁用", "请联系管理员")
			return
		}
		if admin && !user.IsAdmin() {
			writeError(w, http.StatusForbidden, CodeForbidden, "需要管理员权限", "")
			return
		}
		next.ServeHTTP(w, r.WithContext(context.WithValue(r.Context(), ctxUser{}, user)))
	})
}

// limit applies the per-IP limiter to sensitive endpoints (auth writes).
func (s *Server) limit(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ip, _, err := net.SplitHostPort(r.RemoteAddr)
		if err != nil {
			ip = r.RemoteAddr
		}
		if fwd := r.Header.Get("X-Real-IP"); fwd != "" {
			ip = fwd
		}
		if !s.limiter.allow(ip) {
			writeError(w, http.StatusTooManyRequests, CodeRateLimited, "请求过于频繁", "请稍后再试")
			return
		}
		next(w, r)
	}
}

// rateLimiter is a tiny fixed-window per-key counter (no external deps).
type rateLimiter struct {
	mu      sync.Mutex
	counts  map[string]int
	window  time.Time
	limit   int
	perTime time.Duration
}

func newRateLimiter(limit int, per time.Duration) *rateLimiter {
	return &rateLimiter{counts: map[string]int{}, limit: limit, perTime: per, window: time.Now()}
}

func (l *rateLimiter) allow(key string) bool {
	l.mu.Lock()
	defer l.mu.Unlock()
	now := time.Now()
	if now.Sub(l.window) >= l.perTime {
		l.window = now
		l.counts = map[string]int{}
	}
	l.counts[key]++
	return l.counts[key] <= l.limit
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

func decodeJSON(w http.ResponseWriter, r *http.Request, into any, maxBytes int64) bool {
	r.Body = http.MaxBytesReader(w, r.Body, maxBytes)
	dec := json.NewDecoder(r.Body)
	if err := dec.Decode(into); err != nil {
		writeError(w, http.StatusBadRequest, CodeValidation, "请求体不是合法 JSON: "+err.Error(), "")
		return false
	}
	return true
}

func clientIP(r *http.Request) string {
	if fwd := r.Header.Get("X-Real-IP"); fwd != "" {
		return fwd
	}
	ip, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return r.RemoteAddr
	}
	return ip
}

// newUpstream builds an upstream client from a settings snapshot.
func (s *Server) newUpstream(settings store.Settings) (*upstream.Client, error) {
	if settings.UpstreamBaseURL == "" || settings.UpstreamAPIKey == "" {
		return nil, errUpstreamMisconfigured
	}
	return upstream.NewWithRetry(settings.UpstreamBaseURL, settings.UpstreamAPIKey, s.UpstreamTimeout, s.UpstreamRetries), nil
}

var errUpstreamMisconfigured = &upstreamMisconfigured{}

type upstreamMisconfigured struct{}

func (*upstreamMisconfigured) Error() string { return "upstream base url or api key not configured" }
