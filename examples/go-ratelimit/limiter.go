// Package ratelimit implements a per-key token bucket limiter and an HTTP middleware.
package ratelimit

import (
	"sync"
	"time"
)

type bucket struct {
	tokens float64
	last   time.Time
}

// Limiter hands out tokens per key, refilling at rate tokens per second up to burst.
type Limiter struct {
	rate  float64
	burst float64
	now   func() time.Time

	mu      sync.Mutex
	buckets map[string]*bucket
}

// New returns a limiter that allows rate requests per second with bursts of up to burst.
func New(rate float64, burst int) *Limiter {
	return &Limiter{rate: rate, burst: float64(burst), now: time.Now, buckets: make(map[string]*bucket)}
}

// Allow reports whether one request for key may proceed now, consuming a token if so.
func (l *Limiter) Allow(key string) bool {
	l.mu.Lock()
	defer l.mu.Unlock()

	b := l.refill(key)
	if b.tokens < 1 {
		return false
	}
	b.tokens--
	return true
}

// refill returns the bucket for key with the tokens earned since its last use. Callers hold l.mu.
func (l *Limiter) refill(key string) *bucket {
	now := l.now()
	b, ok := l.buckets[key]
	if !ok {
		b = &bucket{tokens: l.burst, last: now}
		l.buckets[key] = b
	}
	elapsed := now.Sub(b.last).Seconds()
	b.tokens = min(l.burst, b.tokens+elapsed*l.rate)
	b.last = now
	return b
}

// Forget drops buckets idle for longer than maxIdle, to bound memory. It returns how many were dropped.
func (l *Limiter) Forget(maxIdle time.Duration) int {
	l.mu.Lock()
	defer l.mu.Unlock()

	cutoff := l.now().Add(-maxIdle)
	removed := 0
	for key, b := range l.buckets {
		if b.last.Before(cutoff) {
			delete(l.buckets, key)
			removed++
		}
	}
	return removed
}
