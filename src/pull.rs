//! The model package from Hugging Face, at the revision this release was tested with. The manifest is pinned by its
//! SHA-256 here and lists the SHA-256 of the tokenizer and the ONNX, so every byte is checked before it is used.
//! Each file is written next to its place as `.part` and renamed once verified: an interrupted download never leaves a
//! package that looks complete.

use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

pub const REPO: &str = "typestrict/TS-PRGuard";
/// The commit of the repo the scoreboard in examples/ ran against (TS-PRGuard v1, published 04/11/2026).
pub const REVISION: &str = "2179dee21c568da8c75304c0fd6dd3c97a079b3c";
const MANIFEST_SHA256: &str = "c33899c0ba2de2d7929abfb97e7dd72b2b16e9f19e8895c9e4c0a04055cc03f8";
const FILES: [&str; 2] = ["tokenizer.json", "onnx/model_8bit.onnx"];

/// `$PR_GUARD_HF_ENDPOINT` points at a mirror (an internal proxy where the runners have no internet).
fn url(file: &str) -> String {
    let base = std::env::var("PR_GUARD_HF_ENDPOINT").unwrap_or_else(|_| "https://huggingface.co".into());
    format!("{}/{REPO}/resolve/{REVISION}/{file}", base.trim_end_matches('/'))
}

fn hex(d: impl AsRef<[u8]>) -> String {
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether `dir` holds this release's package: the pinned manifest and the files it lists. The large files are not
/// hashed again on every run (that costs more than the answer); `pull` checked them when it wrote them.
pub fn is_current(dir: &Path) -> bool {
    std::fs::read(dir.join("decision_ai.json")).is_ok_and(|m| hex(Sha256::digest(m)) == MANIFEST_SHA256) && FILES.iter().all(|f| dir.join(f).is_file())
}

/// Downloads the package into `dir` unless it is already there; `quiet` keeps stderr for errors only.
pub fn pull(dir: &Path, quiet: bool) -> Result<()> {
    if is_current(dir) {
        return Ok(());
    }
    let say = |m: String| {
        if !quiet {
            eprintln!("pr-guard: {m}");
        }
    };
    say(format!("downloading {REPO}@{} into {}", &REVISION[..7], dir.display()));
    let manifest = fetch(&url("decision_ai.json"), dir, "decision_ai.json", MANIFEST_SHA256)?;
    let sums: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest)?)?;
    for f in FILES {
        let want = sums["sha256"][f].as_str().with_context(|| format!("the manifest has no sha256 for {f}"))?;
        if dir.join(f).is_file() && hex(Sha256::digest(std::fs::read(dir.join(f))?)) == want {
            continue;
        }
        let t0 = std::time::Instant::now();
        let p = fetch(&url(f), dir, f, want)?;
        say(format!("{f}: {} MB in {:.0?}", std::fs::metadata(&p)?.len() / 1_000_000, t0.elapsed()));
    }
    // The manifest last: a package with it in place is complete.
    std::fs::rename(&manifest, dir.join("decision_ai.json"))?;
    Ok(())
}

/// One file, streamed to `<dir>/<name>.part` while hashing, renamed to its place when the hash matches. The manifest
/// stays `.part` for `pull` to move last.
fn fetch(url: &str, dir: &Path, name: &str, sha256: &str) -> Result<std::path::PathBuf> {
    let dest = dir.join(name);
    std::fs::create_dir_all(dest.parent().unwrap_or(dir))?;
    let part = dest.with_file_name(format!("{}.part", dest.file_name().unwrap_or_default().to_string_lossy()));
    let res = ureq::get(url).call().with_context(|| format!("downloading {url}"))?;
    let mut body = res.into_body().into_reader();
    let mut out = std::io::BufWriter::new(std::fs::File::create(&part)?);
    let (mut hash, mut buf) = (Sha256::new(), vec![0u8; 1 << 16]);
    loop {
        let n = body.read(&mut buf).with_context(|| format!("downloading {url}"))?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        out.write_all(&buf[..n])?;
    }
    out.flush()?;
    drop(out);
    let got = hex(hash.finalize());
    if got != sha256 {
        let _ = std::fs::remove_file(&part);
        bail!("{name} from {url} has sha256 {got}, expected {sha256}");
    }
    if name == "decision_ai.json" {
        return Ok(part);
    }
    std::fs::rename(&part, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_package_is_current_only_with_the_pinned_manifest_and_its_files() {
        let dir = std::env::temp_dir().join(format!("pr-guard-pull-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("onnx")).unwrap();
        assert!(!is_current(&dir));
        std::fs::write(dir.join("decision_ai.json"), b"{}").unwrap();
        std::fs::write(dir.join("tokenizer.json"), b"{}").unwrap();
        std::fs::write(dir.join("onnx/model_8bit.onnx"), b"x").unwrap();
        // Another manifest (an older model in the cache): pulled again.
        assert!(!is_current(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_url_names_the_pinned_revision() {
        assert_eq!(url("tokenizer.json"), format!("https://huggingface.co/{REPO}/resolve/{REVISION}/tokenizer.json"));
    }
}
