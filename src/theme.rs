// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

use anyhow::{Context, Result};
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::str::FromStr;

fn def(s: &str) -> ColorDef {
    ColorDef(s.to_string())
}

/// Visual theme configuration loaded from theme.toml.
/// Every visible color in the TUI is configurable.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub app_background: ColorDef,
    pub border: ColorDef,
    pub header: ColorDef,
    pub highlight: ColorDef,
    pub text: ColorDef,
    #[serde(rename = "list_border_active")]
    pub list_border_active: ColorDef,
    #[serde(rename = "list_border_inactive")]
    pub list_border_inactive: ColorDef,
    #[serde(rename = "list_text_selected_fg")]
    pub list_text_selected_fg: ColorDef,
    #[serde(rename = "list_text_selected_bg")]
    pub list_text_selected_bg: ColorDef,
    #[serde(rename = "list_text_normal")]
    pub list_text_normal: ColorDef,
    #[serde(rename = "preview_border_active")]
    pub preview_border_active: ColorDef,
    #[serde(rename = "preview_border_inactive")]
    pub preview_border_inactive: ColorDef,
    #[serde(rename = "preview_text")]
    pub preview_text: ColorDef,
    #[serde(rename = "search_match")]
    pub search_match: ColorDef,
    #[serde(rename = "help_text")]
    pub help_text: ColorDef,
    #[serde(rename = "editor_bg")]
    pub editor_bg: ColorDef,
    #[serde(rename = "editor_fg")]
    pub editor_fg: ColorDef,
    #[serde(rename = "editor_cursor")]
    pub editor_cursor: ColorDef,
    #[serde(rename = "editor_line_number")]
    pub editor_line_number: ColorDef,
    #[serde(rename = "md_header_fg")]
    pub md_header_fg: ColorDef,
    #[serde(rename = "md_code_bg")]
    pub md_code_bg: ColorDef,
    #[serde(rename = "md_list_marker")]
    pub md_list_marker: ColorDef,
    #[serde(rename = "editor_header")]
    pub editor_header: ColorDef,
    #[serde(rename = "editor_list")]
    pub editor_list: ColorDef,
    #[serde(rename = "editor_checkbox")]
    pub editor_checkbox: ColorDef,
    #[serde(rename = "editor_checkbox_checked")]
    pub editor_checkbox_checked: ColorDef,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            app_background: def("black"),
            border: def("cyan"),
            header: def("yellow"),
            highlight: def("magenta"),
            text: def("white"),
            list_border_active: def("cyan"),
            list_border_inactive: def("dark_gray"),
            list_text_selected_fg: def("green"),
            list_text_selected_bg: def("black"),
            list_text_normal: def("white"),
            preview_border_active: def("blue"),
            preview_border_inactive: def("dark_gray"),
            preview_text: def("white"),
            search_match: def("red"),
            help_text: def("white"),
            editor_bg: def("black"),
            editor_fg: def("white"),
            editor_cursor: def("cyan"),
            editor_line_number: def("dark_gray"),
            md_header_fg: def("yellow"),
            md_code_bg: def("dark_gray"),
            md_list_marker: def("cyan"),
            editor_header: def("blue"),
            editor_list: def("yellow"),
            editor_checkbox: def("yellow"),
            editor_checkbox_checked: def("green"),
        }
    }
}

/// Color definition supporting hex (#RRGGBB) or named colors.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct ColorDef(String);

impl ColorDef {
    pub fn to_ratatui_color(&self) -> Result<Color> {
        let s = self.0.trim();
        // Map common invalid names to valid ratatui colors (ratatui has no "orange")
        let normalized = match s.to_lowercase().as_str() {
            "orange1" | "orange" => "yellow",
            "orange2" | "dark_orange" => "dark_gray",
            _ => s,
        };
        Color::from_str(normalized).with_context(|| format!("Invalid color: {}", self.0))
    }
}

/// Load theme from ~/.config/oxid/theme.toml.
pub fn load_theme(config_dir: &Path) -> Result<Theme> {
    let theme_path = config_dir.join("theme.toml");

    let theme = if theme_path.exists() {
        let content = fs::read_to_string(&theme_path)
            .with_context(|| format!("Failed to read theme: {}", theme_path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse theme: {}", theme_path.display()))?
    } else {
        let default = Theme::default();
        let content = generate_default_theme(&default);
        fs::write(&theme_path, content)
            .with_context(|| format!("Failed to write default theme: {}", theme_path.display()))?;
        default
    };

    Ok(theme)
}

fn generate_default_theme(theme: &Theme) -> String {
    fn cv(c: &ColorDef) -> String {
        format!("\"{}\"", c.0)
    }
    format!(
        r#"# Oxid Theme Configuration
# Every visible color is configurable. Hex (#RRGGBB) or named colors.

app_background = {}
border = {}
header = {}
highlight = {}
text = {}

# Notes list
list_border_active = {}
list_border_inactive = {}
list_text_selected_fg = {}
list_text_selected_bg = {}
list_text_normal = {}

# Preview pane
preview_border_active = {}
preview_border_inactive = {}
preview_text = {}

# Search highlighting (list + preview)
search_match = {}

# Footer / help
help_text = {}

# Editor pane
editor_bg = {}
editor_fg = {}
editor_cursor = {}
editor_line_number = {}

# Markdown preview (headers, code blocks, list markers)
md_header_fg = {}
md_code_bg = {}
md_list_marker = {}

# Editor syntax highlighting
editor_header = {}
editor_list = {}
editor_checkbox = {}
editor_checkbox_checked = {}
"#,
        cv(&theme.app_background),
        cv(&theme.border),
        cv(&theme.header),
        cv(&theme.highlight),
        cv(&theme.text),
        cv(&theme.list_border_active),
        cv(&theme.list_border_inactive),
        cv(&theme.list_text_selected_fg),
        cv(&theme.list_text_selected_bg),
        cv(&theme.list_text_normal),
        cv(&theme.preview_border_active),
        cv(&theme.preview_border_inactive),
        cv(&theme.preview_text),
        cv(&theme.search_match),
        cv(&theme.help_text),
        cv(&theme.editor_bg),
        cv(&theme.editor_fg),
        cv(&theme.editor_cursor),
        cv(&theme.editor_line_number),
        cv(&theme.md_header_fg),
        cv(&theme.md_code_bg),
        cv(&theme.md_list_marker),
        cv(&theme.editor_header),
        cv(&theme.editor_list),
        cv(&theme.editor_checkbox),
        cv(&theme.editor_checkbox_checked),
    )
}

/// Resolved theme with Ratatui Style objects. No hardcoded colors.
#[derive(Clone)]
pub struct ResolvedTheme {
    pub app_background_style: Style,
    pub border_style: Style,
    pub header_style: Style,
    pub highlight_style: Style,
    pub text_style: Style,
    pub list_border_active_style: Style,
    pub list_border_inactive_style: Style,
    pub list_text_selected_style: Style,
    pub list_text_normal_style: Style,
    pub preview_border_active_style: Style,
    pub preview_border_inactive_style: Style,
    pub preview_text_style: Style,
    pub search_match_style: Style,
    pub help_text_style: Style,
    pub editor_bg_style: Style,
    pub editor_fg_style: Style,
    pub editor_cursor_style: Style,
    pub editor_line_number_style: Style,
    pub md_header_fg_style: Style,
    pub md_code_bg_style: Style,
    pub md_list_marker_style: Style,
    pub editor_header_style: Style,
    pub editor_list_style: Style,
    pub editor_checkbox_style: Style,
    pub editor_checkbox_checked_style: Style,
}

impl ResolvedTheme {
    pub fn from_theme(theme: &Theme) -> Result<Self> {
        Ok(Self {
            app_background_style: Style::default().bg(theme.app_background.to_ratatui_color()?),
            border_style: Style::default().fg(theme.border.to_ratatui_color()?),
            header_style: Style::default()
                .fg(theme.header.to_ratatui_color()?)
                .add_modifier(Modifier::BOLD),
            highlight_style: Style::default().fg(theme.highlight.to_ratatui_color()?),
            text_style: Style::default().fg(theme.text.to_ratatui_color()?),
            list_border_active_style: Style::default()
                .fg(theme.list_border_active.to_ratatui_color()?),
            list_border_inactive_style: Style::default()
                .fg(theme.list_border_inactive.to_ratatui_color()?),
            list_text_selected_style: Style::default()
                .fg(theme.list_text_selected_fg.to_ratatui_color()?)
                .bg(theme.list_text_selected_bg.to_ratatui_color()?)
                .add_modifier(Modifier::BOLD),
            list_text_normal_style: Style::default()
                .fg(theme.list_text_normal.to_ratatui_color()?),
            preview_border_active_style: Style::default()
                .fg(theme.preview_border_active.to_ratatui_color()?),
            preview_border_inactive_style: Style::default()
                .fg(theme.preview_border_inactive.to_ratatui_color()?),
            preview_text_style: Style::default().fg(theme.preview_text.to_ratatui_color()?),
            search_match_style: Style::default()
                .fg(theme.search_match.to_ratatui_color()?)
                .add_modifier(Modifier::BOLD),
            help_text_style: Style::default().fg(theme.help_text.to_ratatui_color()?),
            editor_bg_style: Style::default().bg(theme.editor_bg.to_ratatui_color()?),
            editor_fg_style: Style::default().fg(theme.editor_fg.to_ratatui_color()?),
            editor_cursor_style: Style::default()
                .fg(theme.editor_cursor.to_ratatui_color()?)
                .add_modifier(Modifier::REVERSED),
            editor_line_number_style: Style::default()
                .fg(theme.editor_line_number.to_ratatui_color()?),
            md_header_fg_style: Style::default()
                .fg(theme.md_header_fg.to_ratatui_color()?)
                .add_modifier(Modifier::BOLD),
            md_code_bg_style: Style::default().bg(theme.md_code_bg.to_ratatui_color()?),
            md_list_marker_style: Style::default()
                .fg(theme.md_list_marker.to_ratatui_color()?),
            editor_header_style: Style::default()
                .fg(theme.editor_header.to_ratatui_color()?),
            editor_list_style: Style::default()
                .fg(theme.editor_list.to_ratatui_color()?),
            editor_checkbox_style: Style::default()
                .fg(theme.editor_checkbox.to_ratatui_color()?)
                .add_modifier(Modifier::BOLD),
            editor_checkbox_checked_style: Style::default()
                .fg(theme.editor_checkbox_checked.to_ratatui_color()?)
                .add_modifier(Modifier::CROSSED_OUT),
        })
    }
}
