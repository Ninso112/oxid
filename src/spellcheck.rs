// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - Spellcheck support

use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Spellchecker using word lists from system or config.
#[allow(dead_code)]
pub struct Spellchecker {
    dict: HashSet<String>,
}

#[allow(dead_code)]
impl Spellchecker {
    /// Create spellchecker with given languages. Tries common system paths.
    pub fn new(languages: &[String]) -> Self {
        let mut dict = HashSet::new();
        for lang in languages {
            let words = Self::load_dict(lang);
            for w in words {
                dict.insert(w.to_lowercase());
            }
        }
        Self { dict }
    }

    fn load_dict(lang: &str) -> Vec<String> {
        let paths = [
            format!("/usr/share/dict/{}-words", lang),
            format!("/usr/share/dict/{}", lang),
            format!("/usr/share/hunspell/{}.dic", lang),
            format!("/usr/share/myspell/dicts/{}.dic", lang),
        ];

        for path in &paths {
            if let Ok(content) = fs::read_to_string(Path::new(path)) {
                return content
                    .lines()
                    .filter_map(|l| {
                        let word = l.split('/').next()?.trim().to_string();
                        if word.chars().all(|c| c.is_alphabetic()) && word.len() > 1 {
                            Some(word)
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }

        // Fallback: /usr/share/dict/words (common on Linux)
        if lang == "en" {
            if let Ok(content) = fs::read_to_string("/usr/share/dict/words") {
                return content
                    .lines()
                    .filter_map(|l| {
                        let word = l.trim().to_string();
                        if word.chars().all(|c| c.is_alphabetic()) && word.len() > 1 {
                            Some(word)
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }

        Vec::new()
    }

    /// Check if word is correctly spelled.
    pub fn check(&self, word: &str) -> bool {
        if word.is_empty() || word.chars().any(|c| !c.is_alphabetic()) {
            return true;
        }
        self.dict.contains(&word.to_lowercase())
    }

    /// Extract misspelled words from text. Returns set of (start_byte, end_byte) for each misspelled word.
    pub fn find_misspelled_ranges(&self, text: &str) -> Vec<(usize, usize)> {
        let re = Regex::new(r"\b[a-zA-Z][a-zA-Z']*\b")
            .unwrap_or_else(|_| Regex::new(r"\b\w+\b").unwrap());
        let mut ranges = Vec::new();
        for mat in re.find_iter(text) {
            let word = mat.as_str();
            if !self.check(word) {
                ranges.push((mat.start(), mat.end()));
            }
        }
        ranges
    }
}
