//! SARIF 2.1.0: GitHub code scanning, Azure DevOps and most security dashboards read it.

use serde_json::{Value, json};

use super::fingerprint;
use crate::policy::Level;
use crate::verdict::Verdict;

fn rule_description(id: &str) -> &str {
    match id {
        "sensitive_paths" => "Changes a path the policy marks as sensitive",
        "secrets" => "A credential committed in the code",
        "dependencies" => "Changes dependency manifests or lockfiles",
        "size" => "More changed lines than the policy allows",
        "destructive_sql" => "Destructive SQL in a migration",
        "description_matches" => "The description may not match the diff",
        "complexity" => "Hard to review",
        "vulnerability" => "May introduce a security vulnerability",
        "tests_cover" => "The tests may not cover the change",
        "breaking" => "May break compatibility for users",
        "resolves_issue" => "May not resolve the linked issue",
        "focused" => "May mix unrelated work",
        _ => id,
    }
}

pub fn render(v: &Verdict) -> String {
    // One rule per rule id, in the order the findings first use them; results point at their rule by index.
    let mut ids: Vec<&str> = Vec::new();
    for f in &v.findings {
        if !ids.contains(&f.rule.as_str()) {
            ids.push(&f.rule);
        }
    }
    let rules: Vec<Value> = ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "name": id,
                "shortDescription": {"text": rule_description(id)},
                "helpUri": "https://github.com/typestrict-lab/pr-guard#readme",
            })
        })
        .collect();
    let results: Vec<Value> = v
        .findings
        .iter()
        .map(|f| {
            let level = match f.level {
                Level::Danger => "error",
                Level::Warning => "warning",
                Level::Info => "note",
            };
            let mut r = json!({
                "ruleId": f.rule,
                "ruleIndex": ids.iter().position(|id| *id == f.rule).unwrap_or(0),
                "level": level,
                "message": {"text": f.message},
                "partialFingerprints": {"prGuardFinding/v1": fingerprint(f)},
            });
            if let Some(path) = &f.path {
                let mut physical = json!({"artifactLocation": {"uri": path, "uriBaseId": "%SRCROOT%"}});
                if let Some(line) = f.line {
                    physical["region"] = json!({"startLine": line});
                }
                r["locations"] = json!([{"physicalLocation": physical}]);
            }
            r
        })
        .collect();
    let doc = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {"driver": {
                "name": "pr-guard",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": "https://github.com/typestrict-lab/pr-guard",
                "rules": rules,
            }},
            "results": results,
            "properties": {"reviewers": v.reviewers, "blocked": v.blocked, "shadow": v.shadow, "skipped": v.skipped, "score": v.score},
        }],
    });
    format!("{}\n", serde_json::to_string_pretty(&doc).expect("SARIF always serializes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::tests::sample;
    use crate::policy::Mode;

    #[test]
    fn sarif_is_2_1_0_with_one_rule_per_id_and_locations_only_when_known() {
        let v = sample(Mode::Enforce);
        let doc: Value = serde_json::from_str(&render(&v)).unwrap();
        assert_eq!(doc["version"], "2.1.0");
        let run = &doc["runs"][0];
        let rules: Vec<&str> = run["tool"]["driver"]["rules"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert_eq!(rules, ["secrets", "size", "tests_cover"]);
        let results = run["results"].as_array().unwrap();
        assert_eq!(results.len(), v.findings.len());
        let secret = &results[0];
        assert_eq!((secret["ruleId"].as_str(), secret["level"].as_str(), secret["ruleIndex"].as_u64()), (Some("secrets"), Some("error"), Some(0)));
        let loc = &secret["locations"][0]["physicalLocation"];
        assert_eq!((loc["artifactLocation"]["uri"].as_str(), loc["region"]["startLine"].as_u64()), (Some("src/a.ts"), Some(3)));
        assert!(results[1].get("locations").is_none());
        assert_eq!(results[2]["ruleIndex"].as_u64(), Some(2));
    }
}
