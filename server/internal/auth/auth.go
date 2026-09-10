// Package auth issues and verifies the server's HS256 session JWTs and
// provides bcrypt password hashing.
package auth

import (
	"errors"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"golang.org/x/crypto/bcrypt"
)

// TokenTTL keeps desktop sessions alive for 30 days.
const TokenTTL = 30 * 24 * time.Hour

// Claims carried by every session token.
type Claims struct {
	Role string `json:"role"`
	jwt.RegisteredClaims
}

// ErrInvalid is returned for malformed, expired or wrongly-signed tokens.
var ErrInvalid = errors.New("invalid or expired token")

// HashPassword hashes with bcrypt default cost.
func HashPassword(password string) (string, error) {
	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return "", err
	}
	return string(hash), nil
}

// CheckPassword compares a plaintext password against the stored hash.
func CheckPassword(hash, password string) bool {
	return bcrypt.CompareHashAndPassword([]byte(hash), []byte(password)) == nil
}

// IssueToken signs a 30-day session token for the user.
func IssueToken(secret, userID, role string) (string, error) {
	now := time.Now()
	claims := Claims{
		Role: role,
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   userID,
			IssuedAt:  jwt.NewNumericDate(now),
			ExpiresAt: jwt.NewNumericDate(now.Add(TokenTTL)),
			Issuer:    "rudder-server",
		},
	}
	return jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString([]byte(secret))
}

// VerifyToken parses and validates a token, returning the user id and role.
func VerifyToken(secret, token string) (userID string, role string, err error) {
	parsed, err := jwt.ParseWithClaims(token, &Claims{}, func(t *jwt.Token) (any, error) {
		if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, jwt.ErrSignatureInvalid
		}
		return []byte(secret), nil
	})
	if err != nil || !parsed.Valid {
		return "", "", ErrInvalid
	}
	claims, ok := parsed.Claims.(*Claims)
	if !ok || claims.Subject == "" {
		return "", "", ErrInvalid
	}
	return claims.Subject, claims.Role, nil
}
