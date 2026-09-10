package api

import (
	"errors"
	"net/http"
	"strconv"
	"strings"

	"rudder-server/internal/auth"
	"rudder-server/internal/store"
)

// ---------------------------------------------------------------------------
// GET /api/v1/admin/stats
// ---------------------------------------------------------------------------

func (s *Server) handleAdminStats(w http.ResponseWriter, r *http.Request) {
	st, err := s.Store.LoadStats(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, st)
}

// ---------------------------------------------------------------------------
// GET /api/v1/admin/users?query=&limit=&offset=
// ---------------------------------------------------------------------------

func (s *Server) handleAdminUsers(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	limit, _ := strconv.Atoi(q.Get("limit"))
	offset, _ := strconv.Atoi(q.Get("offset"))
	list, err := s.Store.ListUsers(r.Context(), q.Get("query"), limit, offset)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	users := make([]userDTO, 0, len(list.Users))
	for i := range list.Users {
		users = append(users, toUserDTO(&list.Users[i]))
	}
	writeJSON(w, http.StatusOK, map[string]any{"total": list.Total, "users": users})
}

// ---------------------------------------------------------------------------
// PATCH /api/v1/admin/users/{id}  {status?, role?}
// ---------------------------------------------------------------------------

type patchUserInput struct {
	Status *string `json:"status"`
	Role   *string `json:"role"`
}

func (s *Server) handleAdminUserPatch(w http.ResponseWriter, r *http.Request) {
	var in patchUserInput
	if !decodeJSON(w, r, &in, 8<<10) {
		return
	}
	if in.Status != nil && *in.Status != "active" && *in.Status != "disabled" {
		writeError(w, http.StatusBadRequest, CodeValidation, "status 只能是 active 或 disabled", "")
		return
	}
	if in.Role != nil && *in.Role != "user" && *in.Role != "admin" {
		writeError(w, http.StatusBadRequest, CodeValidation, "role 只能是 user 或 admin", "")
		return
	}
	if in.Status != nil && userFrom(r.Context()).ID == r.PathValue("id") && *in.Status == "disabled" {
		writeError(w, http.StatusBadRequest, CodeValidation, "不能禁用自己", "")
		return
	}
	user, err := s.Store.UpdateProfile(r.Context(), r.PathValue("id"), in.Status, in.Role)
	if errors.Is(err, store.ErrNotFound) {
		writeError(w, http.StatusNotFound, CodeNotFound, "用户不存在", "")
		return
	}
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"user": toUserDTO(user)})
}

// ---------------------------------------------------------------------------
// POST /api/v1/admin/users/{id}/credits  {amount, note?}
// ---------------------------------------------------------------------------

type adjustCreditsInput struct {
	Amount int64  `json:"amount"`
	Note   string `json:"note"`
}

func (s *Server) handleAdminCredits(w http.ResponseWriter, r *http.Request) {
	var in adjustCreditsInput
	if !decodeJSON(w, r, &in, 8<<10) {
		return
	}
	if in.Amount == 0 {
		writeError(w, http.StatusBadRequest, CodeValidation, "amount 不能为 0", "")
		return
	}
	if in.Amount < -1_000_000 || in.Amount > 1_000_000 {
		writeError(w, http.StatusBadRequest, CodeValidation, "amount 超出范围", "")
		return
	}
	if in.Note == "" {
		if in.Amount > 0 {
			in.Note = "管理员充值"
		} else {
			in.Note = "管理员扣减"
		}
	}
	user, err := s.Store.AdjustCredits(r.Context(), r.PathValue("id"), in.Amount, in.Note)
	switch {
	case errors.Is(err, store.ErrNotFound):
		writeError(w, http.StatusNotFound, CodeNotFound, "用户不存在", "")
		return
	case errors.Is(err, store.ErrInsufficientCredits):
		writeError(w, http.StatusBadRequest, CodeInsufficientCredits, "扣减后余额不能为负", "")
		return
	case err != nil:
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"user": toUserDTO(user)})
}

// ---------------------------------------------------------------------------
// POST /api/v1/admin/users/{id}/reset-password  {newPassword}
// ---------------------------------------------------------------------------

type resetPasswordInput struct {
	NewPassword string `json:"newPassword"`
}

func (s *Server) handleAdminResetPassword(w http.ResponseWriter, r *http.Request) {
	var in resetPasswordInput
	if !decodeJSON(w, r, &in, 8<<10) {
		return
	}
	if len(in.NewPassword) < 6 || len(in.NewPassword) > 128 {
		writeError(w, http.StatusBadRequest, CodeValidation, "密码长度需为 6-128 位", "")
		return
	}
	hash, err := auth.HashPassword(in.NewPassword)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	if err := s.Store.UpdatePassword(r.Context(), r.PathValue("id"), hash); err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"ok": true})
}

// ---------------------------------------------------------------------------
// GET/PUT /api/v1/admin/settings — price + upstream address
// ---------------------------------------------------------------------------

type settingsDTO struct {
	CreditsPerImage    int64   `json:"creditsPerImage"`
	SignupBonusCredits int64   `json:"signupBonusCredits"`
	RegistrationOpen   bool    `json:"registrationOpen"`
	UpstreamBaseURL    string  `json:"upstreamBaseUrl"`
	UpstreamModel      string  `json:"upstreamModel"`
	UpstreamKeyTail    string  `json:"upstreamKeyTail"` // 只有尾 4 位，绝不回传全量
	UpstreamConfigured bool    `json:"upstreamConfigured"`
}

func (s *Server) handleAdminGetSettings(w http.ResponseWriter, r *http.Request) {
	settings, err := s.Store.LoadSettings(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, settingsDTO{
		CreditsPerImage:    settings.CreditsPerImage,
		SignupBonusCredits: settings.SignupBonusCredits,
		RegistrationOpen:   settings.RegistrationOpen,
		UpstreamBaseURL:    settings.UpstreamBaseURL,
		UpstreamModel:      settings.UpstreamModel,
		UpstreamKeyTail:    store.Tail4(settings.UpstreamAPIKey),
		UpstreamConfigured: settings.UpstreamAPIKey != "",
	})
}

type putSettingsInput struct {
	CreditsPerImage    *int64  `json:"creditsPerImage"`
	SignupBonusCredits *int64  `json:"signupBonusCredits"`
	RegistrationOpen   *bool   `json:"registrationOpen"`
	UpstreamBaseURL    *string `json:"upstreamBaseUrl"`
	UpstreamModel      *string `json:"upstreamModel"`
	// UpstreamAPIKey 留空字段表示不修改；显式空串清除。
	UpstreamAPIKey *string `json:"upstreamApiKey"`
}

func (s *Server) handleAdminPutSettings(w http.ResponseWriter, r *http.Request) {
	var in putSettingsInput
	if !decodeJSON(w, r, &in, 16<<10) {
		return
	}
	ctx := r.Context()
	applyString := func(key string, value *string, maxLen int) error {
		if value == nil {
			return nil
		}
		v := strings.TrimSpace(*value)
		if len(v) > maxLen {
			writeError(w, http.StatusBadRequest, CodeValidation, key+" 过长", "")
			return errAbort
		}
		return s.Store.SetSetting(ctx, key, v)
	}
	if in.CreditsPerImage != nil {
		if *in.CreditsPerImage < 0 || *in.CreditsPerImage > 1_000_000 {
			writeError(w, http.StatusBadRequest, CodeValidation, "creditsPerImage 需在 0-1000000 之间", "")
			return
		}
		if err := s.Store.SetSetting(ctx, store.KeyCreditsPerImage, strconv.FormatInt(*in.CreditsPerImage, 10)); err != nil {
			writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
			return
		}
	}
	if in.SignupBonusCredits != nil {
		if *in.SignupBonusCredits < 0 || *in.SignupBonusCredits > 1_000_000 {
			writeError(w, http.StatusBadRequest, CodeValidation, "signupBonusCredits 需在 0-1000000 之间", "")
			return
		}
		if err := s.Store.SetSetting(ctx, store.KeySignupBonusCredits, strconv.FormatInt(*in.SignupBonusCredits, 10)); err != nil {
			writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
			return
		}
	}
	if in.RegistrationOpen != nil {
		v := "false"
		if *in.RegistrationOpen {
			v = "true"
		}
		if err := s.Store.SetSetting(ctx, store.KeyRegistrationOpen, v); err != nil {
			writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
			return
		}
	}
	if err := applyString(store.KeyUpstreamBaseURL, in.UpstreamBaseURL, 512); err != nil {
		if err == errAbort {
			return
		}
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	if err := applyString(store.KeyUpstreamModel, in.UpstreamModel, 128); err != nil {
		if err == errAbort {
			return
		}
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	if in.UpstreamAPIKey != nil {
		v := strings.TrimSpace(*in.UpstreamAPIKey)
		if len(v) > 512 {
			writeError(w, http.StatusBadRequest, CodeValidation, "upstreamApiKey 过长", "")
			return
		}
		if err := s.Store.SetSetting(ctx, store.KeyUpstreamAPIKey, v); err != nil {
			writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
			return
		}
	}
	s.handleAdminGetSettings(w, r)
}

var errAbort = errors.New("abort")

// ---------------------------------------------------------------------------
// GET /api/v1/admin/generations?limit=&userId=
// ---------------------------------------------------------------------------

func (s *Server) handleAdminGenerations(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	limit, _ := strconv.Atoi(q.Get("limit"))
	generations, err := s.Store.ListGenerations(r.Context(), q.Get("userId"), limit)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"generations": generations})
}
