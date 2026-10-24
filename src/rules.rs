//! Deterministic checks that need no model: sensitive paths, secrets, dependency changes, size and destructive SQL.

use std::sync::LazyLock;

use globset::{Glob, GlobSetBuilder};
use regex::Regex;

use crate::diff::FileChange;
use crate::policy::Rules;
use crate::verdict::Finding;

static SECRETS: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    [
        ("AWS access key", r"\b(AKIA|ASIA)[0-9A-Z]{16}\b"),
        ("GitHub token", r"\b(gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{60,})\b"),
        ("private key", r"-----BEGIN (RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY"),
        ("Slack token", r"\bxox[abprs]-[A-Za-z0-9-]{10,}"),
        ("Stripe live key", r"\b(sk|rk)_live_[A-Za-z0-9]{20,}"),
        ("Google API key", r"\bAIza[0-9A-Za-z_-]{35}\b"),
        ("OpenAI-style key", r"\bsk-(proj-)?[A-Za-z0-9_-]{32,}"),
    ]
    .into_iter()
    .map(|(n, r)| (n, Regex::new(r).unwrap()))
    .collect()
});

static SQL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(drop\s+(table|database|schema|column)|truncate\s+(table\s+)?\w|alter\s+table\s+\S+\s+drop\b|delete\s+from\s+[\w.\x22`]+\s*;)").unwrap()
});

const MANIFESTS: &[&str] = &[
    "package.json", "package-lock.json", "pnpm-lock.yaml", "yarn.lock", "bun.lockb", "go.mod", "go.sum", "Cargo.toml", "Cargo.lock",
    "pyproject.toml", "poetry.lock", "uv.lock", "Pipfile", "Pipfile.lock", "pubspec.yaml", "pubspec.lock", "build.gradle",
    "build.gradle.kts", "libs.versions.toml", "pom.xml", "Podfile", "Podfile.lock", "Package.swift", "Package.resolved", "Cartfile",
    "composer.json", "composer.lock", "Gemfile", "Gemfile.lock",
];

fn is_lockfile(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.ends_with(".lock") || name.ends_with("-lock.yaml") || name.ends_with("-lock.json") || matches!(name, "go.sum" | "Package.resolved" | "bun.lockb")
}

pub fn check(rules: &Rules, files: &[FileChange]) -> anyhow::Result<Vec<Finding>> {
    let mut out = Vec::new();
    if let Some(r) = &rules.sensitive_paths {
        let mut b = GlobSetBuilder::new();
        for p in &r.paths {
            b.add(Glob::new(p)?);
        }
        let set = b.build()?;
        for f in files.iter().filter(|f| set.is_match(&f.path)) {
            out.push(Finding::at(r.level, "sensitive_paths", format!("Changes a sensitive path: {}", f.path), &f.path, None));
        }
    }
    if let Some(r) = &rules.secrets {
        for f in files {
            for (line, text) in &f.added {
                if let Some((name, _)) = SECRETS.iter().find(|(_, re)| re.is_match(text)) {
                    out.push(Finding::at(r.level, "secrets", format!("Looks like a {name} committed in the code"), &f.path, Some(*line)));
                }
            }
        }
    }
    if let Some(r) = &rules.dependencies {
        for f in files {
            let name = f.path.rsplit('/').next().unwrap_or(&f.path);
            if MANIFESTS.contains(&name) || name.starts_with("requirements") && name.ends_with(".txt") || name.ends_with(".podspec") {
                out.push(Finding::at(r.level, "dependencies", format!("Changes dependencies: {}", f.path), &f.path, None));
            }
        }
    }
    if let Some(r) = &rules.size {
        // Lockfiles are generated: they say nothing about how much a person has to read.
        let total: u32 = files.iter().filter(|f| !is_lockfile(&f.path)).map(FileChange::changed_lines).sum();
        if total > r.max_lines {
            out.push(Finding::new(r.level, "size", format!("{total} changed lines, above the {} the policy allows", r.max_lines)));
        }
    }
    if let Some(r) = &rules.destructive_sql {
        let sqlish = |p: &str| p.ends_with(".sql") || p.contains("migration") || p.contains("migrate");
        for f in files.iter().filter(|f| sqlish(&f.path)) {
            for (line, text) in &f.added {
                if let Some(m) = SQL.find(text) {
                    out.push(Finding::at(r.level, "destructive_sql", format!("Destructive SQL: `{}`", m.as_str().trim()), &f.path, Some(*line)));
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::Level;

    fn file(path: &str, added: &[&str]) -> FileChange {
        FileChange { path: path.into(), deleted: false, added: added.iter().enumerate().map(|(i, t)| (i as u32 + 1, t.to_string())).collect(), removed: 0 }
    }

    #[test]
    fn each_rule_fires_on_its_case_and_only_there() {
        let files = [
            file(".github/workflows/ci.yml", &["run: echo"]),
            file("src/config.ts", &["const k = \"AKIAABCDEFGHIJKLMNOP\""]),
            file("web/package.json", &["\"left-pad\": \"1.0.0\""]),
            file("db/migrations/0003.sql", &["ALTER TABLE users DROP COLUMN email;", "DELETE FROM sessions WHERE id = 1;", "DELETE FROM sessions;"]),
            file("src/lib.rs", &["let drop_table = 1;"]),
        ];
        let got = check(&Rules::default(), &files).unwrap();
        let rules: Vec<_> = got.iter().map(|f| (f.rule.as_str(), f.path.as_deref().unwrap(), f.line)).collect();
        assert_eq!(rules, vec![
            ("sensitive_paths", ".github/workflows/ci.yml", None),
            ("sensitive_paths", "db/migrations/0003.sql", None),
            ("secrets", "src/config.ts", Some(1)),
            ("dependencies", "web/package.json", None),
            ("destructive_sql", "db/migrations/0003.sql", Some(1)),
            ("destructive_sql", "db/migrations/0003.sql", Some(3)),
        ]);
        assert_eq!(got.iter().find(|f| f.rule == "secrets").unwrap().level, Level::Danger);
    }

    #[test]
    fn size_counts_added_and_removed_lines_and_null_turns_a_rule_off() {
        let big = file("a.py", &["x"; 900]);
        assert_eq!(check(&Rules::default(), std::slice::from_ref(&big)).unwrap()[0].rule, "size");
        let lock = file("web/pnpm-lock.yaml", &["x"; 900]);
        assert!(check(&Rules::default(), &[lock]).unwrap().iter().all(|f| f.rule != "size"));
        let off = Rules { size: None, ..Rules::default() };
        assert!(check(&off, &[big]).unwrap().is_empty());
    }
}
