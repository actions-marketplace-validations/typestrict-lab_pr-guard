Key requests by X-Forwarded-For behind the load balancer

In production every request comes from the load balancer's address, so all users shared a
single bucket and got 429s together. ClientIP now uses the first address in X-Forwarded-For
when the header is present and falls back to RemoteAddr otherwise.
