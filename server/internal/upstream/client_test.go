package upstream

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

const okBody = `{"created":1,"data":[{"b64_json":"aGVsbG8="},{"b64_json":"d29ybGQ="}]}`

func TestGenerateSuccess(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1/images/generations" {
			t.Errorf("path = %s", r.URL.Path)
		}
		if got := r.Header.Get("Authorization"); got != "Bearer sk-test" {
			t.Errorf("auth header = %q", got)
		}
		var body map[string]any
		_ = json.NewDecoder(r.Body).Decode(&body)
		if body["model"] != "gpt-image-2" || body["prompt"] != "p" {
			t.Errorf("body = %v", body)
		}
		w.Write([]byte(okBody))
	}))
	defer srv.Close()

	c := NewWithRetry(srv.URL, "sk-test", 5*time.Second, 0)
	out, err := c.Generate(context.Background(), []byte(`{"model":"gpt-image-2","prompt":"p"}`))
	if err != nil {
		t.Fatalf("Generate: %v", err)
	}
	if !strings.Contains(string(out), "b64_json") {
		t.Errorf("body = %s", out)
	}
}

func TestRetryOnBad2xxThenSuccess(t *testing.T) {
	var calls atomic.Int32
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if calls.Add(1) == 1 {
			w.Write([]byte(`{"data":[]}`)) // malformed payload → retry
			return
		}
		w.Write([]byte(okBody))
	}))
	defer srv.Close()

	c := NewWithRetry(srv.URL, "k", 5*time.Second, 3)
	c.Backoff = time.Millisecond
	if _, err := c.Generate(context.Background(), nil); err != nil {
		t.Fatalf("expected retry to succeed, got %v", err)
	}
	if calls.Load() != 2 {
		t.Errorf("calls = %d, want 2", calls.Load())
	}
}

func TestRetryOn429ThenSuccess(t *testing.T) {
	var calls atomic.Int32
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if calls.Add(1) == 1 {
			http.Error(w, `{"error":{"message":"slow down"}}`, http.StatusTooManyRequests)
			return
		}
		w.Write([]byte(okBody))
	}))
	defer srv.Close()

	c := NewWithRetry(srv.URL, "k", 5*time.Second, 3)
	c.Backoff = time.Millisecond
	if _, err := c.Generate(context.Background(), nil); err != nil {
		t.Fatalf("expected success after 429 retry: %v", err)
	}
}

func TestExhaustRetriesMapsStatus(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "boom", http.StatusInternalServerError)
	}))
	defer srv.Close()

	c := NewWithRetry(srv.URL, "k", 5*time.Second, 1)
	c.Backoff = time.Millisecond
	_, err := c.Generate(context.Background(), nil)
	uerr, ok := err.(*Error)
	if !ok {
		t.Fatalf("want *Error, got %T %v", err, err)
	}
	if uerr.Status != http.StatusInternalServerError {
		t.Errorf("want status 500, got %d", uerr.Status)
	}
}

func TestEditMultipartShape(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if err := r.ParseMultipartForm(10 << 20); err != nil {
			t.Errorf("parse multipart: %v", err)
		}
		if got := len(r.MultipartForm.File["image[]"]); got != 2 {
			t.Errorf("image[] parts = %d, want 2", got)
		}
		if r.FormValue("prompt") != "p" || r.FormValue("model") != "m" {
			t.Errorf("fields = %v", r.Form)
		}
		w.Write([]byte(okBody))
	}))
	defer srv.Close()

	c := NewWithRetry(srv.URL, "k", 5*time.Second, 0)
	_, err := c.Edit(context.Background(), map[string]string{"model": "m", "prompt": "p"}, []ImagePart{
		{Filename: "anchor.png", Data: []byte("a")},
		{Filename: "ref.png", Data: []byte("b")},
	})
	if err != nil {
		t.Fatalf("Edit: %v", err)
	}
}
