//! The repository's policy (`.github/pr-guard.yml`): how much each answer weighs, the minimums, the reviewer bands and
//! the deterministic rules. Every field has a default, so an empty file is a valid policy.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Reports on every pull request but never blocks, labels for merge or fails the check: for calibrating a new policy.
    Shadow,
    #[default]
    Enforce,
}

/// The questions TS-PRGuard answers. `ChangeType` only labels; the others carry a score where 1 is the safe side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Question {
    DescriptionMatches,
    Complexity,
    Vulnerability,
    TestsCover,
    Breaking,
    ResolvesIssue,
    Focused,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    #[serde(default = "one")]
    pub weight: f64,
    /// Below this the answer is a warning (at least one reviewer).
    #[serde(default)]
    pub warn_below: Option<f64>,
    /// Below this the answer is a danger (two reviewers; blocks when `block_on: danger`).
    #[serde(default)]
    pub min: Option<f64>,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reviewers {
    /// Combined score at or above this: no reviewer needed (auto-merge when allowed).
    pub none_at: f64,
    /// At or above this: one reviewer; below: two.
    pub one_at: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathRule {
    pub paths: Vec<String>,
    pub level: Level,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelRule {
    pub level: Level,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeRule {
    pub max_lines: u32,
    pub level: Level,
}

/// Each rule can be turned off with `null`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Rules {
    pub sensitive_paths: Option<PathRule>,
    pub secrets: Option<LevelRule>,
    pub dependencies: Option<LevelRule>,
    pub size: Option<SizeRule>,
    pub destructive_sql: Option<LevelRule>,
}

impl Default for Rules {
    fn default() -> Self {
        let paths = [".github/workflows/**", "**/auth/**", "**/*auth*.*", "**/migrations/**", "**/Dockerfile", "**/*.tf", "**/.env*"];
        Rules {
            sensitive_paths: Some(PathRule { paths: paths.map(String::from).to_vec(), level: Level::Warning }),
            secrets: Some(LevelRule { level: Level::Danger }),
            dependencies: Some(LevelRule { level: Level::Info }),
            size: Some(SizeRule { max_lines: 800, level: Level::Warning }),
            destructive_sql: Some(LevelRule { level: Level::Danger }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Policy {
    pub mode: Mode,
    pub skip_label: String,
    pub questions: BTreeMap<Question, Gate>,
    pub reviewers: Reviewers,
    /// Lets a pull request that needs no reviewer be labeled for auto-merge.
    pub auto_merge: bool,
    /// Findings at this level or above fail the check; `null` never fails it.
    pub block_on: Option<Level>,
    pub rules: Rules,
    /// Languages the model was evaluated on; code in any other language is "not evaluated" and never auto-merges.
    pub languages: Vec<String>,
}

impl Default for Policy {
    fn default() -> Self {
        let g = |weight, warn_below, min| Gate { weight, warn_below, min };
        Policy {
            mode: Mode::Enforce,
            skip_label: "pr-guard:skip".into(),
            questions: BTreeMap::from([
                (Question::DescriptionMatches, g(2.0, Some(0.6), Some(0.3))),
                (Question::Complexity, g(1.0, Some(0.25), None)),
                (Question::Vulnerability, g(3.0, Some(0.7), Some(0.4))),
                (Question::TestsCover, g(2.0, Some(0.5), None)),
                (Question::Breaking, g(2.0, Some(0.5), None)),
                (Question::ResolvesIssue, g(1.0, Some(0.5), None)),
                (Question::Focused, g(1.0, Some(0.5), None)),
            ]),
            reviewers: Reviewers { none_at: 0.85, one_at: 0.6 },
            auto_merge: false,
            block_on: Some(Level::Danger),
            rules: Rules::default(),
            languages: crate::lang::SUPPORTED.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Policy {
    pub fn from_yaml(s: &str) -> anyhow::Result<Policy> {
        if s.trim().is_empty() {
            return Ok(Policy::default());
        }
        let p: Policy = serde_yaml::from_str(s)?;
        anyhow::ensure!(p.reviewers.one_at <= p.reviewers.none_at, "reviewers.one_at must not exceed reviewers.none_at");
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_is_the_default_and_partial_files_keep_the_other_defaults() {
        assert_eq!(Policy::from_yaml("").unwrap(), Policy::default());
        let p = Policy::from_yaml("mode: shadow\nrules:\n  size: null\n").unwrap();
        assert_eq!(p.mode, Mode::Shadow);
        assert!(p.rules.size.is_none() && p.rules.secrets.is_some());
        assert_eq!(p.reviewers, Policy::default().reviewers);
    }

    #[test]
    fn typos_and_inverted_bands_are_errors() {
        assert!(Policy::from_yaml("reviewrs: {}").is_err());
        assert!(Policy::from_yaml("reviewers: {none_at: 0.5, one_at: 0.8}").is_err());
    }
}
