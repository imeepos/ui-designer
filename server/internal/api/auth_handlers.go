package api

import (
	"errors"
	"net/http"
	"regexp"
	"strings"
	"time"

	"rudder-server/internal/auth"
	"rudder-server/internal/store"
)

// handleHealth is the liveness probe (no auth).
func (s *Server) handleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{"ok": true, "service": "rudder-server", "time": time.Now().Format(time.RFC3339)})
}

// ---------------------------------------------------------------------------
// DTOs — password hashes never cross this boundary.
// ---------------------------------------------------------------------------

type userDTO struct {
	ID          string     `json:"id"`
	Username    string     `json:"username"`
	Email       *string    `json:"email"`
	Role        string     `json:"role"`
	Status      string     `json:"status"`
	Credits     int64      `json:"credits"`
	CreatedAt   time.Time  `json:"createdAt"`
	LastLoginAt *time.Time `json:"lastLoginAt,omitempty"`
}

func toUserDTO(u *store.User) userDTO {
	return userDTO{
		ID: u.ID, Username: u.Username, Email: u.Email, Role: u.Role, Status: u.Status,
		Credits: u.Credits, CreatedAt: u.CreatedAt, LastLoginAt: u.LastLoginAt,
	}
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/register
// ---------------------------------------------------------------------------

var (
	usernameRe = regexp.MustCompile(`^[a-zA-Z0-9_-]{3,32}$`)
	emailRe    = regexp.MustCompile(`^[^@\s]+@[^@\s]+\.[^@\s]+$`)
)

type registerInput struct {
	Username string  `json:"username"`
	Password string  `json:"password"`
	Email    *string `json:"email"`
}

func (s *Server) handleRegister(w http.ResponseWriter, r *http.Request) {
	var in registerInput
	if !decodeJSON(w, r, &in, 8<<10) {
		return
	}
	in.Username = strings.TrimSpace(in.Username)
	if !usernameRe.MatchString(in.Username) {
		writeError(w, http.StatusBadRequest, CodeValidation, "用户名需为 3-32 位字母、数字、下划线或中划线", "")
		return
	}
	if len(in.Password) < 6 || len(in.Password) > 128 {
		writeError(w, http.StatusBadRequest, CodeValidation, "密码长度需为 6-128 位", "")
		return
	}
	if in.Email != nil {
		email := strings.TrimSpace(strings.ToLower(*in.Email))
		if email == "" {
			in.Email = nil
		} else if !emailRe.MatchString(email) || len(email) > 254 {
			writeError(w, http.StatusBadRequest, CodeValidation, "邮箱格式不正确", "")
			return
		} else {
			in.Email = &email
		}
	}

	settings, err := s.Store.LoadSettings(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	if !settings.RegistrationOpen {
		writeError(w, http.StatusForbidden, CodeRegistrationClosed, "注册已关闭", "请联系管理员开通账号")
		return
	}

	hash, err := auth.HashPassword(in.Password)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	user, err := s.Store.CreateUser(r.Context(), in.Username, in.Email, hash, "user", settings.SignupBonusCredits)
	switch {
	case errors.Is(err, store.ErrUsernameTaken):
		writeError(w, http.StatusConflict, CodeUsernameTaken, "用户名已被占用", "")
		return
	case errors.Is(err, store.ErrEmailTaken):
		writeError(w, http.StatusConflict, CodeEmailTaken, "邮箱已被占用", "")
		return
	case err != nil:
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}

	token, err := auth.IssueToken(s.Secret, user.ID, user.Role)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"token": token, "user": toUserDTO(user)})
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/login
// ---------------------------------------------------------------------------

type loginInput struct {
	Username string `json:"username"`
	Password string `json:"password"`
}

func (s *Server) handleLogin(w http.ResponseWriter, r *http.Request) {
	var in loginInput
	if !decodeJSON(w, r, &in, 8<<10) {
		return
	}
	user, err := s.Store.GetUserByUsername(r.Context(), strings.TrimSpace(in.Username))
	if errors.Is(err, store.ErrNotFound) || (err == nil && !auth.CheckPassword(user.PasswordHash, in.Password)) {
		writeError(w, http.StatusUnauthorized, CodeInvalidCredentials, "用户名或密码错误", "")
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
	_ = s.Store.TouchLastLogin(r.Context(), user.ID)

	token, err := auth.IssueToken(s.Secret, user.ID, user.Role)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"token": token, "user": toUserDTO(user)})
}

// ---------------------------------------------------------------------------
// GET /api/v1/auth/me
// ---------------------------------------------------------------------------

func (s *Server) handleMe(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{"user": toUserDTO(userFrom(r.Context()))})
}

// ---------------------------------------------------------------------------
// POST /api/v1/auth/change-password
// ---------------------------------------------------------------------------

type changePasswordInput struct {
	OldPassword string `json:"oldPassword"`
	NewPassword string `json:"newPassword"`
}

func (s *Server) handleChangePassword(w http.ResponseWriter, r *http.Request) {
	var in changePasswordInput
	if !decodeJSON(w, r, &in, 8<<10) {
		return
	}
	if len(in.NewPassword) < 6 || len(in.NewPassword) > 128 {
		writeError(w, http.StatusBadRequest, CodeValidation, "新密码长度需为 6-128 位", "")
		return
	}
	user := userFrom(r.Context())
	fresh, err := s.Store.GetUserByID(r.Context(), user.ID)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	if !auth.CheckPassword(fresh.PasswordHash, in.OldPassword) {
		writeError(w, http.StatusBadRequest, CodeInvalidCredentials, "原密码不正确", "")
		return
	}
	hash, err := auth.HashPassword(in.NewPassword)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	if err := s.Store.UpdatePassword(r.Context(), user.ID, hash); err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"ok": true})
}
