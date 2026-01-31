// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - Note templates for new files

use chrono::Local;

/// Template type for new notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Empty,
    DailyNote,
    Meeting,
}

impl Template {
    pub fn name(&self) -> &'static str {
        match self {
            Template::Empty => "Empty",
            Template::DailyNote => "Daily Note",
            Template::Meeting => "Meeting",
        }
    }

    /// Generate template content.
    pub fn content(&self) -> String {
        match self {
            Template::Empty => String::new(),
            Template::DailyNote => format!("# {}\n\n", Local::now().format("%Y-%m-%d")),
            Template::Meeting => {
                "## Participants\n\n\n## Notes\n\n".to_string()
            }
        }
    }

    pub fn all() -> &'static [Template] {
        &[Template::Empty, Template::DailyNote, Template::Meeting]
    }
}
