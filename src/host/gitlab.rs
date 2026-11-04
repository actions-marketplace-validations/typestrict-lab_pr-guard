//! GitLab REST v4 (gitlab.com and self-managed). The project id is the URL-encoded path, so no lookup is needed.
//! Required reviewers use the merge request's own approval rule, which GitLab enforces (Premium and above).

use serde_json::{Value, json};

use super::{Client, Comment, State, enc, id};

const RULE: &str = "pr-guard";

fn mr(c: &Client) -> String {
    format!("{}/projects/{}/merge_requests/{}", c.t.api, enc(&c.t.repo), c.t.pr)
}

pub fn me(c: &Client) -> anyhow::Result<String> {
    let u = c.get(format!("{}/user", c.t.api))?;
    Ok(u["username"].as_str().unwrap_or_default().to_string())
}

/// System notes ("added 1 commit", label changes) are not comments.
pub fn comments(c: &Client) -> anyhow::Result<Vec<Comment>> {
    let items = c.pages(&format!("{}/notes?sort=asc&order_by=created_at", mr(c)))?;
    Ok(items
        .iter()
        .filter(|n| !n["system"].as_bool().unwrap_or(false))
        .map(|n| Comment { id: id(&n["id"]), author: n["author"]["username"].as_str().unwrap_or_default().into(), body: n["body"].as_str().unwrap_or_default().into() })
        .collect())
}

pub fn create_comment(c: &Client, body: &str) -> anyhow::Result<()> {
    c.post(format!("{}/notes", mr(c)), json!({ "body": body })).map(drop)
}

pub fn update_comment(c: &Client, note: &str, body: &str) -> anyhow::Result<()> {
    c.put(format!("{}/notes/{note}", mr(c)), json!({ "body": body })).map(drop)
}

pub fn labels(c: &Client) -> anyhow::Result<Vec<String>> {
    let m = c.get(mr(c))?;
    Ok(m["labels"].as_array().into_iter().flatten().filter_map(|l| l.as_str().map(String::from)).collect())
}

/// One call: `add_labels` and `remove_labels` leave every other label alone (and create missing ones).
pub fn set_labels(c: &Client, add: &[String], remove: &[String]) -> anyhow::Result<()> {
    c.put(mr(c), json!({ "add_labels": add.join(","), "remove_labels": remove.join(",") })).map(drop)
}

pub fn status(c: &Client, sha: &str, name: &str, state: State, description: &str) -> anyhow::Result<()> {
    let state = match state {
        State::Success => "success",
        State::Failure => "failed",
        State::Waiting => "pending",
    };
    let mut body = json!({ "state": state, "name": name, "description": description });
    if let Some(l) = &c.t.link {
        body["target_url"] = Value::String(l.clone());
    }
    c.post(format!("{}/projects/{}/statuses/{sha}", c.t.api, enc(&c.t.repo)), body).map(drop)
}

pub fn head(c: &Client) -> anyhow::Result<String> {
    let m = c.get(mr(c))?;
    m["sha"].as_str().map(String::from).ok_or_else(|| anyhow::anyhow!("GitLab merge request {} has no sha", c.t.pr))
}

pub fn approvals(c: &Client) -> anyhow::Result<u32> {
    let a = c.get(format!("{}/approvals", mr(c)))?;
    Ok(a["approved_by"].as_array().map_or(0, |v| v.len() as u32))
}

/// The merge request's `pr-guard` rule asks for `n` approvals: updated when it exists, created only when someone is
/// needed (a rule asking for 0 is noise in the merge request's widget).
pub fn require_approvals(c: &Client, n: u8) -> anyhow::Result<bool> {
    let rules = c.get(format!("{}/approval_rules", mr(c)))?;
    let ours = rules.as_array().into_iter().flatten().find(|r| r["name"] == RULE);
    match ours {
        Some(r) if r["approvals_required"] == n => {}
        Some(r) => {
            c.put(format!("{}/approval_rules/{}", mr(c), id(&r["id"])), json!({ "approvals_required": n }))?;
        }
        None if n == 0 => {}
        None => {
            c.post(format!("{}/approval_rules", mr(c)), json!({ "name": RULE, "approvals_required": n }))?;
        }
    }
    Ok(true)
}
