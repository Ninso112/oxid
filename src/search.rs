// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

use crate::app::NoteEntry;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};

/// Filter notes using fuzzy matching against both filename and content.
/// Filename matches rank higher than content matches (filename is first in searchable string).
pub fn filter_notes(
    notes: &[NoteEntry],
    query: &str,
    matcher: &mut Matcher,
) -> Vec<NoteEntry> {
    if query.is_empty() {
        return notes.to_vec();
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let matches = pattern.match_list(notes, matcher);

    matches.into_iter().map(|(entry, _)| entry.clone()).collect()
}

/// Get match indices for highlighting in the display (filename) string.
/// Returns character indices that match the query. Empty vec if no match or no query.
pub fn get_match_indices(
    display: &str,
    query: &str,
    matcher: &mut Matcher,
) -> Vec<u32> {
    if query.is_empty() || display.is_empty() {
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
