// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - Input handling: key comparison against config

use crossterm::event::KeyEvent;

/// Returns true if the pressed key matches any of the given keys (code + modifiers only).
pub fn key_matches(event: KeyEvent, keys: &[KeyEvent]) -> bool {
    keys.iter()
        .any(|k| event.code == k.code && event.modifiers == k.modifiers)
}
