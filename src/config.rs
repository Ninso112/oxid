// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Theme overrides in config.toml. Hex (#RRGGBB) or named colors.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub selection: String,
    pub statusbar_bg: String,
    pub statusbar_fg: String,
    pub border_color: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background: "black".to_string(),
            foreground: "white".to_string(),
            cursor: "cyan".to_string(),
            selection: "green".to_string(),
            statusbar_bg: "black".to_string(),
            statusbar_fg: "white".to_string(),
            border_color: "cyan".to_string(),
        }
    }
}

/// UI behavior and appearance (borders, icons, hidden files).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Border style: "rounded", "double", "thick", "plain".
    pub border_style: String,
    /// Use Nerd Fonts icons in file tree.
    pub icons: bool,
    /// Show dotfiles in file tree.
    pub show_hidden: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            border_style: "rounded".to_string(),
            icons: false,
            show_hidden: false,
        }
    }
}

/// Editor-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub typewriter_mode: bool,
    pub enable_spellcheck: bool,
    pub spellcheck_languages: Vec<String>,
    pub show_backlinks: bool,
    pub syntax_highlighting: bool,
    pub auto_save: bool,
    pub auto_save_interval: u64,
    /// Show line numbers in gutter.
    pub line_numbers: bool,
    /// Relative / hybrid line numbers (when line_numbers is true).
    pub rel_line_numbers: bool,
    /// Tab width in spaces (1–16).
    pub tab_width: u8,
    /// Enable mouse in editor.
    pub mouse_support: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            typewriter_mode: false,
            enable_spellcheck: false,
            spellcheck_languages: vec!["en".to_string()],
            show_backlinks: true,
            syntax_highlighting: true,
            auto_save: true,
            auto_save_interval: 30,
            line_numbers: true,
            rel_line_numbers: false,
            tab_width: 4,
            mouse_support: true,
        }
    }
}

/// Keybindings configuration (string form, e.g. "ctrl-q", "enter").
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct KeysConfig {
    // Global
    pub quit: String,
    pub zen_mode: String,
    pub search: String,
    pub command_palette: String,
    pub daily_note: String,
    pub task_board: String,
    // Generic (used in multiple contexts)
    pub escape: String,
    pub enter: String,
    pub backspace: String,
    pub move_up: String,
    pub move_down: String,
    pub move_left: String,
    pub move_up_alt: String,
    pub move_down_alt: String,
    pub move_left_alt: String,
    pub delete: String,
    // List (file explorer) Normal mode
    pub list_create_note: String,
    pub list_create_dir: String,
    pub list_tag_explorer: String,
    pub list_rename: String,
    pub list_edit_config: String,
    pub list_delete: String,
    pub list_parent: String,
    pub list_parent_alt: String,
    // Editor
    pub editor_back: String,
    pub editor_pdf: String,
    pub editor_backlinks: String,
    pub editor_wiki_link: String,
    pub editor_insert: String,
    pub editor_append: String,
    pub editor_split_focus: String,
}

impl Default for KeysConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            zen_mode: "f11".to_string(),
            search: "/".to_string(),
            command_palette: "ctrl-p".to_string(),
            daily_note: "alt-d".to_string(),
            task_board: "alt-t".to_string(),
            escape: "esc".to_string(),
            enter: "enter".to_string(),
            backspace: "backspace".to_string(),
            move_up: "k".to_string(),
            move_down: "j".to_string(),
            move_left: "h".to_string(),
            move_up_alt: "up".to_string(),
            move_down_alt: "down".to_string(),
            move_left_alt: "left".to_string(),
            delete: "delete".to_string(),
            list_create_note: "n".to_string(),
            list_create_dir: "shift-n".to_string(),
            list_tag_explorer: "shift-t".to_string(),
            list_rename: "r".to_string(),
            list_edit_config: "c".to_string(),
            list_delete: "d".to_string(),
            list_parent: "backspace".to_string(),
            list_parent_alt: "left".to_string(),
            editor_back: "q".to_string(),
            editor_pdf: "ctrl-e".to_string(),
            editor_backlinks: "ctrl-b".to_string(),
            editor_wiki_link: "ctrl-]".to_string(),
            editor_insert: "i".to_string(),
            editor_append: "a".to_string(),
            editor_split_focus: "tab".to_string(),
        }
    }
}

/// Parses a key string (e.g. "ctrl-q", "enter", "f1") into a KeyEvent.
pub fn parse_key_event(s: &str) -> Option<KeyEvent> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split('-').collect();
    let (modifiers, key_part) = if parts.len() >= 2 {
        let mut mods = KeyModifiers::empty();
        for p in parts.iter().take(parts.len() - 1) {
            match *p {
                "ctrl" => mods.insert(KeyModifiers::CONTROL),
                "alt" => mods.insert(KeyModifiers::ALT),
                "shift" => mods.insert(KeyModifiers::SHIFT),
                _ => {}
            }
        }
        (mods, parts[parts.len() - 1])
    } else {
        (KeyModifiers::empty(), parts[0])
    };

    let code = match key_part {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "tab" => KeyCode::Tab,
        "delete" => KeyCode::Delete,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        _ if key_part.len() == 1 => {
            let c = key_part.chars().next().unwrap();
            KeyCode::Char(c)
        }
        _ => return None,
    };

    Some(KeyEvent::new(code, modifiers))
}

/// Resolved keybindings (parsed KeyEvents for fast comparison).
#[derive(Debug, Clone)]
pub struct ResolvedKeys {
    pub quit: KeyEvent,
    pub zen_mode: KeyEvent,
    pub search: KeyEvent,
    pub command_palette: KeyEvent,
    pub daily_note: KeyEvent,
    pub task_board: KeyEvent,
    pub escape: KeyEvent,
    pub enter: KeyEvent,
    pub backspace: KeyEvent,
    pub move_up: KeyEvent,
    pub move_down: KeyEvent,
    pub move_left: KeyEvent,
    pub move_up_alt: KeyEvent,
    pub move_down_alt: KeyEvent,
    pub move_left_alt: KeyEvent,
    pub delete: KeyEvent,
    pub list_create_note: KeyEvent,
    pub list_create_dir: KeyEvent,
    pub list_tag_explorer: KeyEvent,
    pub list_rename: KeyEvent,
    pub list_edit_config: KeyEvent,
    pub list_delete: KeyEvent,
    pub list_parent: KeyEvent,
    pub list_parent_alt: KeyEvent,
    pub editor_back: KeyEvent,
    pub editor_pdf: KeyEvent,
    pub editor_backlinks: KeyEvent,
    pub editor_wiki_link: KeyEvent,
    pub editor_insert: KeyEvent,
    pub editor_append: KeyEvent,
    pub editor_split_focus: KeyEvent,
}

impl ResolvedKeys {
    pub fn from_config(keys: &KeysConfig) -> Self {
        fn parse_or(s: &str, default: KeyEvent) -> KeyEvent {
            parse_key_event(s).unwrap_or(default)
        }
        let def_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let def_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let def_backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        let def_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty());
        let def_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        let def_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty());
        let def_left = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
        let def_del = KeyEvent::new(KeyCode::Delete, KeyModifiers::empty());

        Self {
            quit: parse_or(&keys.quit, KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty())),
            zen_mode: parse_or(&keys.zen_mode, KeyEvent::new(KeyCode::F(11), KeyModifiers::empty())),
            search: parse_or(&keys.search, KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty())),
            command_palette: parse_or(&keys.command_palette, KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)),
            daily_note: parse_or(&keys.daily_note, KeyEvent::new(KeyCode::Char('d'), KeyModifiers::ALT)),
            task_board: parse_or(&keys.task_board, KeyEvent::new(KeyCode::Char('t'), KeyModifiers::ALT)),
            escape: parse_or(&keys.escape, def_esc),
            enter: parse_or(&keys.enter, def_enter),
            backspace: parse_or(&keys.backspace, def_backspace),
            move_up: parse_or(&keys.move_up, def_k),
            move_down: parse_or(&keys.move_down, def_j),
            move_left: parse_or(&keys.move_left, def_h),
            move_up_alt: parse_or(&keys.move_up_alt, KeyEvent::new(KeyCode::Up, KeyModifiers::empty())),
            move_down_alt: parse_or(&keys.move_down_alt, KeyEvent::new(KeyCode::Down, KeyModifiers::empty())),
            move_left_alt: parse_or(&keys.move_left_alt, def_left),
            delete: parse_or(&keys.delete, def_del),
            list_create_note: parse_or(&keys.list_create_note, KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty())),
            list_create_dir: parse_or(&keys.list_create_dir, KeyEvent::new(KeyCode::Char('n'), KeyModifiers::SHIFT)),
            list_tag_explorer: parse_or(&keys.list_tag_explorer, KeyEvent::new(KeyCode::Char('t'), KeyModifiers::SHIFT)),
            list_rename: parse_or(&keys.list_rename, KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty())),
            list_edit_config: parse_or(&keys.list_edit_config, KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty())),
            list_delete: parse_or(&keys.list_delete, KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty())),
            list_parent: parse_or(&keys.list_parent, def_backspace),
            list_parent_alt: parse_or(&keys.list_parent_alt, def_left),
            editor_back: parse_or(&keys.editor_back, KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty())),
            editor_pdf: parse_or(&keys.editor_pdf, KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL)),
            editor_backlinks: parse_or(&keys.editor_backlinks, KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)),
            editor_wiki_link: parse_or(&keys.editor_wiki_link, KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL)),
            editor_insert: parse_or(&keys.editor_insert, KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty())),
            editor_append: parse_or(&keys.editor_append, KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty())),
            editor_split_focus: parse_or(&keys.editor_split_focus, KeyEvent::new(KeyCode::Tab, KeyModifiers::empty())),
        }
    }
}

/// Formats a key config string for display (e.g. "ctrl-q" -> "Ctrl+Q").
pub fn key_display_string(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = s.split('-').collect();
    let (mods, key_part) = if parts.len() >= 2 {
        let mod_str: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|p| match *p {
                "ctrl" => "Ctrl",
                "alt" => "Alt",
                "shift" => "Shift",
                _ => *p,
            })
            .map(|s| s.to_string())
            .collect();
        (mod_str.join("+"), parts[parts.len() - 1])
    } else {
        (String::new(), parts[0])
    };

    let key_display = match key_part.to_lowercase().as_str() {
        "enter" => "Enter".to_string(),
        "esc" | "escape" => "Esc".to_string(),
        "backspace" => "Backspace".to_string(),
        "tab" => "Tab".to_string(),
        "delete" => "Delete".to_string(),
        "space" => "Space".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        s if matches!(s, "f1" | "f2" | "f3" | "f4" | "f5" | "f6" | "f7" | "f8" | "f9" | "f10" | "f11" | "f12") => key_part.to_uppercase(),
        _ if key_part.len() == 1 => key_part.to_uppercase(),
        _ => key_part.to_string(),
    };

    if mods.is_empty() {
        key_display
    } else {
        format!("{}+{}", mods, key_display)
    }
}

/// Application logic configuration loaded from config.toml.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory where markdown notes are stored.
    pub notes_directory: String,
    /// Folder for daily notes (journal), relative to notes_directory.
    pub daily_notes_folder: String,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub keys: KeysConfig,
}

impl Default for Config {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        Self {
            notes_directory: format!("{}/Documents/Notes", home),
            daily_notes_folder: "journal".to_string(),
            theme: ThemeConfig::default(),
            editor: EditorConfig::default(),
            ui: UiConfig::default(),
            keys: KeysConfig::default(),
        }
    }
}

/// Returns the path to config.toml.
pub fn config_file_path() -> Result<PathBuf> {
    let dir = ensure_config_dir()?;
    Ok(dir.join("config.toml"))
}

/// Returns the Oxid config directory (~/.config/oxid).
/// Creates it if it does not exist.
pub fn ensure_config_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", "oxid")
        .context("Could not determine XDG config directory")?;
    let config_dir = dirs.config_dir().to_path_buf();
    fs::create_dir_all(&config_dir)
        .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;
    Ok(config_dir)
}

/// Load config from ~/.config/oxid/config.toml.
/// Creates default config file if missing.
pub fn load_config() -> Result<Config> {
    let config_dir = ensure_config_dir()?;
    let config_path = config_dir.join("config.toml");

    let config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", config_path.display()))?
    } else {
        let default = Config::default();
        let content = generate_default_config(&default);
        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write default config: {}", config_path.display()))?;
        default
    };

    Ok(config)
}

fn generate_default_config(config: &Config) -> String {
    let k = &config.keys;
    let t = &config.theme;
    let u = &config.ui;
    format!(
        r#"# Oxid Configuration
# Logic settings for the note manager

# Directory where your markdown notes are stored
notes_directory = "{}"

# Folder for daily notes (relative to notes_directory)
daily_notes_folder = "{}"

[theme]
# Hex (#RRGGBB) or named colors. Override theme.toml for main editor/status bar/borders.
background = "{}"
foreground = "{}"
cursor = "{}"
selection = "{}"
statusbar_bg = "{}"
statusbar_fg = "{}"
border_color = "{}"

[editor]
typewriter_mode = false
enable_spellcheck = false
spellcheck_languages = ["en"]
show_backlinks = true
syntax_highlighting = true
auto_save = true
auto_save_interval = 30
line_numbers = true
rel_line_numbers = false
tab_width = 4
mouse_support = true

[ui]
# Border style: "rounded", "double", "thick", "plain"
border_style = "{}"
icons = {}
show_hidden = {}

[keys]
# Global
quit = "{}"
zen_mode = "{}"
search = "{}"
command_palette = "{}"
daily_note = "{}"
task_board = "{}"
# Generic
escape = "{}"
enter = "{}"
backspace = "{}"
move_up = "{}"
move_down = "{}"
move_left = "{}"
move_up_alt = "{}"
move_down_alt = "{}"
move_left_alt = "{}"
delete = "{}"
# List (file explorer)
list_create_note = "{}"
list_create_dir = "{}"
list_tag_explorer = "{}"
list_rename = "{}"
list_edit_config = "{}"
list_delete = "{}"
list_parent = "{}"
list_parent_alt = "{}"
# Editor
editor_back = "{}"
editor_pdf = "{}"
editor_backlinks = "{}"
editor_wiki_link = "{}"
editor_insert = "{}"
editor_append = "{}"
editor_split_focus = "{}"
"#,
        config.notes_directory,
        config.daily_notes_folder,
        t.background,
        t.foreground,
        t.cursor,
        t.selection,
        t.statusbar_bg,
        t.statusbar_fg,
        t.border_color,
        u.border_style,
        u.icons,
        u.show_hidden,
        k.quit,
        k.zen_mode,
        k.search,
        k.command_palette,
        k.daily_note,
        k.task_board,
        k.escape,
        k.enter,
        k.backspace,
        k.move_up,
        k.move_down,
        k.move_left,
        k.move_up_alt,
        k.move_down_alt,
        k.move_left_alt,
        k.delete,
        k.list_create_note,
        k.list_create_dir,
        k.list_tag_explorer,
        k.list_rename,
        k.list_edit_config,
        k.list_delete,
        k.list_parent,
        k.list_parent_alt,
        k.editor_back,
        k.editor_pdf,
        k.editor_backlinks,
        k.editor_wiki_link,
        k.editor_insert,
        k.editor_append,
        k.editor_split_focus,
    )
}

/// Resolves ~ in paths to the user's home directory.
pub fn expand_path(path: &str) -> PathBuf {
    let path = path.trim();
    if path.starts_with("~/") || path == "~" {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        PathBuf::from(path.replacen('~', &home, 1))
    } else {
        PathBuf::from(path)
    }
}
