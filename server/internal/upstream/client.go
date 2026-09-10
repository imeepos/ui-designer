// Package upstream is the OpenAI-compatible image API client the server uses
// to reach the real provider. Mirrors the retry semantics of the desktop
// client (429/5xx and malformed 2xx bodies, exponential backoff, ≤3 retries).
package upstream

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"strings"
	"time"
)

// Client calls the upstream provider with the server-held key.
type Client struct {
	BaseURL string
	APIKey  string
	HTTP    *http.Client
	// Backoff is the base delay between retries (×1, ×2, ×4).
	Backoff time.Duration
	// MaxRetries after the initial attempt.
	MaxRetries int
}

// New builds a client with production defaults (300s per attempt, 3 retries).
func New(baseURL, apiKey string, timeout time.Duration) *Client {
	return NewWithRetry(baseURL, apiKey, timeout, 3)
}

// NewWithRetry builds a client with an explicit retry budget (server side the
// desktop client retries too, so 1 extra attempt is usually enough).
func NewWithRetry(baseURL, apiKey string, timeout time.Duration, maxRetries int) *Client {
	return &Client{
		BaseURL:    strings.TrimRight(baseURL, "/"),
		APIKey:     apiKey,
		HTTP:       &http.Client{Timeout: timeout},
		Backoff:    2 * time.Second,
		MaxRetries: maxRetries,
	}
}

// Error carries the upstream HTTP status and a truncated body summary so the
// caller can map it onto the API response.
type Error struct {
	Status  int
	Summary string
}

func (e *Error) Error() string {
	return fmt.Sprintf("upstream status %d: %s", e.Status, e.Summary)
}

// Generate posts a JSON generations request and returns the raw upstream
// JSON body (validated to contain data[].b64_json).
func (c *Client) Generate(ctx context.Context, body []byte) (json.RawMessage, error) {
	return c.send(ctx, http.MethodPost, c.BaseURL+"/v1/images/generations", "application/json", body)
}

// Edit posts a multipart edits request: text fields plus one or more
// `image[]` file parts (anchor first, mirroring the desktop client).
func (c *Client) Edit(ctx context.Context, fields map[string]string, images []ImagePart) (json.RawMessage, error) {
	var buf bytes.Buffer
	writer := multipart.NewWriter(&buf)
	for k, v := range fields {
		if err := writer.WriteField(k, v); err != nil {
			return nil, err
		}
	}
	for _, img := range images {
		part, err := writer.CreateFormFile("image[]", img.Filename)
		if err != nil {
			return nil, err
		}
		if _, err := part.Write(img.Data); err != nil {
			return nil, err
		}
	}
	if err := writer.Close(); err != nil {
		return nil, err
	}
	return c.send(ctx, http.MethodPost, c.BaseURL+"/v1/images/edits", writer.FormDataContentType(), buf.Bytes())
}

// ImagePart is one uploaded reference image.
type ImagePart struct {
	Filename string
	Data     []byte
}

// send executes the request with retry on 429/5xx and malformed 2xx bodies.
func (c *Client) send(ctx context.Context, method, url, contentType string, body []byte) (json.RawMessage, error) {
	var lastErr error
	for attempt := 0; ; attempt++ {
		req, err := http.NewRequestWithContext(ctx, method, url, bytes.NewReader(body))
		if err != nil {
			return nil, err
		}
		req.Header.Set("Content-Type", contentType)
		req.Header.Set("Authorization", "Bearer "+c.APIKey)

		resp, err := c.HTTP.Do(req)
		if err != nil {
			lastErr = &Error{Status: 0, Summary: err.Error()}
			if attempt < c.MaxRetries {
				if !sleepCtx(ctx, c.Backoff*(1<<attempt)) {
					return nil, lastErr
				}
				continue
			}
			return nil, lastErr
		}

		data, err := io.ReadAll(io.LimitReader(resp.Body, 256<<20))
		resp.Body.Close()
		if err != nil {
			lastErr = &Error{Status: resp.StatusCode, Summary: "read body: " + err.Error()}
			if attempt < c.MaxRetries {
				if !sleepCtx(ctx, c.Backoff*(1<<attempt)) {
					return nil, lastErr
				}
				continue
			}
			return nil, lastErr
		}

		if resp.StatusCode >= 200 && resp.StatusCode < 300 {
			if err := validateB64Payload(data); err != nil {
				// A 2xx without usable images is treated like a transient
				// glitch (some proxies degrade the payload) and retried.
				lastErr = &Error{Status: resp.StatusCode, Summary: err.Error()}
				if attempt < c.MaxRetries {
					if !sleepCtx(ctx, c.Backoff*(1<<attempt)) {
						return nil, lastErr
					}
					continue
				}
				return nil, lastErr
			}
			return json.RawMessage(data), nil
		}

		lastErr = &Error{Status: resp.StatusCode, Summary: summarize(data)}
		retryable := resp.StatusCode == http.StatusTooManyRequests || resp.StatusCode >= 500
		if retryable && attempt < c.MaxRetries {
			if !sleepCtx(ctx, c.Backoff*(1<<attempt)) {
				return nil, lastErr
			}
			continue
		}
		return nil, lastErr
	}
}

// validateB64Payload ensures the 2xx body actually contains images.
func validateB64Payload(data []byte) error {
	var payload struct {
		Data []struct {
			B64JSON string `json:"b64_json"`
		} `json:"data"`
	}
	if err := json.Unmarshal(data, &payload); err != nil {
		return fmt.Errorf("body is not JSON: %v", err)
	}
	if len(payload.Data) == 0 {
		return fmt.Errorf("no `data` array in response")
	}
	for i, item := range payload.Data {
		if item.B64JSON == "" {
			return fmt.Errorf("data[%d] has no b64_json (only b64 responses are supported)", i)
		}
	}
	return nil
}

func summarize(data []byte) string {
	s := strings.Map(func(r rune) rune {
		if r == '\n' || r == '\r' || r == '\t' {
			return ' '
		}
		return r
	}, string(data))
	if len(s) > 200 {
		return s[:200] + "…"
	}
	return s
}

// sleepCtx sleeps or aborts early when the context is cancelled.
func sleepCtx(ctx context.Context, d time.Duration) bool {
	timer := time.NewTimer(d)
	defer timer.Stop()
	select {
	case <-ctx.Done():
		return false
	case <-timer.C:
		return true
	}
}
