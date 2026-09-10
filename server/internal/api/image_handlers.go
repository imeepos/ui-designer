package api

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"log"
	"net/http"
	"strconv"
	"strings"
	"time"

	"rudder-server/internal/store"
	"rudder-server/internal/upstream"
)

// maxImageBody caps uploads (JSON or multipart) at 32 MiB — four PNG
// reference images fit comfortably.
const maxImageBody = 32 << 20

// genParams is the OpenAI-shaped body the desktop client sends. Only the
// fields present are forwarded upstream; `model` is always overridden with
// the server-configured model (client zero-config).
type genParams struct {
	Model    *string `json:"model,omitempty"`
	Prompt   string  `json:"prompt"`
	Size     *string `json:"size,omitempty"`
	Quality  *string `json:"quality,omitempty"`
	N        *int    `json:"n,omitempty"`
	Seed     *uint64 `json:"seed,omitempty"`
	Thinking *string `json:"thinking,omitempty"`
	// `user` is accepted for OpenAI compatibility and ignored.
	User *string `json:"user,omitempty"`
}

// clampN bounds the image count per call.
func clampN(n *int) int {
	if n == nil || *n < 1 {
		return 1
	}
	if *n > 10 {
		return 10
	}
	return *n
}

// validateGenParams enforces the shared field rules.
func validateGenParams(p *genParams) (string, string) {
	if strings.TrimSpace(p.Prompt) == "" {
		return "prompt 不能为空", ""
	}
	if len(p.Prompt) > 32000 {
		return "prompt 过长（>32000 字符）", ""
	}
	if p.Size != nil && (len(*p.Size) == 0 || len(*p.Size) > 32) {
		return "size 参数不合法", ""
	}
	if p.Quality != nil && (len(*p.Quality) == 0 || len(*p.Quality) > 32) {
		return "quality 参数不合法", ""
	}
	if p.Thinking != nil && (len(*p.Thinking) == 0 || len(*p.Thinking) > 32) {
		return "thinking 参数不合法", ""
	}
	return "", ""
}

// forwardFields builds the multipart text fields (all values are strings
// there); the model is injected server-side so clients stay zero-config.
func (p *genParams) forwardFields(model string) map[string]string {
	fields := map[string]string{"model": model, "prompt": p.Prompt}
	if p.Size != nil {
		fields["size"] = *p.Size
	}
	if p.Quality != nil {
		fields["quality"] = *p.Quality
	}
	if p.N != nil {
		fields["n"] = strconv.Itoa(clampN(p.N))
	}
	if p.Seed != nil {
		fields["seed"] = strconv.FormatUint(*p.Seed, 10)
	}
	if p.Thinking != nil {
		fields["thinking"] = *p.Thinking
	}
	return fields
}

// jsonBody builds the JSON generations body with NATIVE types — upstream
// rejects `"n":"1"` (string) with `invalid n field type`.
func (p *genParams) jsonBody(model string) []byte {
	body := map[string]any{"model": model, "prompt": p.Prompt}
	if p.Size != nil {
		body["size"] = *p.Size
	}
	if p.Quality != nil {
		body["quality"] = *p.Quality
	}
	body["n"] = clampN(p.N)
	if p.Seed != nil {
		body["seed"] = *p.Seed
	}
	if p.Thinking != nil {
		body["thinking"] = *p.Thinking
	}
	out, _ := json.Marshal(body)
	return out
}

// charge reserves credits (admins are exempt) and records the generation row.
// On failure it has already written the HTTP error and reports ok=false.
func (s *Server) charge(w http.ResponseWriter, r *http.Request, p *genParams, kind string) (*store.Settings, string, int64, int64, bool) {
	user := userFrom(r.Context())
	settings, err := s.Store.LoadSettings(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return nil, "", 0, 0, false
	}
	n := clampN(p.N)
	cost := int64(0)
	if !user.IsAdmin() {
		cost = settings.CreditsPerImage * int64(n)
	}
	paramsJSON, _ := json.Marshal(map[string]any{
		"prompt": p.Prompt, "size": p.Size, "quality": p.Quality,
		"n": n, "seed": p.Seed, "thinking": p.Thinking,
	})
	genID, balance, err := s.Store.ReserveGeneration(r.Context(), user.ID, cost, kind, settings.UpstreamModel, n, paramsJSON)
	if errors.Is(err, store.ErrInsufficientCredits) {
		writeError(w, http.StatusPaymentRequired, CodeInsufficientCredits,
			"积分不足", "本次生成需要 "+strconv.FormatInt(cost, 10)+" 积分，请联系管理员充值")
		return nil, "", 0, 0, false
	}
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return nil, "", 0, 0, false
	}
	return &settings, genID, balance, cost, true
}

// settleOK completes a successful generation and responds with the upstream
// JSON body enriched with billing info.
func (s *Server) settleOK(w http.ResponseWriter, r *http.Request, genID string, balance, charged int64, body json.RawMessage) {
	if err := s.Store.CompleteGeneration(r.Context(), genID); err != nil {
		log.Printf("complete generation %s: %v", genID, err)
	}
	var payload map[string]any
	if err := json.Unmarshal(body, &payload); err != nil {
		payload = map[string]any{}
	}
	payload["creditsCharged"] = charged
	payload["balance"] = balance
	payload["generationId"] = genID
	writeJSON(w, http.StatusOK, payload)
}

// settleFailure refunds when charged and maps the upstream error onto the
// response. Refunding uses a fresh context: the request may already be dead.
func (s *Server) settleFailure(w http.ResponseWriter, r *http.Request, genID string, charged int64, uerr error) {
	var uerrHTTP *upstream.Error
	status := 0
	summary := uerr.Error()
	if errors.As(uerr, &uerrHTTP) {
		status = uerrHTTP.Status
		summary = uerrHTTP.Summary
	}
	refundCtx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()
	if charged > 0 {
		if err := s.Store.RefundGeneration(refundCtx, genID, status, summary); err != nil {
			log.Printf("refund generation %s: %v", genID, err)
		}
	} else if err := s.Store.FailGeneration(refundCtx, genID, status, summary); err != nil {
		log.Printf("fail generation %s: %v", genID, err)
	}

	if errors.Is(uerr, errUpstreamMisconfigured) {
		writeError(w, http.StatusServiceUnavailable, CodeUpstreamMisconfig,
			"服务端上游地址未配置", "请联系管理员在后台配置上游地址")
		return
	}
	if uerrHTTP != nil && uerrHTTP.Status == http.StatusTooManyRequests {
		writeError(w, http.StatusTooManyRequests, CodeRateLimited, "上游限流，稍后重试", summary)
		return
	}
	if uerrHTTP != nil && uerrHTTP.Status >= 400 && uerrHTTP.Status < 500 {
		writeError(w, http.StatusBadGateway, CodeUpstreamError, "上游拒绝了请求", summary)
		return
	}
	writeError(w, http.StatusBadGateway, CodeUpstreamError, "上游调用失败（已退还积分）", summary)
}

// ---------------------------------------------------------------------------
// POST /api/v1/images/generations
// ---------------------------------------------------------------------------

func (s *Server) handleGenerations(w http.ResponseWriter, r *http.Request) {
	var p genParams
	if !decodeJSON(w, r, &p, maxImageBody) {
		return
	}
	if msg, hint := validateGenParams(&p); msg != "" {
		writeError(w, http.StatusBadRequest, CodeValidation, msg, hint)
		return
	}
	settings, genID, balance, charged, ok := s.charge(w, r, &p, "generations")
	if !ok {
		return
	}
	client, err := s.newUpstream(*settings)
	if err != nil {
		s.settleFailure(w, r, genID, charged, err)
		return
	}
	body := p.jsonBody(settings.UpstreamModel)
	ctx, cancel := context.WithTimeout(r.Context(), s.upstreamBudget())
	defer cancel()
	respBody, err := client.Generate(ctx, body)
	if err != nil {
		s.settleFailure(w, r, genID, charged, err)
		return
	}
	s.settleOK(w, r, genID, balance, charged, respBody)
}

// ---------------------------------------------------------------------------
// POST /api/v1/images/edits — multipart with image[] parts
// ---------------------------------------------------------------------------

func (s *Server) handleEdits(w http.ResponseWriter, r *http.Request) {
	if err := r.ParseMultipartForm(maxImageBody); err != nil {
		writeError(w, http.StatusBadRequest, CodeValidation, "需要 multipart/form-data 请求", err.Error())
		return
	}
	p := genParams{
		Model:    optString(r.FormValue("model")),
		Prompt:   r.FormValue("prompt"),
		Size:     optString(r.FormValue("size")),
		Quality:  optString(r.FormValue("quality")),
		N:        optInt(r.FormValue("n")),
		Seed:     optUint(r.FormValue("seed")),
		Thinking: optString(r.FormValue("thinking")),
	}
	if msg, hint := validateGenParams(&p); msg != "" {
		writeError(w, http.StatusBadRequest, CodeValidation, msg, hint)
		return
	}
	if r.MultipartForm == nil || len(r.MultipartForm.File["image[]"]) == 0 {
		writeError(w, http.StatusBadRequest, CodeValidation, "edits 需要至少一张 image[] 参考图（锚点图在前）", "")
		return
	}
	images := []upstream.ImagePart{}
	for _, fh := range r.MultipartForm.File["image[]"] {
		file, err := fh.Open()
		if err != nil {
			writeError(w, http.StatusBadRequest, CodeValidation, "无法读取上传图片: "+fh.Filename, "")
			return
		}
		var buf bytes.Buffer
		if _, err := buf.ReadFrom(file); err != nil {
			file.Close()
			writeError(w, http.StatusBadRequest, CodeValidation, "读取上传图片失败: "+fh.Filename, "")
			return
		}
		file.Close()
		images = append(images, upstream.ImagePart{Filename: fh.Filename, Data: buf.Bytes()})
	}

	settings, genID, balance, charged, ok := s.charge(w, r, &p, "edits")
	if !ok {
		return
	}
	client, err := s.newUpstream(*settings)
	if err != nil {
		s.settleFailure(w, r, genID, charged, err)
		return
	}
	ctx, cancel := context.WithTimeout(r.Context(), s.upstreamBudget())
	defer cancel()
	respBody, err := client.Edit(ctx, p.forwardFields(settings.UpstreamModel), images)
	if err != nil {
		s.settleFailure(w, r, genID, charged, err)
		return
	}
	s.settleOK(w, r, genID, balance, charged, respBody)
}

// ---------------------------------------------------------------------------
// GET /api/v1/models — compatibility probe (free, no generation)
// ---------------------------------------------------------------------------

func (s *Server) handleModels(w http.ResponseWriter, r *http.Request) {
	settings, err := s.Store.LoadSettings(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"object": "list",
		"data": []map[string]any{{
			"id": settings.UpstreamModel, "object": "model", "owned_by": "rudder-server",
		}},
	})
}

// ---------------------------------------------------------------------------
// GET /api/v1/usage — balance + recent ledger + generations
// ---------------------------------------------------------------------------

func (s *Server) handleUsage(w http.ResponseWriter, r *http.Request) {
	user := userFrom(r.Context())
	settings, err := s.Store.LoadSettings(r.Context())
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	fresh, err := s.Store.GetUserByID(r.Context(), user.ID)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	transactions, err := s.Store.ListTransactions(r.Context(), user.ID, 50)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	generations, err := s.Store.ListGenerations(r.Context(), user.ID, 50)
	if err != nil {
		writeError(w, http.StatusInternalServerError, CodeInternal, "internal error", "")
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{
		"balance":         fresh.Credits,
		"creditsPerImage": settings.CreditsPerImage,
		"transactions":    transactions,
		"generations":     generations,
	})
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

// upstreamBudget is the whole-call ceiling across all attempts (kept under
// nginx's 330s proxy_read_timeout so the client never sees a silent cut).
func (s *Server) upstreamBudget() time.Duration {
	attempts := s.UpstreamRetries + 1
	return time.Duration(attempts)*s.UpstreamTimeout + time.Duration(s.UpstreamRetries)*2*time.Second
}

func optString(v string) *string {
	if v == "" {
		return nil
	}
	return &v
}

func optInt(v string) *int {
	if v == "" {
		return nil
	}
	n, err := strconv.Atoi(strings.TrimSpace(v))
	if err != nil {
		return nil
	}
	return &n
}

func optUint(v string) *uint64 {
	if v == "" {
		return nil
	}
	n, err := strconv.ParseUint(strings.TrimSpace(v), 10, 64)
	if err != nil {
		return nil
	}
	return &n
}
