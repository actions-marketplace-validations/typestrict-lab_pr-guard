//! The change, read from the local git: a commit range (the merge base with the default branch up to `HEAD` by default)
//! or the staged changes. Shells out to `git` so it reads the repository exactly as the user's git does.

use std::path::Path;
use std::process::Command;

use anyhow::Context;

use crate::ci::Ci;

/// What to compare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The index against `HEAD` (pre-commit).
    Staged,
    /// `base` is already the merge base, so the diff is only what the branch added.
    Range { base: String, head: String },
}

pub fn git(dir: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().context("running git (is it installed?)")?;
    anyhow::ensure!(out.status.success(), "git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
    // As the lab mined it: invalid UTF-8 becomes U+FFFD instead of an error.
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn resolves(dir: &Path, rev: &str) -> bool {
    git(dir, &["rev-parse", "--verify", "--quiet", &format!("{rev}^{{commit}}")]).is_ok()
}

/// `origin/HEAD` when the clone knows it, else `main`, else `master` (local first, then the remote's).
pub fn default_branch(dir: &Path) -> Option<String> {
    if let Ok(r) = git(dir, &["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"]) {
        let r = r.trim();
        if !r.is_empty() && resolves(dir, r) {
            return Some(r.to_string());
        }
    }
    ["main", "master", "origin/main", "origin/master"].into_iter().find(|b| resolves(dir, b)).map(String::from)
}

/// The commit range to review: `--base` and `--head` when given, else what the CI says the PR targets, else the default
/// branch; the base is then moved to the merge base so commits that landed on it after the branch point stay out.
pub fn range(dir: &Path, base: Option<&str>, head: Option<&str>, ci: Option<&Ci>) -> anyhow::Result<Source> {
    git(dir, &["rev-parse", "--git-dir"]).context("not a git repository (pass --diff <file> or --diff - to read a diff)")?;
    let head = head.unwrap_or("HEAD");
    anyhow::ensure!(resolves(dir, head), "--head {head} is not a commit");
    let base = match base {
        Some(b) => {
            anyhow::ensure!(resolves(dir, b), "--base {b} is not a commit");
            b.to_string()
        }
        None => {
            // The CI's branch name is the remote's: a local branch of the same name may be stale or missing.
            let mut from_ci = ci.into_iter().flat_map(|c| {
                c.base_sha.iter().cloned().chain(c.base_ref.iter().flat_map(|r| [format!("origin/{r}"), r.clone()]))
            });
            from_ci.find(|r| resolves(dir, r)).or_else(|| default_branch(dir)).context(
                "no base to compare with: no origin/HEAD, main or master (pass --base <ref>, or fetch the base branch in CI)",
            )?
        }
    };
    let merge_base = git(dir, &["merge-base", &base, head])
        .with_context(|| format!("no common ancestor of {base} and {head} (a shallow clone? fetch with depth 0)"))?;
    Ok(Source::Range { base: merge_base.trim().to_string(), head: head.to_string() })
}

pub fn diff(dir: &Path, source: &Source) -> anyhow::Result<String> {
    // Fixed prefixes and no external diff: the parser and the model expect plain `git diff` output whatever the config.
    let mut args = vec!["diff", "--no-color", "--no-ext-diff", "--src-prefix=a/", "--dst-prefix=b/"];
    match source {
        Source::Staged => args.push("--cached"),
        Source::Range { base, head } => args.extend([base.as_str(), head.as_str()]),
    }
    git(dir, &args)
}

/// The commit messages of the range (subject and body), newest first, separated by blank lines: the description when
/// the PR has none at hand. TS-PRGuard learned the change type and API breaks from commit messages, so they serve.
pub fn messages(dir: &Path, source: &Source) -> anyhow::Result<Option<String>> {
    let Source::Range { base, head } = source else { return Ok(None) };
    let log = git(dir, &["log", "--no-merges", "--format=%B%x00", &format!("{base}..{head}")])?;
    let msgs: Vec<&str> = log.split('\0').map(str::trim).filter(|m| !m.is_empty()).collect();
    Ok((!msgs.is_empty()).then(|| msgs.join("\n\n")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A throwaway repository, isolated from the user's git config (signing, hooks, default branch).
    struct Repo(PathBuf);

    impl Repo {
        fn new(name: &str, branch: &str) -> Repo {
            let dir = std::env::temp_dir().join(format!("pr-guard-git-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let r = Repo(dir);
            r.run(&["init", "-q", "-b", branch]);
            r
        }

        fn run(&self, args: &[&str]) {
            let out = Command::new("git")
                .arg("-C")
                .arg(&self.0)
                .args(args)
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@example.com")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@example.com")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
        }

        fn commit(&self, file: &str, text: &str, msg: &str) {
            std::fs::write(self.0.join(file), text).unwrap();
            self.run(&["add", file]);
            self.run(&["commit", "-q", "-m", msg]);
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_default_range_is_the_branch_since_its_merge_base_with_main() {
        let r = Repo::new("range", "main");
        r.commit("a.txt", "a\n", "Initial");
        r.run(&["checkout", "-q", "-b", "feature"]);
        r.commit("b.py", "print(1)\n", "Add b\n\nWith a body.");
        r.commit("c.py", "print(2)\n", "Fix c");
        r.run(&["checkout", "-q", "main"]);
        r.commit("d.txt", "later on main\n", "Unrelated");
        r.run(&["checkout", "-q", "feature"]);

        assert_eq!(default_branch(&r.0).as_deref(), Some("main"));
        let src = range(&r.0, None, None, None).unwrap();
        let d = diff(&r.0, &src).unwrap();
        assert!(d.contains("+++ b/b.py") && d.contains("+++ b/c.py") && !d.contains("d.txt"), "{d}");
        assert_eq!(messages(&r.0, &src).unwrap().as_deref(), Some("Fix c\n\nAdd b\n\nWith a body."));

        // --base and --head pick the range; the CI's target branch wins over the default one.
        let only_c = range(&r.0, Some("feature~1"), Some("feature"), None).unwrap();
        assert!(!diff(&r.0, &only_c).unwrap().contains("b.py"));
        let ci = crate::ci::detect(&crate::ci::Env::from_pairs(&[("GITLAB_CI", "true"), ("CI_MERGE_REQUEST_TARGET_BRANCH_NAME", "feature")])).unwrap();
        assert_eq!(messages(&r.0, &range(&r.0, None, None, Some(&ci)).unwrap()).unwrap(), None);
        assert!(range(&r.0, Some("nope"), None, None).is_err());
    }

    #[test]
    fn staged_reads_the_index_and_master_is_the_fallback() {
        let r = Repo::new("staged", "master");
        r.commit("a.txt", "a\n", "Initial");
        assert_eq!(default_branch(&r.0).as_deref(), Some("master"));
        std::fs::write(r.0.join("a.txt"), "a\nstaged\n").unwrap();
        r.run(&["add", "a.txt"]);
        std::fs::write(r.0.join("a.txt"), "a\nstaged\nnot staged\n").unwrap();
        let d = diff(&r.0, &Source::Staged).unwrap();
        assert!(d.contains("+staged") && !d.contains("not staged"), "{d}");
        assert_eq!(messages(&r.0, &Source::Staged).unwrap(), None);

        let none = Repo::new("nobase", "trunk");
        none.commit("a.txt", "a\n", "Initial");
        assert!(range(&none.0, None, None, None).unwrap_err().to_string().contains("no base"));
    }
}
