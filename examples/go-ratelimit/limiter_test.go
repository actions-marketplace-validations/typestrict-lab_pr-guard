package ratelimit

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

type fakeClock struct{ t time.Time }

func (c *fakeClock) Now() time.Time          { return c.t }
func (c *fakeClock) Advance(d time.Duration) { c.t = c.t.Add(d) }

func newTestLimiter(rate float64, burst int) (*Limiter, *fakeClock) {
	clock := &fakeClock{t: time.Unix(1_700_000_000, 0)}
	l := New(rate, burst)
	l.now = clock.Now
	return l, clock
}

func TestBurstThenRefill(t *testing.T) {
	l, clock := newTestLimiter(1, 2)
	if !l.Allow("a") || !l.Allow("a") {
		t.Fatal("a burst of 2 should be allowed")
	}
	if l.Allow("a") {
		t.Fatal("the third request should be limited")
	}
	clock.Advance(time.Second)
	if !l.Allow("a") {
		t.Fatal("one token should refill after a second")
	}
}

func TestKeysAreIndependent(t *testing.T) {
	l, _ := newTestLimiter(1, 1)
	if !l.Allow("a") || !l.Allow("b") {
		t.Fatal("different keys must not share a bucket")
	}
}

func TestForgetDropsIdleBuckets(t *testing.T) {
	l, clock := newTestLimiter(1, 1)
	l.Allow("a")
	clock.Advance(time.Minute)
	l.Allow("b")
	if n := l.Forget(30 * time.Second); n != 1 {
		t.Fatalf("Forget dropped %d buckets, want 1", n)
	}
}

func TestMiddlewareReturns429(t *testing.T) {
	l, _ := newTestLimiter(1, 1)
	h := Middleware(l, ClientIP, http.HandlerFunc(func(http.ResponseWriter, *http.Request) {}))
	var codes []int
	for i := 0; i < 2; i++ {
		rec := httptest.NewRecorder()
		req := httptest.NewRequest(http.MethodGet, "/", nil)
		req.RemoteAddr = "203.0.113.7:51234"
		h.ServeHTTP(rec, req)
		codes = append(codes, rec.Code)
	}
	if codes[0] != http.StatusOK || codes[1] != http.StatusTooManyRequests {
		t.Fatalf("got status codes %v, want [200 429]", codes)
	}
}
