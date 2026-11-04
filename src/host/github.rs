//! GitHub REST v3 (github.com and Enterprise Server). Pull request comments and labels live on the issue of the same
//! number; statuses are commit statuses, which any token with `statuses: write` sets (check runs need a GitHub App).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{Client, Comment, State, enc, id};

fn repo(c: &Client) -> String {
    format!("{}/repos/{}", c.t.api, c.t.repo)
}

pub fn me(c: &Client) -> anyhow::Result<String> {
    let u = c.get(format!("{}/user", c.t.api))?;
    Ok(u["login"].as_str().unwrap_or_default().to_string())
}

pub fn comments(c: &Client) -> anyhow::Result<Vec<Comment>> {
    let items = c.pages(&format!("{}/issues/{}/comments", repo(c), c.t.pr))?;
    Ok(items
        .iter()
        .map(|i| Comment { id: id(&i["id"]), author: i["user"]["login"].as_str().unwrap_or_default().into(), body: i["body"].as_str().unwrap_or_default().into() })
        .collect())
}

pub fn create_comment(c: &Client, body: &str) -> anyhow::Result<()> {
    c.post(format!("{}/issues/{}/comments", repo(c), c.t.pr), json!({ "body": body })).map(drop)
}

pub fn update_comment(c: &Client, comment: &str, body: &str) -> anyhow::Result<()> {
    c.patch(format!("{}/issues/comments/{comment}", repo(c)), json!({ "body": body })).map(drop)
}

pub fn labels(c: &Client) -> anyhow::Result<Vec<String>> {
    let items = c.pages(&format!("{}/issues/{}/labels", repo(c), c.t.pr))?;
    Ok(items.iter().filter_map(|l| l["name"].as_str().map(String::from)).collect())
}

/// Adding creates a missing label in the repository; removing is one call per label.
pub fn set_labels(c: &Client, add: &[String], remove: &[String]) -> anyhow::Result<()> {
    if !add.is_empty() {
        c.post(format!("{}/issues/{}/labels", repo(c), c.t.pr), json!({ "labels": add }))?;
    }
    for l in remove {
        c.delete(format!("{}/issues/{}/labels/{}", repo(c), c.t.pr, enc(l)))?;
    }
    Ok(())
}

pub fn status(c: &Client, sha: &str, name: &str, state: State, description: &str) -> anyhow::Result<()> {
    let state = match state {
        State::Success => "success",
        State::Failure => "failure",
        State::Waiting => "pending",
    };
    let mut body = json!({ "state": state, "context": name, "description": description });
    if let Some(l) = &c.t.link {
        body["target_url"] = Value::String(l.clone());
    }
    c.post(format!("{}/statuses/{sha}", repo(c)), body).map(drop)
}

pub fn head(c: &Client) -> anyhow::Result<String> {
    let pr = c.get(format!("{}/pulls/{}", repo(c), c.t.pr))?;
    pr["head"]["sha"].as_str().map(String::from).ok_or_else(|| anyhow::anyhow!("GitHub pull request {} has no head.sha", c.t.pr))
}

/// Reviewers whose latest decisive review approves. A comment does not undo an approval; a later "changes requested"
/// or a dismissal does (the API returns reviews oldest first).
pub fn approvals(c: &Client) -> anyhow::Result<u32> {
    let mut last: BTreeMap<String, String> = BTreeMap::new();
    for r in c.pages(&format!("{}/pulls/{}/reviews", repo(c), c.t.pr))? {
        let state = r["state"].as_str().unwrap_or_default();
        if matches!(state, "APPROVED" | "CHANGES_REQUESTED" | "DISMISSED") {
            last.insert(r["user"]["login"].as_str().unwrap_or_default().into(), state.into());
        }
    }
    Ok(last.values().filter(|s| *s == "APPROVED").count() as u32)
}
