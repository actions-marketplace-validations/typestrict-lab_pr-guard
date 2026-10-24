Improve the middleware log line

The old message (`ratelimit: <err>`) did not say which request failed. Now it logs the method,
path and remote address so we can find the client.
