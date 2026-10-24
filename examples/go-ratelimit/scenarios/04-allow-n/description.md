Add AllowN for weighted requests

Bulk export endpoints should cost more than a single request. AllowN consumes n tokens at once,
so `/export` can call `AllowN(key, 10)`.
