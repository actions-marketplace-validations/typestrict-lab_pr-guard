package ratelimit

import (
	"log"
	"net"
	"net/http"
)

// KeyFunc extracts the rate limit key from a request.
type KeyFunc func(*http.Request) (string, error)

// ClientIP keys requests by the remote address of the TCP connection.
func ClientIP(r *http.Request) (string, error) {
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return "", err
	}
	return host, nil
}

// Middleware rejects requests over the limit with 429 Too Many Requests.
func Middleware(l *Limiter, key KeyFunc, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		k, err := key(r)
		if err != nil {
			log.Printf("ratelimit: %v", err)
			http.Error(w, "bad request", http.StatusBadRequest)
			return
		}
		if !l.Allow(k) {
			http.Error(w, "rate limit exceeded", http.StatusTooManyRequests)
			return
		}
		next.ServeHTTP(w, r)
	})
}
