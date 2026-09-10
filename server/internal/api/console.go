package api

import (
	"net/http"

	"rudder-server/internal/adminui"
)

// handleAdminConsole serves the embedded single-file admin console. The HTML
// itself is public; every privileged call inside it still requires an admin
// JWT.
func (s *Server) handleAdminConsole(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Header().Set("Cache-Control", "no-store")
	_, _ = w.Write(adminui.Page())
}
