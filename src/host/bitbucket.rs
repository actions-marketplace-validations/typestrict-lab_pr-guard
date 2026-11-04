//! Bitbucket Cloud REST 2.0. No pull request labels; required reviewers are a build status that fails until the
//! approvals arrive, enforced by the "passing builds" merge check. Bitbucket Data Center has another API and is not
//! covered.

use serde_json::{Value, json};

use super::{Client, Comment, State, id};

fn repo(c: &Client) -> String {
    format!("{}/repositories/{}", c.t.api, c.t.repo)
}

fn pr(c: &Client) -> String {
    format!("{}/pullrequests/{}", repo(c), c.t.pr)
}

/// Accounts are compared by `account_id`, which, unlike the nickname, never changes.
fn who(u: &Value) -> String {
    u["account_id"].as_str().or_else(|| u["uuid"].as_str()).unwrap_or_default().to_string()
}

pub fn me(c: &Client) -> anyhow::Result<String> {
    Ok(who(&c.get(format!("{}/user", c.t.api))?))
}

/// Pages through `next` links, which already carry the query.
pub fn comments(c: &Client) -> anyhow::Result<Vec<Comment>> {
    let mut url = Some(format!("{}/comments?pagelen=100", pr(c)));
    let mut out = Vec::new();
    while let Some(u) = url.take() {
        let page = c.get(u)?;
        for v in page["values"].as_array().into_iter().flatten() {
            if v["deleted"].as_bool().unwrap_or(false) {
                continue;
            }
            out.push(Comment { id: id(&v["id"]), author: who(&v["user"]), body: v["content"]["raw"].as_str().unwrap_or_default().into() });
        }
        url = page["next"].as_str().map(String::from);
    }
    Ok(out)
}

pub fn create_comment(c: &Client, body: &str) -> anyhow::Result<()> {
    c.post(format!("{}/comments", pr(c)), json!({ "content": { "raw": body } })).map(drop)
}

pub fn update_comment(c: &Client, comment: &str, body: &str) -> anyhow::Result<()> {
    c.put(format!("{}/comments/{comment}", pr(c)), json!({ "content": { "raw": body } })).map(drop)
}

/// Bitbucket requires a link on every status: the pipeline when known, else the pull request.
pub fn status(c: &Client, sha: &str, name: &str, state: State, description: &str) -> anyhow::Result<()> {
    let state = match state {
        State::Success => "SUCCESSFUL",
        State::Failure | State::Waiting => "FAILED",
    };
    let url = c.t.link.clone().unwrap_or_else(|| format!("https://bitbucket.org/{}/pull-requests/{}", c.t.repo, c.t.pr));
    // Keys are limited to letters, digits and dashes in practice; the name keeps the slash.
    let key = name.replace('/', "-");
    let body = json!({ "key": key, "state": state, "name": name, "description": description, "url": url });
    c.post(format!("{}/commit/{sha}/statuses/build", repo(c)), body).map(drop)
}

pub fn head(c: &Client) -> anyhow::Result<String> {
    let p = c.get(pr(c))?;
    p["source"]["commit"]["hash"].as_str().map(String::from).ok_or_else(|| anyhow::anyhow!("Bitbucket pull request {} has no source commit", c.t.pr))
}

pub fn approvals(c: &Client) -> anyhow::Result<u32> {
    let p = c.get(pr(c))?;
    Ok(p["participants"].as_array().into_iter().flatten().filter(|p| p["approved"].as_bool().unwrap_or(false)).count() as u32)
}
