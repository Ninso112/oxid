// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Editor-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub typewriter_mode: bool,
    pub enable_spellcheck: bool,
    pub spellcheck_languages: Vec<String>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            typewriter_mode: false,
            enable_spellcheck: false,
            spellcheck_languages: vec!["en".to_string()],
        }
    }
}

/// Keybindings configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct KeysConfig {
    /// Search trigger key. "/" for telescope search.
    pub search: String,
    /// PDF export hotkey. "ctrl+e" etc.
    pub pdf_export: String,
}

impl Default for KeysConfig {
    fn default() -> Self {
        Self {
            search: "/".to_string(),
            pdf_export: "ctrl+e".to_string(),
        }
    }
}

/// Application logic configuration loaded from config.toml.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directory where markdown notes are stored.
    pub notes_directory: String,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub keys: KeysConfig,
}

impl Default for Config {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        Self {
            notes_directory: format!("{}/Documents/Notes", home),
            editor: EditorConfig::default(),
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
    format!(
        r#"# Oxid Configuration
# Logic settings for the note manager

# Directory where your markdown notes are stored
notes_directory = "{}"

[editor]
typewriter_mode = false
enable_spellcheck = false
spellcheck_languages = ["en"]

[keys]
search = "/"
pdf_export = "ctrl+e"
"#,
        config.notes_directory
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
