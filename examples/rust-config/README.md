# svc-config

Loads service settings from `<dir>/<profile>.toml`.

```toml
listen_addr = "0.0.0.0:8080"   # default 127.0.0.1:8080
workers = 8                    # default 4
request_timeout = "10s"        # ms, s or m; default 30s
database_url = "postgres://db/app"
```

```rust
let cfg = svc_config::load(Path::new("/etc/myservice"), "prod")?;
```
