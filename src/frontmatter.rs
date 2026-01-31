// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - YAML frontmatter parsing for tags

use regex::Regex;
use std::collections::HashSet;

/// Parse tags from YAML-like frontmatter at the top of markdown content.
/// Looks for `tags: [a, b, c]` or `tags: a, b, c`.
pub fn parse_tags(content: &str) -> HashSet<String> {
    let mut tags = HashSet::new();

    let frontmatter_re = match Regex::new(r"(?s)^---\s*\n(.*?)\n---") {
        Ok(r) => r,
        Err(_) => return tags,
    };
    let frontmatter = match frontmatter_re.captures(content) {
        Some(c) => match c.get(1) {
            Some(m) => m.as_str(),
            None => return tags,
        },
        None => return tags,
    };

    if let Ok(tags_re) = Regex::new(r"tags:\s*\[([^\]]*)\]") {
        if let Some(cap) = tags_re.captures(frontmatter) {
            if let Some(m) = cap.get(1) {
                for tag in m.as_str().split(',') {
                    let t = tag.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    if !t.is_empty() {
                        tags.insert(t);
                    }
                }
                return tags;
            }
        }
    }

    if let Ok(tags_line_re) = Regex::new(r"tags:\s*(.+)") {
        if let Some(cap) = tags_line_re.captures(frontmatter) {
            if let Some(m) = cap.get(1) {
                for tag in m.as_str().split(|c: char| c.is_whitespace() || c == ',') {
                    let t = tag.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    if !t.is_empty() {
                        tags.insert(t);
                    }
                }
            }
        }
    }

    tags
}
