Accept hours in duration settings

Cache TTLs are naturally expressed in hours (`cache_ttl = "6h"` is coming in the next PR), so the
duration parser now understands `h` as well.
