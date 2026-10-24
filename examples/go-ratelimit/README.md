# ratelimit

Token bucket rate limiting for `net/http` services.

```go
limiter := ratelimit.New(5, 10) // 5 requests per second, bursts of 10
http.ListenAndServe(":8080", ratelimit.Middleware(limiter, ratelimit.ClientIP, mux))
```

Run `go test ./...` before sending a change.
