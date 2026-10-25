//! What the changed files alone settle, so the model is not asked: the change type when every file is documentation,
//! tests, build or CI, or only moves whitespace; and whether the tests question applies at all (no production code
//! changed beyond whitespace means it does not). The model answers the rest, which takes reading the code.

use std::sync::LazyLock;

use regex::Regex;

use crate::model;

pub const DOCUMENTATION: &str = "Documentation only";
pub const TESTS: &str = "Tests only";
pub const BUILD: &str = "Build system, dependencies or continuous integration";
pub const FORMATTING: &str = "Formatting or style only";

static DOC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.(md|markdown|rst|adoc|txt)$|(^|/)docs?/|(^|/)(LICENSE|CHANGELOG|README)[^/]*$").unwrap());
static BUILD_FILE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(^|/)(package\.json|package-lock\.json|yarn\.lock|pnpm-lock\.yaml|Cargo\.(toml|lock)|go\.(mod|sum)|pyproject\.toml",
        r"|setup\.(py|cfg)|requirements[^/]*\.txt|Pipfile[^/]*|poetry\.lock|Gemfile[^/]*|[^/]*\.gemspec|pubspec\.(yaml|lock)",
        r"|build\.gradle[^/]*|settings\.gradle[^/]*|gradle\.properties|gradlew(\.bat)?|pom\.xml|Podfile[^/]*|Package\.(swift|resolved)",
        r"|Makefile|CMakeLists\.txt|Dockerfile[^/]*|docker-compose[^/]*|\.gitlab-ci\.yml|Jenkinsfile|azure-pipelines\.yml|bitrise\.yml",
        r"|\.travis\.yml)$|(^|/)(gradle|\.circleci)/|(^|/)\.github/workflows/",
    ))
    .unwrap()
});
/// Formatter and linter settings, which a formatting change often adds next to the reformatted code.
static FORMAT_CONFIG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(^|/)(\.prettierrc[^/]*|\.prettierignore|prettier\.config\.[^/]+|\.editorconfig|\.rubocop[^/]*\.yml|\.?rustfmt\.toml",
        r"|\.clang-format|\.eslintrc[^/]*|eslint\.config\.[^/]+|biome\.jsonc?|\.swiftformat|\.swiftlint\.yml|\.golangci\.ya?ml",
        r"|\.flake8|\.isort\.cfg|\.scalafmt\.conf|\.stylelintrc[^/]*|analysis_options\.yaml)$",
    ))
    .unwrap()
});

pub fn is_doc(f: &str) -> bool {
    DOC.is_match(f) && !is_build(f)
}

pub fn is_build(f: &str) -> bool {
    BUILD_FILE.is_match(f)
}

/// One file of a unified diff: its path, how many hunks it has and whether each one only moves whitespace.
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub path: String,
    pub hunks: usize,
    pub whitespace_only: bool,
}

impl File {
    /// Has hunks and every one of them only moves whitespace (a pure rename or a binary file has none).
    fn reformatted(&self) -> bool {
        self.hunks > 0 && self.whitespace_only
    }
}

/// The files of a unified diff. A hunk only moves whitespace when its removed lines and its added lines, each joined
/// in order with every whitespace character dropped, are the same text (so rewrapped lines count too). The hunk
/// header's line counts tell where a hunk ends, so a removed line that starts with `-- ` is not a file header.
pub fn files(diff: &str) -> Vec<File> {
    let mut files: Vec<File> = Vec::new();
    let mut old_path = String::new();
    // Lines left in the open hunk (old side, new side) and its removed and added text without whitespace.
    let (mut old_left, mut new_left) = (0u32, 0u32);
    let (mut removed, mut added) = (String::new(), String::new());
    let squeeze = |s: &str, out: &mut String| out.extend(s.chars().filter(|c| !c.is_whitespace()));
    for l in diff.lines() {
        if old_left > 0 || new_left > 0 {
            if let Some(t) = l.strip_prefix('-') {
                squeeze(t, &mut removed);
                old_left = old_left.saturating_sub(1);
            } else if let Some(t) = l.strip_prefix('+') {
                squeeze(t, &mut added);
                new_left = new_left.saturating_sub(1);
            } else if l.starts_with(' ') || l.is_empty() {
                old_left = old_left.saturating_sub(1);
                new_left = new_left.saturating_sub(1);
            } else if !l.starts_with('\\') {
                // Counts that do not match the lines: close the hunk here and read the line as a header.
                (old_left, new_left) = (0, 0);
            }
            if old_left > 0 || new_left > 0 || l.starts_with('\\') {
                continue;
            }
            if let Some(f) = files.last_mut() {
                f.hunks += 1;
                f.whitespace_only &= same_but_style(&f.path, &removed, &added);
            }
            if l.starts_with(['-', '+', ' ']) || l.is_empty() {
                continue;
            }
        }
        if let Some(p) = l.strip_prefix("--- ") {
            old_path = strip_side(p, "a/");
        } else if let Some(p) = l.strip_prefix("+++ ") {
            let new = strip_side(p, "b/");
            let path = if new == "/dev/null" { old_path.clone() } else { new };
            files.push(File { path, hunks: 0, whitespace_only: true });
        } else if let Some(h) = l.strip_prefix("@@ ") {
            // @@ -a,b +c,d @@: b and d lines (1 when the count is left out).
            let count = |side: char| {
                h.split_whitespace().find_map(|t| t.strip_prefix(side)).map_or(1, |t| t.split(',').nth(1).and_then(|n| n.parse().ok()).unwrap_or(1))
            };
            (old_left, new_left) = (count('-'), count('+'));
            removed.clear();
            added.clear();
            if old_left == 0
                && new_left == 0
                && let Some(f) = files.last_mut()
            {
                f.hunks += 1;
            }
        }
    }
    // A diff cut in the middle of a hunk: judge what was read.
    if (old_left > 0 || new_left > 0)
        && let Some(f) = files.last_mut()
    {
        f.hunks += 1;
        f.whitespace_only &= same_but_style(&f.path, &removed, &added);
    }
    files
}

/// Removed and added text (whitespace already dropped) equal up to style: a comma right before a closing bracket is a
/// trailing comma a formatter adds or drops (rustfmt, prettier), except in Python, where `(x,)` is a tuple and `(x)` is not.
fn same_but_style(path: &str, removed: &str, added: &str) -> bool {
    if removed == added {
        return true;
    }
    if path.ends_with(".py") || path.ends_with(".pyi") {
        return false;
    }
    let drop = |s: &str| {
        let c: Vec<char> = s.chars().collect();
        c.iter().enumerate().filter(|&(i, &ch)| !(ch == ',' && matches!(c.get(i + 1), Some(')' | ']' | '}')))).map(|(_, &ch)| ch).collect::<String>()
    };
    drop(removed) == drop(added)
}

fn strip_side(p: &str, side: &str) -> String {
    let p = p.split('\t').next().unwrap_or(p).trim_end();
    p.strip_prefix(side).unwrap_or(p).to_string()
}

/// The change type when the files settle it, in this order: every file is documentation, every file is a test, every
/// file is build, dependencies or CI, or every file only moves whitespace (formatter settings may come along). `None`
/// for mixed or code changes: the model's question.
pub fn change_type(files: &[File]) -> Option<&'static str> {
    if files.is_empty() {
        return None;
    }
    let all = |pred: &dyn Fn(&File) -> bool| files.iter().all(pred);
    if all(&|f| is_doc(&f.path)) {
        Some(DOCUMENTATION)
    } else if all(&|f| model::in_tests(&f.path)) {
        Some(TESTS)
    } else if all(&|f| is_build(&f.path)) {
        Some(BUILD)
    } else if all(&|f| f.reformatted() || FORMAT_CONFIG.is_match(&f.path)) && files.iter().any(File::reformatted) {
        Some(FORMATTING)
    } else {
        None
    }
}

/// Whether the change touches production code beyond whitespace. When it does not (docs, tests, configuration, build,
/// formatting) the tests question does not apply.
pub fn code_changed(files: &[File]) -> bool {
    files.iter().any(|f| model::is_code(&f.path) && !is_doc(&f.path) && !is_build(&f.path) && !f.whitespace_only)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-hunk diff per (path, removed lines, added lines).
    fn diff(parts: &[(&str, &[&str], &[&str])]) -> String {
        let mut d = String::new();
        for (path, minus, plus) in parts {
            d += &format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1,{} +1,{} @@\n", minus.len() + 1, plus.len() + 1);
            d += " context\n";
            for l in *minus {
                d += &format!("-{l}\n");
            }
            for l in *plus {
                d += &format!("+{l}\n");
            }
        }
        d
    }

    fn settle(parts: &[(&str, &[&str], &[&str])]) -> (Option<&'static str>, bool) {
        let f = files(&diff(parts));
        assert_eq!(f.len(), parts.len());
        (change_type(&f), code_changed(&f))
    }

    #[test]
    fn documentation_only() {
        let got = settle(&[("README.md", &["old"], &["new"]), ("docs/guide/conf.py", &[], &["x = 1"]), ("LICENSE", &[], &["MIT"])]);
        assert_eq!(got, (Some(DOCUMENTATION), false));
        // requirements.txt is a dependency file, not text.
        assert_eq!(settle(&[("requirements-dev.txt", &["a==1"], &["a==2"])]), (Some(BUILD), false));
    }

    #[test]
    fn tests_only() {
        let got = settle(&[("tests/test_api.py", &["assert a"], &["assert b"]), ("src/cart.test.ts", &[], &["it()"])]);
        assert_eq!(got, (Some(TESTS), false));
    }

    #[test]
    fn build_dependencies_or_ci_only() {
        let got = settle(&[
            ("Gemfile", &["gem 'sinatra', '3.0'"], &["gem 'sinatra', '4.0'"]),
            ("Gemfile.lock", &["x"], &["y"]),
            (".github/workflows/ci.yml", &[], &["run: make"]),
            ("app/build.gradle.kts", &[], &["implementation(x)"]),
            ("setup.py", &["version='1'"], &["version='2'"]),
        ]);
        assert_eq!(got, (Some(BUILD), false));
    }

    #[test]
    fn whitespace_only_including_rewrapped_lines_and_formatter_settings() {
        let got = settle(&[
            ("src/a.rs", &["let x = f(a, b);"], &["let x = f(", "    a,", "    b", ");"]),
            ("src/b.ts", &["\tif (x) {"], &["  if (x) {"]),
            (".prettierrc", &[], &["{}"]),
        ]);
        assert_eq!(got, (Some(FORMATTING), false));
        // Trailing commas before a closing bracket are style, as rustfmt and prettier write them.
        let got = settle(&[
            ("src/loader.rs", &["Io { path: PathBuf, source: std::io::Error },"], &["Io {", "    path: PathBuf,", "    source: std::io::Error,", "},"]),
            ("src/cart.ts", &["f(a, [b, c], { d: 1 });"], &["f(", "  a,", "  [b, c,],", "  { d: 1, },", ");"]),
        ]);
        assert_eq!(got, (Some(FORMATTING), false));
        // A comma anywhere else still counts, and in Python `(x,)` is a tuple, not `(x)`.
        assert_eq!(settle(&[("src/a.ts", &["f(a b)"], &["f(a, b)"])]), (None, true));
        assert_eq!(settle(&[("src/a.py", &["t = (x)"], &["t = (x,)"])]), (None, true));
        assert_eq!(settle(&[("stubs/a.pyi", &["def f(a): ..."], &["def f(a,): ..."])]), (None, false));
        // Settings alone are not a formatting change.
        assert_eq!(settle(&[(".prettierrc", &[], &["{}"])]).0, None);
        // Same characters in another order is a change.
        assert_eq!(settle(&[("src/a.rs", &["f(a, b)"], &["f(b, a)"])]), (None, true));
    }

    #[test]
    fn mixed_or_code_changes_go_to_the_model() {
        assert_eq!(settle(&[("src/a.go", &["return 1"], &["return 2"])]), (None, true));
        assert_eq!(settle(&[("src/a.go", &["return 1"], &["return 2"]), ("a_test.go", &[], &["t()"])]), (None, true));
        assert_eq!(settle(&[("Gemfile", &["a"], &["b"]), ("app.rb", &["x"], &["y"])]), (None, true));
        assert_eq!(settle(&[("README.md", &["a"], &["b"]), ("src/a.kt", &[], &["/** Docs. */"])]), (None, true));
    }

    #[test]
    fn no_code_changed_beyond_whitespace_skips_the_tests_question() {
        // Configuration and docs together: no rule type, but no code either.
        assert_eq!(settle(&[("config/app.yml", &["a: 1"], &["a: 2"]), ("README.md", &[], &["x"])]), (None, false));
        // Code reformatted next to a real test change: still no production code changed.
        assert_eq!(settle(&[("src/a.py", &["x=1"], &["x = 1"]), ("tests/test_a.py", &[], &["assert x"])]), (None, false));
        assert!(!code_changed(&[]));
    }

    #[test]
    fn hunk_counts_end_a_hunk_so_sql_comments_are_not_headers() {
        let d = "--- a/q.sql\n+++ b/q.sql\n@@ -1,2 +1,1 @@\n--- old comment\n select 1;\n\\ No newline at end of file\n\
                 --- /dev/null\n+++ b/src/new.go\n@@ -0,0 +1 @@\n+package x\n\
                 --- a/gone.md\n+++ /dev/null\n@@ -1 +0,0 @@\n-bye\n";
        let f = files(d);
        assert_eq!(f.iter().map(|f| (f.path.as_str(), f.hunks, f.whitespace_only)).collect::<Vec<_>>(), [
            ("q.sql", 1, false),
            ("src/new.go", 1, false),
            ("gone.md", 1, false),
        ]);
    }
}
