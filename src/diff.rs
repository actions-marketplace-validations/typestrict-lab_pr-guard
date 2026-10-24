//! Unified diff parsing: which files changed and which lines were added, with their new line numbers (for annotations).

#[derive(Debug, Clone, PartialEq)]
pub struct FileChange {
    pub path: String,
    pub deleted: bool,
    /// Added lines as (line number in the new file, text without the leading `+`).
    pub added: Vec<(u32, String)>,
    pub removed: u32,
}

impl FileChange {
    pub fn changed_lines(&self) -> u32 {
        self.added.len() as u32 + self.removed
    }
}

pub fn parse(diff: &str) -> Vec<FileChange> {
    let mut files: Vec<FileChange> = Vec::new();
    let mut old_path = String::new();
    let mut line = 0u32;
    for l in diff.lines() {
        if let Some(p) = l.strip_prefix("--- ") {
            old_path = strip_side(p, "a/");
        } else if let Some(p) = l.strip_prefix("+++ ") {
            let new = strip_side(p, "b/");
            let deleted = new == "/dev/null";
            files.push(FileChange { path: if deleted { old_path.clone() } else { new }, deleted, added: vec![], removed: 0 });
        } else if let Some(h) = l.strip_prefix("@@ ") {
            // @@ -a,b +c,d @@: the next added line is number c.
            line = h.split_whitespace().find_map(|t| t.strip_prefix('+')).and_then(|t| t.split(',').next()?.parse().ok()).unwrap_or(1);
        } else if let Some(f) = files.last_mut() {
            if let Some(t) = l.strip_prefix('+') {
                f.added.push((line, t.to_string()));
                line += 1;
            } else if l.starts_with('-') {
                f.removed += 1;
            } else if l.starts_with(' ') || l.is_empty() {
                line += 1;
            }
        }
    }
    files
}

fn strip_side(p: &str, side: &str) -> String {
    let p = p.split('\t').next().unwrap_or(p).trim_end();
    p.strip_prefix(side).unwrap_or(p).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_lines_carry_their_new_line_numbers() {
        let d = "diff --git a/src/a b.rs b/src/a b.rs\n--- a/src/a b.rs\n+++ b/src/a b.rs\n@@ -10,3 +10,4 @@ fn x()\n ctx\n-old\n+new\n+more\n ctx\n--- a/gone.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-bye\n";
        let f = parse(d);
        assert_eq!(f[0].path, "src/a b.rs");
        assert_eq!(f[0].added, vec![(11, "new".into()), (12, "more".into())]);
        assert_eq!(f[0].removed, 1);
        assert!(f[1].deleted && f[1].path == "gone.txt" && f[1].changed_lines() == 1);
    }
}
