// Package api wires the HTTP surface of rudder-server.
package api

import (
	"encoding/json"
	"net/http"
)

// API error codes (stable machine labels, mirroring the desktop client's
// error taxonomy where applicable).
const (
	CodeUnauthorized        = "UNAUTHORIZED"
	CodeForbidden           = "FORBIDDEN"
	CodeNotFound            = "NOT_FOUND"
	CodeValidation          = "VALIDATION"
	CodeUsernameTaken       = "USERNAME_TAKEN"
	CodeEmailTaken          = "EMAIL_TAKEN"
	CodeInvalidCredentials  = "INVALID_CREDENTIALS"
	CodeUserDisabled        = "USER_DISABLED"
	CodeRegistrationClosed  = "REGISTRATION_CLOSED"
	CodeInsufficientCredits = "INSUFFICIENT_CREDITS"
	CodeUpstreamError       = "UPSTREAM_ERROR"
	CodeUpstreamMisconfig   = "UPSTREAM_MISCONFIGURED"
	CodeRateLimited         = "RATE_LIMITED"
	CodeInternal            = "INTERNAL"
)

type errorBody struct {
	Error errInner `json:"error"`
}

type errInner struct {
	Code    string `json:"code"`
	Message string `json:"message"`
	Hint    string `json:"hint,omitempty"`
}

// writeError emits the standard `{error:{code,message,hint}}` envelope.
func writeError(w http.ResponseWriter, status int, code, message, hint string) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(errorBody{Error: errInner{Code: code, Message: message, Hint: hint}})
}

// writeJSON emits a success payload.
func writeJSON(w http.ResponseWriter, status int, payload any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(payload)
}
