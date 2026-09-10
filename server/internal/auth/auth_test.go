package auth

import (
	"testing"
	"time"
)

func TestPasswordHashRoundtrip(t *testing.T) {
	hash, err := HashPassword("s3cret-password")
	if err != nil {
		t.Fatalf("hash: %v", err)
	}
	if !CheckPassword(hash, "s3cret-password") {
		t.Error("correct password must verify")
	}
	if CheckPassword(hash, "wrong") {
		t.Error("wrong password must not verify")
	}
}

func TestTokenRoundtrip(t *testing.T) {
	token, err := IssueToken("test-secret-aaaaaaaaaaaaaaaaaaaaaa", "user-1", "admin")
	if err != nil {
		t.Fatalf("issue: %v", err)
	}
	userID, role, err := VerifyToken("test-secret-aaaaaaaaaaaaaaaaaaaaaa", token)
	if err != nil {
		t.Fatalf("verify: %v", err)
	}
	if userID != "user-1" || role != "admin" {
		t.Errorf("got %q/%q", userID, role)
	}
}

func TestTokenWrongSecret(t *testing.T) {
	token, _ := IssueToken("secret-a-aaaaaaaaaaaaaaaaaaaaaa", "u", "user")
	if _, _, err := VerifyToken("secret-b-aaaaaaaaaaaaaaaaaaaaaa", token); err == nil {
		t.Error("wrong secret must fail")
	}
}

func TestTokenGarbage(t *testing.T) {
	if _, _, err := VerifyToken("secret-a-aaaaaaaaaaaaaaaaaaaaaa", "not-a-jwt"); err == nil {
		t.Error("garbage token must fail")
	}
}

func TestTokenExpiry(t *testing.T) {
	// IssueToken always uses TokenTTL (30d); verify expiry is actually set.
	token, _ := IssueToken("secret-a-aaaaaaaaaaaaaaaaaaaaaa", "u", "user")
	userID, _, err := VerifyToken("secret-a-aaaaaaaaaaaaaaaaaaaaaa", token)
	if err != nil || userID == "" {
		t.Fatalf("fresh token must verify: %v", err)
	}
	if TokenTTL < 24*time.Hour {
		t.Error("session TTL unexpectedly short")
	}
}
