package api

import (
	"encoding/json"
	"testing"
	"time"
)

func TestClampN(t *testing.T) {
	cases := []struct {
		in    *int
		want  int
	}{
		{nil, 1}, {intPtr(0), 1}, {intPtr(-3), 1}, {intPtr(1), 1},
		{intPtr(4), 4}, {intPtr(10), 10}, {intPtr(11), 10},
	}
	for _, c := range cases {
		if got := clampN(c.in); got != c.want {
			t.Errorf("clampN(%v) = %d, want %d", c.in, got, c.want)
		}
	}
}

func intPtr(n int) *int { return &n }

func TestValidateGenParams(t *testing.T) {
	if msg, _ := validateGenParams(&genParams{Prompt: "  "}); msg == "" {
		t.Error("empty prompt must be rejected")
	}
	long := make([]byte, 32001)
	for i := range long {
		long[i] = 'a'
	}
	if msg, _ := validateGenParams(&genParams{Prompt: string(long)}); msg == "" {
		t.Error("oversized prompt must be rejected")
	}
	if msg, _ := validateGenParams(&genParams{Prompt: "hello", Size: strPtr("1536x1024"), Quality: strPtr("low")}); msg != "" {
		t.Errorf("valid params rejected: %s", msg)
	}
	if msg, _ := validateGenParams(&genParams{Prompt: "hello", Size: strPtr("")}); msg == "" {
		t.Error("empty size string must be rejected")
	}
}

func strPtr(s string) *string { return &s }

func TestForwardFieldsInjectsModelAndOmitsEmpty(t *testing.T) {
	p := genParams{Prompt: "design system", N: intPtr(2)}
	fields := p.forwardFields("gpt-image-2")
	if fields["model"] != "gpt-image-2" {
		t.Errorf("model must be injected, got %q", fields["model"])
	}
	if fields["n"] != "2" {
		t.Errorf("n = %q, want 2", fields["n"])
	}
	if _, ok := fields["seed"]; ok {
		t.Error("unset seed must be omitted")
	}
	if _, ok := fields["thinking"]; ok {
		t.Error("unset thinking must be omitted")
	}
}

func TestForwardFieldsClampsN(t *testing.T) {
	p := genParams{Prompt: "x", N: intPtr(99)}
	if fields := p.forwardFields("m"); fields["n"] != "10" {
		t.Errorf("n must clamp to 10, got %q", fields["n"])
	}
}

func TestJsonBodyNativeTypes(t *testing.T) {
	p := genParams{Prompt: "x", N: intPtr(2), Seed: uintPtr(7)}
	var parsed map[string]any
	if err := json.Unmarshal(p.jsonBody("m"), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if n, ok := parsed["n"].(float64); !ok || n != 2 {
		t.Errorf("n must be a JSON number, got %T %v", parsed["n"], parsed["n"])
	}
	if s, ok := parsed["seed"].(float64); !ok || s != 7 {
		t.Errorf("seed must be a JSON number, got %T %v", parsed["seed"], parsed["seed"])
	}
	if parsed["model"] != "m" {
		t.Errorf("model = %v", parsed["model"])
	}
}

func uintPtr(n uint64) *uint64 { return &n }

func TestOptParsers(t *testing.T) {
	if optInt("") != nil || optUint("") != nil || optString("") != nil {
		t.Error("empty strings must map to nil")
	}
	if n := optInt("3"); n == nil || *n != 3 {
		t.Errorf("optInt(3) = %v", n)
	}
	if n := optUint("42"); n == nil || *n != 42 {
		t.Errorf("optUint(42) = %v", n)
	}
	if n := optInt("abc"); n != nil {
		t.Error("invalid int must map to nil")
	}
}

func TestUpstreamBudgetUnderNginx(t *testing.T) {
	s := Server{UpstreamTimeout: 120 * time.Second, UpstreamRetries: 1}
	if got := s.upstreamBudget(); got >= nginxReadLimit {
		t.Errorf("budget %s must stay under nginx proxy_read_timeout %s", got, nginxReadLimit)
	}
}

const nginxReadLimit = 330 * time.Second
