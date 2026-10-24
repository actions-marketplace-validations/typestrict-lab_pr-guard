//! Where pr-guard is running, from the CI's environment variables: only to find the base, the head, the PR number and the
//! description. No network: the host (where to publish) is another axis and comes from `origin`, not from the CI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The environment as a map, so tests inject one instead of mutating the process's (tests run in parallel).
#[derive(Debug, Clone, Default)]
pub struct Env(HashMap<String, String>);

impl Env {
    pub fn from_process() -> Env {
        Env(std::env::vars().collect())
    }

    pub fn from_pairs(pairs: &[(&str, &str)]) -> Env {
        Env(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    /// Set and not empty: CIs export empty variables for what does not apply (no PR, no target branch).
    pub fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).map(|v| v.trim()).filter(|v| !v.is_empty()).map(String::from)
    }

    fn has(&self, key: &str) -> bool {
        self.get(key).is_some_and(|v| !v.eq_ignore_ascii_case("false"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    GitHub,
    GitLab,
    Bitbucket,
    Jenkins,
    CircleCi,
    Bitrise,
    Buildkite,
    Azure,
}

impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Provider::GitHub => "GitHub Actions",
            Provider::GitLab => "GitLab CI",
            Provider::Bitbucket => "Bitbucket Pipelines",
            Provider::Jenkins => "Jenkins",
            Provider::CircleCi => "CircleCI",
            Provider::Bitrise => "Bitrise",
            Provider::Buildkite => "Buildkite",
            Provider::Azure => "Azure Pipelines",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ci {
    pub provider: Provider,
    /// The branch the PR goes into, without `refs/heads/`.
    pub base_ref: Option<String>,
    /// A commit the CI already computed as the diff base (GitLab), tried before `base_ref`.
    pub base_sha: Option<String>,
    pub head_sha: Option<String>,
    /// PR or MR number; `None` on a push build.
    pub pr: Option<String>,
    /// `owner/name`.
    pub repo: Option<String>,
    /// The PR's title and body when the CI exports them (GitLab, Jenkins); GitHub's are in `event_path`.
    title: Option<String>,
    body: Option<String>,
    event_path: Option<PathBuf>,
}

pub fn detect(env: &Env) -> Option<Ci> {
    let g = |k: &str| env.get(k);
    let ci = |provider| Ci { provider, base_ref: None, base_sha: None, head_sha: None, pr: None, repo: None, title: None, body: None, event_path: None };
    let c = if env.has("GITHUB_ACTIONS") {
        Ci {
            base_ref: g("GITHUB_BASE_REF"),
            head_sha: g("GITHUB_SHA"),
            // refs/pull/123/merge on pull_request events.
            pr: g("GITHUB_REF").and_then(|r| Some(r.strip_prefix("refs/pull/")?.split('/').next()?.to_string())),
            repo: g("GITHUB_REPOSITORY"),
            event_path: g("GITHUB_EVENT_PATH").map(PathBuf::from),
            ..ci(Provider::GitHub)
        }
    } else if env.has("GITLAB_CI") {
        Ci {
            base_ref: g("CI_MERGE_REQUEST_TARGET_BRANCH_NAME"),
            base_sha: g("CI_MERGE_REQUEST_DIFF_BASE_SHA"),
            head_sha: g("CI_COMMIT_SHA"),
            pr: g("CI_MERGE_REQUEST_IID"),
            repo: g("CI_PROJECT_PATH"),
            title: g("CI_MERGE_REQUEST_TITLE"),
            body: g("CI_MERGE_REQUEST_DESCRIPTION"),
            ..ci(Provider::GitLab)
        }
    } else if env.has("BITBUCKET_BUILD_NUMBER") {
        Ci {
            base_ref: g("BITBUCKET_PR_DESTINATION_BRANCH"),
            head_sha: g("BITBUCKET_COMMIT"),
            pr: g("BITBUCKET_PR_ID"),
            repo: g("BITBUCKET_REPO_FULL_NAME"),
            ..ci(Provider::Bitbucket)
        }
    } else if env.has("JENKINS_URL") {
        // Multibranch pipelines set CHANGE_* on pull request builds only.
        Ci {
            base_ref: g("CHANGE_TARGET"),
            head_sha: g("GIT_COMMIT"),
            pr: g("CHANGE_ID"),
            repo: g("GIT_URL").as_deref().and_then(slug),
            title: g("CHANGE_TITLE"),
            ..ci(Provider::Jenkins)
        }
    } else if env.has("CIRCLECI") {
        // CircleCI exports no target branch: the default branch is the base.
        let pr = g("CIRCLE_PR_NUMBER").or_else(|| g("CIRCLE_PULL_REQUEST").and_then(|u| u.rsplit('/').next().map(String::from)));
        let repo = match (g("CIRCLE_PROJECT_USERNAME"), g("CIRCLE_PROJECT_REPONAME")) {
            (Some(o), Some(n)) => Some(format!("{o}/{n}")),
            _ => None,
        };
        Ci { head_sha: g("CIRCLE_SHA1"), pr, repo, ..ci(Provider::CircleCi) }
    } else if env.has("BITRISE_IO") {
        Ci {
            base_ref: g("BITRISEIO_GIT_BRANCH_DEST"),
            head_sha: g("BITRISE_GIT_COMMIT"),
            pr: g("BITRISE_PULL_REQUEST"),
            repo: g("GIT_REPOSITORY_URL").as_deref().and_then(slug),
            ..ci(Provider::Bitrise)
        }
    } else if env.has("BUILDKITE") {
        Ci {
            base_ref: g("BUILDKITE_PULL_REQUEST_BASE_BRANCH"),
            head_sha: g("BUILDKITE_COMMIT"),
            // "false" on push builds.
            pr: g("BUILDKITE_PULL_REQUEST").filter(|p| p != "false"),
            repo: g("BUILDKITE_REPO").as_deref().and_then(slug),
            ..ci(Provider::Buildkite)
        }
    } else if env.has("TF_BUILD") {
        Ci {
            base_ref: g("SYSTEM_PULLREQUEST_TARGETBRANCH"),
            head_sha: g("BUILD_SOURCEVERSION"),
            pr: g("SYSTEM_PULLREQUEST_PULLREQUESTNUMBER").or_else(|| g("SYSTEM_PULLREQUEST_PULLREQUESTID")),
            repo: g("BUILD_REPOSITORY_NAME"),
            ..ci(Provider::Azure)
        }
    } else {
        return None;
    };
    Some(Ci { base_ref: c.base_ref.as_deref().map(|r| r.strip_prefix("refs/heads/").unwrap_or(r).to_string()), ..c })
}

impl Ci {
    /// The PR's title and body ("title\n\nbody"), when the CI makes them available without the network.
    pub fn description(&self) -> Option<String> {
        let (title, body) = match &self.event_path {
            Some(p) => github_event(p)?,
            None => (self.title.clone()?, self.body.clone()),
        };
        Some(match body.filter(|b| !b.trim().is_empty()) {
            Some(b) => format!("{title}\n\n{b}"),
            None => title,
        })
    }
}

/// `pull_request.title` and `.body` from GitHub's event payload; push events have none.
fn github_event(path: &Path) -> Option<(String, Option<String>)> {
    let event: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let pr = event.get("pull_request")?;
    let title = pr.get("title")?.as_str()?.to_string();
    Some((title, pr.get("body").and_then(|b| b.as_str()).map(String::from)))
}

/// `owner/name` from an https or ssh remote URL.
fn slug(url: &str) -> Option<String> {
    let path = url.trim_end_matches('/').trim_end_matches(".git");
    let path = match path.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?.1,
        None => path.split_once(':')?.1,
    };
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    (parts.len() >= 2).then(|| parts[parts.len() - 2..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_ci_variables_is_local() {
        assert_eq!(detect(&Env::from_pairs(&[("HOME", "/home/x"), ("CI", "true")])), None);
        assert_eq!(detect(&Env::from_pairs(&[("GITHUB_ACTIONS", "")])), None);
    }

    #[test]
    fn each_provider_gives_base_head_pr_and_repo() {
        let gh = detect(&Env::from_pairs(&[
            ("GITHUB_ACTIONS", "true"), ("GITHUB_BASE_REF", "main"), ("GITHUB_SHA", "abc"), ("GITHUB_REF", "refs/pull/42/merge"),
            ("GITHUB_REPOSITORY", "acme/web"),
        ]))
        .unwrap();
        assert_eq!((gh.provider, gh.base_ref.as_deref(), gh.head_sha.as_deref(), gh.pr.as_deref(), gh.repo.as_deref()),
            (Provider::GitHub, Some("main"), Some("abc"), Some("42"), Some("acme/web")));
        let push = detect(&Env::from_pairs(&[("GITHUB_ACTIONS", "true"), ("GITHUB_REF", "refs/heads/main"), ("GITHUB_BASE_REF", "")])).unwrap();
        assert_eq!((push.pr, push.base_ref), (None, None));

        let gl = detect(&Env::from_pairs(&[
            ("GITLAB_CI", "true"), ("CI_MERGE_REQUEST_TARGET_BRANCH_NAME", "develop"), ("CI_MERGE_REQUEST_DIFF_BASE_SHA", "b4se"),
            ("CI_COMMIT_SHA", "h3ad"), ("CI_MERGE_REQUEST_IID", "7"), ("CI_PROJECT_PATH", "group/sub/proj"),
            ("CI_MERGE_REQUEST_TITLE", "Fix login"), ("CI_MERGE_REQUEST_DESCRIPTION", "Closes #3"),
        ]))
        .unwrap();
        assert_eq!((gl.base_ref.as_deref(), gl.base_sha.as_deref(), gl.pr.as_deref(), gl.repo.as_deref()),
            (Some("develop"), Some("b4se"), Some("7"), Some("group/sub/proj")));
        assert_eq!(gl.description().as_deref(), Some("Fix login\n\nCloses #3"));

        let bb = detect(&Env::from_pairs(&[
            ("BITBUCKET_BUILD_NUMBER", "12"), ("BITBUCKET_PR_DESTINATION_BRANCH", "main"), ("BITBUCKET_COMMIT", "c0"),
            ("BITBUCKET_PR_ID", "5"), ("BITBUCKET_REPO_FULL_NAME", "team/app"),
        ]))
        .unwrap();
        assert_eq!((bb.provider, bb.pr.as_deref(), bb.repo.as_deref()), (Provider::Bitbucket, Some("5"), Some("team/app")));

        let jk = detect(&Env::from_pairs(&[
            ("JENKINS_URL", "https://ci/"), ("CHANGE_TARGET", "main"), ("CHANGE_ID", "9"), ("GIT_URL", "git@github.com:acme/api.git"),
            ("CHANGE_TITLE", "Add retries"),
        ]))
        .unwrap();
        assert_eq!((jk.repo.as_deref(), jk.description().as_deref()), (Some("acme/api"), Some("Add retries")));

        let cc = detect(&Env::from_pairs(&[
            ("CIRCLECI", "true"), ("CIRCLE_SHA1", "s"), ("CIRCLE_PULL_REQUEST", "https://github.com/acme/web/pull/88"),
            ("CIRCLE_PROJECT_USERNAME", "acme"), ("CIRCLE_PROJECT_REPONAME", "web"),
        ]))
        .unwrap();
        assert_eq!((cc.base_ref, cc.pr.as_deref(), cc.repo.as_deref()), (None, Some("88"), Some("acme/web")));

        let br = detect(&Env::from_pairs(&[
            ("BITRISE_IO", "true"), ("BITRISEIO_GIT_BRANCH_DEST", "main"), ("BITRISE_PULL_REQUEST", "4"),
            ("GIT_REPOSITORY_URL", "https://github.com/acme/ios.git"),
        ]))
        .unwrap();
        assert_eq!((br.provider, br.pr.as_deref(), br.repo.as_deref()), (Provider::Bitrise, Some("4"), Some("acme/ios")));

        let bk = detect(&Env::from_pairs(&[("BUILDKITE", "true"), ("BUILDKITE_PULL_REQUEST", "false"), ("BUILDKITE_REPO", "git@github.com:acme/x.git")])).unwrap();
        assert_eq!((bk.pr, bk.repo.as_deref()), (None, Some("acme/x")));

        let az = detect(&Env::from_pairs(&[
            ("TF_BUILD", "True"), ("SYSTEM_PULLREQUEST_TARGETBRANCH", "refs/heads/release/2"), ("SYSTEM_PULLREQUEST_PULLREQUESTID", "31"),
            ("BUILD_REPOSITORY_NAME", "acme/svc"),
        ]))
        .unwrap();
        assert_eq!((az.provider, az.base_ref.as_deref(), az.pr.as_deref()), (Provider::Azure, Some("release/2"), Some("31")));
    }

    #[test]
    fn the_github_description_comes_from_the_event_payload() {
        let dir = std::env::temp_dir().join(format!("pr-guard-ci-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let event = dir.join("event.json");
        std::fs::write(&event, r#"{"pull_request": {"title": "Add search", "body": "Uses a bound parameter."}}"#).unwrap();
        let env = |p: &Path| Env::from_pairs(&[("GITHUB_ACTIONS", "true"), ("GITHUB_EVENT_PATH", p.to_str().unwrap())]);
        assert_eq!(detect(&env(&event)).unwrap().description().as_deref(), Some("Add search\n\nUses a bound parameter."));
        std::fs::write(&event, r#"{"pull_request": {"title": "Only a title", "body": null}}"#).unwrap();
        assert_eq!(detect(&env(&event)).unwrap().description().as_deref(), Some("Only a title"));
        std::fs::write(&event, r#"{"ref": "refs/heads/main"}"#).unwrap();
        assert_eq!(detect(&env(&event)).unwrap().description(), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
