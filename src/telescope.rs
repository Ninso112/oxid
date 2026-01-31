// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - Telescope-style fuzzy file search (Space+f)

use crate::app::NoteEntry;
use crate::frontmatter::parse_tags;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use std::fs;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

const MAX_CONTENT_BYTES: usize = 50_000;

/// Recursively find all .md files under a directory.
pub fn find_md_files_recursive(dir: &Path) -> Vec<NoteEntry> {
    let mut notes = Vec::new();
    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    let display = path.strip_prefix(dir).unwrap_or(path).display().to_string();
                    let (content, searchable) = read_note_content(path, &display);
                    notes.push(NoteEntry::new(
                        path.to_path_buf(),
                        display,
                        content,
                        searchable,
                    ));
                }
            }
        }
    }
    notes.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));
    notes
}

fn read_note_content(path: &Path, display: &str) -> (String, String) {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (String::new(), display.to_string()),
    };
    let mut buf = Vec::with_capacity(MAX_CONTENT_BYTES + 1);
    let mut take = file.take(MAX_CONTENT_BYTES as u64);
    if take.read_to_end(&mut buf).is_err() {
        return (String::new(), display.to_string());
    }
    let content = String::from_utf8_lossy(&buf).into_owned();
    let tags = parse_tags(&content);
    let tag_str: String = tags.into_iter().collect::<Vec<_>>().join(" ");
    let searchable = format!("{}\n{}\n{}", display, content, tag_str);
    (content, searchable)
}

/// Filter notes: if query starts with #, filter by tag; else fuzzy match.
pub fn filter_telescope_notes(
    notes: &[NoteEntry],
    query: &str,
    matcher: &mut Matcher,
) -> Vec<NoteEntry> {
    let query = query.trim();
    if query.is_empty() {
        return notes.to_vec();
    }

    if let Some(rest) = query.strip_prefix('#') {
        let tag = rest.trim().to_lowercase();
        if tag.is_empty() {
            return notes.to_vec();
        }
        return notes
            .iter()
            .filter(|n| {
                let tags = parse_tags(&n.content);
                tags.iter().any(|t| t.to_lowercase() == tag)
            })
            .cloned()
            .collect();
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let matches = pattern.match_list(notes, matcher);
    matches
        .into_iter()
        .map(|(entry, _)| entry.clone())
        .collect()
}

/// Get match indices for telescope list highlighting.
pub fn get_telescope_match_indices(display: &str, query: &str, matcher: &mut Matcher) -> Vec<u32> {
    if query.is_empty() || query.starts_with('#') || display.is_empty() {
        return Vec::new();
    }
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut buf = Vec::new();
    let haystack = Utf32Str::new(display, &mut buf);
    let mut indices = Vec::new();
    if pattern.indices(haystack, matcher, &mut indices).is_some() {
        indices.sort_unstable();
        indices.dedup();
    }
    indices
}
