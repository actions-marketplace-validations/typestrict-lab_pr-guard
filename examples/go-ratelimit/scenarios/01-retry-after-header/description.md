Send Retry-After on 429 responses

Clients were retrying in a tight loop after a 429. The limiter now knows how long until the next
token, and the middleware puts that (rounded up to whole seconds) in the Retry-After header.
`Allow` keeps its signature; the new `Take` returns the wait as well.
