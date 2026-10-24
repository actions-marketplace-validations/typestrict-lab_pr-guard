//! GitLab Code Quality (the Code Climate issue format): the merge request widget lists each finding.

use serde_json::{Value, json};

use super::fingerprint;
use crate::policy::Level;
use crate::verdict::Verdict;

pub fn render(v: &Verdict) -> String {
    let issues: Vec<Value> = v
        .findings
        .iter()
        .map(|f| {
            let severity = match f.level {
                Level::Danger => "critical",
                Level::Warning => "major",
                Level::Info => "info",
            };
            json!({
                "type": "issue",
                "check_name": format!("pr-guard/{}", f.rule),
                "description": f.message,
                "categories": [if f.rule == "secrets" || f.rule == "vulnerability" { "Security" } else { "Bug Risk" }],
                "severity": severity,
                "fingerprint": fingerprint(f),
                // GitLab requires a path and a line: a finding about the whole change sits at the repository root.
                "location": {"path": f.path.as_deref().unwrap_or("."), "lines": {"begin": f.line.unwrap_or(1)}},
            })
        })
        .collect();
    format!("{}\n", serde_json::to_string_pretty(&issues).expect("issues always serialize"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::tests::sample;
    use crate::policy::Mode;

    #[test]
    fn every_finding_is_an_issue_with_the_fields_gitlab_requires() {
        let v = sample(Mode::Enforce);
        let issues: Vec<Value> = serde_json::from_str(&render(&v)).unwrap();
        assert_eq!(issues.len(), v.findings.len());
        for i in &issues {
            for k in ["description", "check_name", "fingerprint", "severity"] {
                assert!(i[k].is_string(), "{k} in {i}");
            }
            assert!(i["location"]["path"].is_string() && i["location"]["lines"]["begin"].is_u64());
        }
        assert_eq!((issues[0]["severity"].as_str(), issues[0]["location"]["path"].as_str()), (Some("critical"), Some("src/a.ts")));
        assert_eq!(issues[0]["location"]["lines"]["begin"].as_u64(), Some(3));
        assert_eq!(issues[1]["check_name"].as_str(), Some("pr-guard/size"));
    }
}
