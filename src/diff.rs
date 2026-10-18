use std::collections::BTreeSet;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AddedLine {
    pub file: String,
    pub line: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedDiff {
    pub text: String,
    pub files: BTreeSet<String>,
    pub added_lines: Vec<AddedLine>,
}

impl ParsedDiff {
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut files = BTreeSet::new();
        let mut added_lines = Vec::new();
        let mut current_file: Option<String> = None;
        let mut new_line: Option<u32> = None;

        for raw in text.lines() {
            if let Some(path) = raw.strip_prefix("+++ b/") {
                current_file = Some(path.to_owned());
                files.insert(path.to_owned());
                continue;
            }
            if raw.starts_with("@@ ") {
                new_line = parse_hunk_new_start(raw);
                continue;
            }
            let Some(line_no) = new_line else {
                continue;
            };
            if raw.starts_with('+') && !raw.starts_with("+++") {
                if let Some(file) = &current_file {
                    added_lines.push(AddedLine {
                        file: file.clone(),
                        line: line_no,
                        text: raw[1..].to_owned(),
                    });
                }
                new_line = line_no.checked_add(1);
            } else if raw.starts_with('-') && !raw.starts_with("---") {
            } else {
                new_line = line_no.checked_add(1);
            }
        }
        Self {
            text: text.to_owned(),
            files,
            added_lines,
        }
    }
}

fn parse_hunk_new_start(line: &str) -> Option<u32> {
    let plus = line.split_whitespace().find(|part| part.starts_with('+'))?;
    plus.trim_start_matches('+').split(',').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::ParsedDiff;

    #[test]
    fn tracks_added_line_numbers() {
        let diff = "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -10,2 +10,3 @@\n old\n+new\n tail\n";
        let parsed = ParsedDiff::parse(diff);
        assert_eq!(parsed.added_lines.len(), 1);
        assert_eq!(parsed.added_lines[0].line, 11);
        assert_eq!(parsed.added_lines[0].text, "new");
    }
}
