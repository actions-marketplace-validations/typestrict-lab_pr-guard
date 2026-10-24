//! JUnit XML: Jenkins, CircleCI, Bitrise, Azure and most CIs show failed tests without a plugin. One test case for the
//! verdict, one per question the model answered and one per rule finding; a case fails at danger level (never in
//! shadow mode, which must not fail anything) and the verdict fails when the policy blocks.

use std::fmt::Write;

use super::{location, question_name};
use crate::policy::Level;
use crate::verdict::{Finding, Verdict};

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // XML 1.0 has no escape for most control characters: drop them.
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

struct Case {
    class: &'static str,
    name: String,
    failure: Option<String>,
    skipped: bool,
    out: String,
}

pub fn render(v: &Verdict) -> String {
    let fails = |f: &Finding| !v.shadow && f.level >= Level::Danger;
    let mut cases = Vec::new();
    let summary = match v.reviewers {
        0 => "no reviewer needed".to_string(),
        1 => "needs 1 reviewer".to_string(),
        n => format!("needs {n} reviewers"),
    };
    cases.push(Case {
        class: "pr-guard",
        name: "verdict".into(),
        failure: v.blocked.then(|| format!("Blocked by the policy, {summary}")),
        skipped: v.skipped,
        out: if v.skipped { "Skipped by label".into() } else { summary },
    });
    for (q, score) in &v.scores {
        let name = question_name(q);
        let finding = v.findings.iter().find(|f| f.rule == name);
        cases.push(Case {
            class: "pr-guard.questions",
            out: match finding {
                Some(f) => format!("{:?}: {} (score {:.0}%)", f.level, f.message, score * 100.0),
                None => format!("score {:.0}%", score * 100.0),
            },
            failure: finding.filter(|f| fails(f)).map(|f| f.message.clone()),
            name,
            skipped: false,
        });
    }
    let questions: Vec<String> = v.scores.keys().map(question_name).collect();
    for f in v.findings.iter().filter(|f| !questions.contains(&f.rule)) {
        let at = location(f);
        cases.push(Case {
            class: "pr-guard.rules",
            name: if at.is_empty() { f.rule.clone() } else { format!("{} {at}", f.rule) },
            failure: fails(f).then(|| f.message.clone()),
            skipped: false,
            out: format!("{:?}: {}", f.level, f.message),
        });
    }

    let failures = cases.iter().filter(|c| c.failure.is_some()).count();
    let skipped = cases.iter().filter(|c| c.skipped).count();
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(s, "<testsuites name=\"pr-guard\" tests=\"{}\" failures=\"{failures}\" errors=\"0\" skipped=\"{skipped}\">", cases.len());
    let _ = writeln!(s, "  <testsuite name=\"pr-guard\" tests=\"{}\" failures=\"{failures}\" errors=\"0\" skipped=\"{skipped}\">", cases.len());
    for c in &cases {
        let _ = writeln!(s, "    <testcase classname=\"{}\" name=\"{}\">", c.class, esc(&c.name));
        if let Some(m) = &c.failure {
            let _ = writeln!(s, "      <failure message=\"{}\" type=\"danger\">{}</failure>", esc(m), esc(m));
        }
        if c.skipped {
            s.push_str("      <skipped/>\n");
        }
        let _ = writeln!(s, "      <system-out>{}</system-out>", esc(&c.out));
        s.push_str("    </testcase>\n");
    }
    s.push_str("  </testsuite>\n</testsuites>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::tests::sample;
    use crate::policy::Mode;

    /// Well-formed enough for a CI's parser: tags nest and close, attributes are quoted, text has no raw `<` or `&`.
    fn check_xml(xml: &str) -> Vec<String> {
        let body = xml.strip_prefix("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n").expect("XML declaration");
        let (mut stack, mut seen, mut rest) = (Vec::<String>::new(), Vec::new(), body);
        while let Some(open) = rest.find('<') {
            let text = &rest[..open];
            assert!(!text.contains('>') && text.split('&').skip(1).all(|t| ["amp;", "lt;", "gt;", "quot;", "apos;"].iter().any(|e| t.starts_with(e))), "text {text:?}");
            let close = open + rest[open..].find('>').expect("unclosed tag");
            let tag = &rest[open + 1..close];
            rest = &rest[close + 1..];
            assert!(!tag.contains('<') && tag.matches('"').count() % 2 == 0, "tag {tag:?}");
            if let Some(name) = tag.strip_prefix('/') {
                assert_eq!(stack.pop().as_deref(), Some(name), "closing {name}");
            } else {
                let name = tag.split_whitespace().next().unwrap().trim_end_matches('/').to_string();
                seen.push(name.clone());
                if !tag.ends_with('/') {
                    stack.push(name);
                }
            }
        }
        assert!(stack.is_empty() && rest.trim().is_empty(), "unclosed {stack:?}");
        seen
    }

    #[test]
    fn junit_is_well_formed_and_fails_on_danger_and_blocked_only_outside_shadow() {
        let xml = render(&sample(Mode::Enforce));
        let tags = check_xml(&xml);
        assert_eq!(tags.iter().filter(|t| *t == "testcase").count(), 5, "{xml}"); // verdict, 2 questions, 2 rule findings
        assert_eq!(tags.iter().filter(|t| *t == "failure").count(), 2, "{xml}"); // the verdict and the secret
        assert!(xml.contains("name=\"secrets src/a.ts:3\""));
        let shadow = render(&sample(Mode::Shadow));
        assert!(!check_xml(&shadow).contains(&"failure".to_string()), "{shadow}");
        assert_eq!(esc("a<b & \"c\"\u{1}"), "a&lt;b &amp; &quot;c&quot;");
    }
}
