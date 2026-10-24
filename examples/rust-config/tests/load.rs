use std::path::{Path, PathBuf};
use std::time::Duration;

use svc_config::{load, LoadError};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("svc-config-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_profile(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(format!("{name}.toml")), body).unwrap();
}

#[test]
fn applies_defaults() {
    let dir = temp_dir("defaults");
    write_profile(&dir, "dev", "database_url = \"postgres://localhost/dev\"\n");
    let cfg = load(&dir, "dev").unwrap();
    assert_eq!(cfg.listen_addr, "127.0.0.1:8080");
    assert_eq!(cfg.workers, 4);
    assert_eq!(cfg.request_timeout, Duration::from_secs(30));
}

#[test]
fn reads_overrides() {
    let dir = temp_dir("overrides");
    write_profile(
        &dir,
        "prod",
        "listen_addr = \"0.0.0.0:80\"\nworkers = 16\nrequest_timeout = \"500ms\"\ndatabase_url = \"postgres://db/prod\"\n",
    );
    let cfg = load(&dir, "prod").unwrap();
    assert_eq!(cfg.listen_addr, "0.0.0.0:80");
    assert_eq!(cfg.workers, 16);
    assert_eq!(cfg.request_timeout, Duration::from_millis(500));
}

#[test]
fn rejects_profile_outside_dir() {
    let dir = temp_dir("traversal");
    assert!(matches!(load(&dir, "../secrets"), Err(LoadError::InvalidProfile(_))));
}

#[test]
fn rejects_unknown_keys() {
    let dir = temp_dir("unknown");
    write_profile(&dir, "dev", "database_url = \"x\"\nlisten = \"0.0.0.0:1\"\n");
    assert!(matches!(load(&dir, "dev"), Err(LoadError::Parse { .. })));
}
