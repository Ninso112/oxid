// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

use crate::config::{expand_path, key_display_string, load_config, Config, ResolvedKeys};
use crate::handlers::key_matches;
use crate::git::{get_git_status, GitStatus};
use crate::search::{filter_notes, get_match_indices};
use crate::spellcheck::Spellchecker;
use crate::telescope::{
    filter_telescope_notes, find_md_files_recursive, get_telescope_match_indices,
};
use crate::templates::Template;
use crate::theme::{load_theme, ResolvedTheme};
use anyhow::Result;
use chrono::Local;
use nucleo_matcher::{Config as MatcherConfig, Matcher};
use regex::Regex;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use tui_textarea::{CursorMove, Scrolling, TextArea};
use walkdir::WalkDir;

/// Maximum bytes to read from a note file for indexing and preview.
const MAX_CONTENT_BYTES: usize = 100_000;

/// Layout mode for editor panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorLayout {
    Single,
    SplitVertical,
}

/// Which pane or popup has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Editor,
    /// Telescope-style fuzzy search (/).
    Search,
    /// Command palette (Ctrl+p).
    CommandPalette,
    /// Rename file popup (r).
    Rename,
    /// Create directory popup (Shift+n).
    CreatingDirectory,
    /// Backlinks panel.
    Backlinks,
    /// Tag explorer view.
    TagExplorer,
    /// Global task board (unchecked tasks).
    TaskView,
}

/// Single editor buffer (tab).
#[derive(Clone)]
pub struct EditorBuffer {
    pub path: Option<PathBuf>,
    pub textarea: TextArea<'static>,
}

impl EditorBuffer {
    pub fn new(path: Option<PathBuf>, lines: Vec<String>) -> Self {
        let textarea = if lines.is_empty() {
            TextArea::default()
        } else {
            TextArea::new(lines)
        };
        Self { path, textarea }
    }

    pub fn display_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string()
    }
}

/// Vim-like editor mode when Focus::Editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
}

/// Application mode (when Focus::List).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Create,
}

/// Represents a file or directory in the explorer.
#[derive(Clone, Debug)]
pub struct NoteEntry {
    pub path: PathBuf,
    pub display: String,
    pub content: String,
    pub(crate) searchable: String,
    pub is_directory: bool,
}

impl NoteEntry {
    pub fn new(path: PathBuf, display: String, content: String, searchable: String) -> Self {
        Self {
            path,
            display,
            content,
            searchable,
            is_directory: false,
        }
    }

    pub fn dir(path: PathBuf, display: String) -> Self {
        let searchable = display.clone();
        Self {
            path,
            display,
            content: String::new(),
            searchable,
            is_directory: true,
        }
    }
}

impl AsRef<str> for NoteEntry {
    fn as_ref(&self) -> &str {
        &self.searchable
    }
}

/// Unchecked task from a markdown file (`- [ ] ...`).
#[derive(Clone, Debug)]
pub struct TaskEntry {
    pub path: PathBuf,
    pub line_number: usize,
    pub content: String,
}

/// Command palette action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    RenameFile,
    DeleteFile,
    InsertDate,
    ToggleZenMode,
    ToggleSplitView,
    GitPush,
    ExportPdf,
}

impl CommandAction {
    pub fn label(&self) -> &'static str {
        match self {
            CommandAction::RenameFile => "Rename File",
            CommandAction::DeleteFile => "Delete File",
            CommandAction::InsertDate => "Insert Date",
            CommandAction::ToggleZenMode => "Toggle Zen Mode",
            CommandAction::ToggleSplitView => "Toggle Split View",
            CommandAction::GitPush => "Git Push",
            CommandAction::ExportPdf => "Export to PDF",
        }
    }

    pub fn all() -> &'static [CommandAction] {
        &[
            CommandAction::RenameFile,
            CommandAction::DeleteFile,
            CommandAction::InsertDate,
            CommandAction::ToggleZenMode,
            CommandAction::ToggleSplitView,
            CommandAction::GitPush,
            CommandAction::ExportPdf,
        ]
    }
}

/// Main application state.
pub struct App {
    pub config: Config,
    pub resolved_keys: ResolvedKeys,
    pub theme: ResolvedTheme,
    pub notes_dir: PathBuf,
    /// Directory currently being browsed in the file explorer.
    pub current_dir: PathBuf,
    pub all_notes: Vec<NoteEntry>,
    pub filtered_notes: Vec<NoteEntry>,
    pub selected: usize,
    pub mode: Mode,
    pub search_query: String,
    pub create_filename: String,
    pub message: Option<String>,
    matcher: Matcher,
    pub match_indices: Vec<Vec<u32>>,

    // Focus and editor state
    pub focus: Focus,
    pub editor_mode: EditorMode,
    /// Open buffers (tabs).
    pub buffers: Vec<EditorBuffer>,
    /// Active tab index.
    pub active_tab: usize,
    /// Split view: right pane shows this tab.
    pub split_right_tab: Option<usize>,
    /// Which pane receives input when split.
    pub split_focus_left: bool,
    pub editor_layout: EditorLayout,

    // Zen mode
    pub zen_mode: bool,

    // Telescope (/)
    pub telescope_notes: Vec<NoteEntry>,
    pub telescope_filtered: Vec<NoteEntry>,
    pub telescope_query: String,
    pub telescope_selected: usize,
    pub telescope_match_indices: Vec<Vec<u32>>,
    telescope_matcher: Matcher,

    // Command palette
    pub command_palette_query: String,
    pub command_palette_filtered: Vec<CommandAction>,
    pub command_palette_selected: usize,

    // Rename popup
    pub rename_input: String,

    // Create directory popup (Shift+n)
    pub directory_input: String,

    // Template picker for new files
    pub template_picker_active: bool,
    pub template_picker_selected: usize,

    // Spellchecker (lazy-loaded)
    pub spellchecker: Option<Spellchecker>,

    // g-pending for gt/gT tab switch
    pub g_pending: bool,

    // Backlinks
    pub backlinks: Vec<PathBuf>,
    pub backlinks_selected: usize,

    // Tag Explorer
    pub tag_explorer_active: bool,
    pub all_tags: Vec<String>,
    pub tag_selected: usize,
    pub tag_files: Vec<PathBuf>,
    pub tag_file_selected: usize,
    pub tag_explorer_view: TagExplorerView,

    // Auto-save
    pub last_keystroke_time: Option<Instant>,
    pub editor_dirty: bool,
    pub save_indicator_until: Option<Instant>,

    // Global Task Board
    pub task_view_active: bool,
    pub tasks: Vec<TaskEntry>,
    pub task_selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TagExplorerView {
    TagList,
    FileList,
}

impl App {
    /// Index of the buffer that receives input.
    pub fn focused_buffer_index(&self) -> usize {
        if self.editor_layout == EditorLayout::SplitVertical && self.split_right_tab.is_some() {
            if self.split_focus_left {
                self.active_tab
            } else {
                self.split_right_tab.unwrap_or(0).min(self.buffers.len().saturating_sub(1))
            }
        } else {
            self.active_tab
        }
    }

    pub fn focused_buffer_mut(&mut self) -> Option<&mut EditorBuffer> {
        let idx = self.focused_buffer_index();
        self.buffers.get_mut(idx)
    }

    pub fn focused_buffer(&self) -> Option<&EditorBuffer> {
        let idx = self.focused_buffer_index();
        self.buffers.get(idx)
    }

    pub fn editing_path(&self) -> Option<PathBuf> {
        self.focused_buffer()?.path.clone()
    }

    pub fn has_open_buffers(&self) -> bool {
        !self.buffers.is_empty()
    }

    /// Returns the display string for a keybinding action (e.g. "quit" -> "Q").
    pub fn get_key_display_string(&self, action_name: &str) -> String {
        let s = match action_name {
            "quit" => &self.config.keys.quit,
            "zen_mode" => &self.config.keys.zen_mode,
            "search" => &self.config.keys.search,
            "command_palette" => &self.config.keys.command_palette,
            "daily_note" => &self.config.keys.daily_note,
            "task_board" => &self.config.keys.task_board,
            "escape" => &self.config.keys.escape,
            "enter" => &self.config.keys.enter,
            "backspace" => &self.config.keys.backspace,
            "move_up" => &self.config.keys.move_up,
            "move_down" => &self.config.keys.move_down,
            "move_left" => &self.config.keys.move_left,
            "delete" => &self.config.keys.delete,
            "list_create_note" => &self.config.keys.list_create_note,
            "list_create_dir" => &self.config.keys.list_create_dir,
            "list_tag_explorer" => &self.config.keys.list_tag_explorer,
            "list_rename" => &self.config.keys.list_rename,
            "list_edit_config" => &self.config.keys.list_edit_config,
            "list_delete" => &self.config.keys.list_delete,
            "list_parent" => &self.config.keys.list_parent,
            "list_parent_alt" => &self.config.keys.list_parent_alt,
            "editor_back" => &self.config.keys.editor_back,
            "editor_pdf" => &self.config.keys.editor_pdf,
            "editor_backlinks" => &self.config.keys.editor_backlinks,
            "editor_wiki_link" => &self.config.keys.editor_wiki_link,
            "editor_insert" => &self.config.keys.editor_insert,
            "editor_append" => &self.config.keys.editor_append,
            "editor_split_focus" => &self.config.keys.editor_split_focus,
            "move_up_alt" => &self.config.keys.move_up_alt,
            "move_down_alt" => &self.config.keys.move_down_alt,
            "move_left_alt" => &self.config.keys.move_left_alt,
            _ => return String::new(),
        };
        key_display_string(s)
    }

    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let config_dir = crate::config::ensure_config_dir()?;
        let theme_raw = load_theme(&config_dir)?;
        let theme = ResolvedTheme::resolve(&theme_raw, Some(&config.theme))?;
        let notes_dir = expand_path(&config.notes_directory);

        fs::create_dir_all(&notes_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create notes directory: {}", e))?;

        let current_dir = notes_dir.clone();
        let all_notes = load_entries(&current_dir)?;
        let filtered_notes = all_notes.clone();
        let match_indices = vec![Vec::new(); filtered_notes.len()];
        let matcher = Matcher::new(MatcherConfig::DEFAULT.match_paths());

        let mut buf = EditorBuffer::new(None, vec![String::new()]);
        buf.textarea.set_max_histories(50);
        let buffers = vec![buf];

        let spellchecker = if config.editor.enable_spellcheck && !config.editor.spellcheck_languages.is_empty() {
            Some(Spellchecker::new(&config.editor.spellcheck_languages))
        } else {
            None
        };

        let resolved_keys = ResolvedKeys::from_config(&config.keys);
        let mut app = Self {
            config,
            resolved_keys,
            theme,
            notes_dir,
            current_dir,
            all_notes,
            filtered_notes,
            selected: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            create_filename: String::new(),
            message: None,
            matcher,
            match_indices,
            focus: Focus::List,
            editor_mode: EditorMode::Normal,
            buffers,
            active_tab: 0,
            split_right_tab: None,
            split_focus_left: true,
            editor_layout: EditorLayout::Single,
            zen_mode: false,
            telescope_notes: Vec::new(),
            telescope_filtered: Vec::new(),
            telescope_query: String::new(),
            telescope_selected: 0,
            telescope_match_indices: Vec::new(),
            telescope_matcher: Matcher::new(MatcherConfig::DEFAULT.match_paths()),
            command_palette_query: String::new(),
            command_palette_filtered: CommandAction::all().to_vec(),
            command_palette_selected: 0,
            rename_input: String::new(),
            directory_input: String::new(),
            template_picker_active: false,
            template_picker_selected: 0,
            spellchecker,
            g_pending: false,
            backlinks: Vec::new(),
            backlinks_selected: 0,
            tag_explorer_active: false,
            all_tags: Vec::new(),
            tag_selected: 0,
            tag_files: Vec::new(),
            tag_file_selected: 0,
            tag_explorer_view: TagExplorerView::TagList,
            last_keystroke_time: None,
            editor_dirty: false,
            save_indicator_until: None,
            task_view_active: false,
            tasks: Vec::new(),
            task_selected: 0,
        };
        app.apply_editor_theme_to_all();
        Ok(app)
    }

    pub fn refresh_notes(&mut self) -> Result<()> {
        self.all_notes = load_entries(&self.current_dir)?;
        if !self.config.ui.show_hidden {
            self.all_notes
                .retain(|e| !e.display.starts_with('.'));
        }
        self.apply_filter();
        self.clamp_selection();
        Ok(())
    }

    /// Returns Nerd Font icon for path/extension when config.ui.icons is true, else empty string.
    pub fn file_icon(&self, path: &std::path::Path) -> &'static str {
        if !self.config.ui.icons {
            return "";
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        match ext.to_lowercase().as_str() {
            "md" | "markdown" => "\u{f48a} ",   // md
            "rs" => "\u{e79b} ",                 // rust
            "toml" | "yaml" | "yml" => "\u{f718} ", // config
            "json" => "\u{e60b} ",               // json
            "txt" => "\u{f15c} ",                // text
            "pdf" => "\u{f1c1} ",                // pdf
            "png" | "jpg" | "jpeg" | "gif" | "svg" => "\u{f1c5} ", // image
            _ if path.is_dir() => "\u{f115} ",   // folder
            _ => "\u{f016} ",                    // file
        }
    }

    /// Enter the selected directory. Returns true if we navigated.
    pub fn enter_selected_directory(&mut self) -> bool {
        let entry = match self.filtered_notes.get(self.selected) {
            Some(e) if e.is_directory => e,
            _ => return false,
        };
        match fs::metadata(&entry.path) {
            Ok(m) if m.is_dir() => {}
            _ => return false,
        }
        self.current_dir = entry.path.clone();
        if let Err(e) = self.refresh_notes() {
            self.message = Some(format!("Cannot read directory: {}", e));
        }
        true
    }

    /// Go to parent directory. Returns true if we navigated. Never goes above notes_dir.
    pub fn go_to_parent_dir(&mut self) -> bool {
        if self.current_dir == self.notes_dir {
            return false;
        }
        let parent = match self.current_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return false,
        };
        if !parent.starts_with(&self.notes_dir) {
            return false;
        }
        let prev_folder_name = self
            .current_dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| format!("{}/", s));
        self.current_dir = parent;
        if let Err(e) = self.refresh_notes() {
            self.message = Some(format!("Cannot read directory: {}", e));
            return true;
        }
        if let Some(name) = prev_folder_name {
            if let Some(idx) = self.filtered_notes.iter().position(|e| e.display == name) {
                self.selected = idx;
            }
        }
        true
    }

    /// Check if we can go up (not at notes root).
    #[allow(dead_code)]
    pub fn can_go_up(&self) -> bool {
        self.current_dir != self.notes_dir
    }

    fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_notes = self.all_notes.clone();
            self.match_indices = vec![Vec::new(); self.filtered_notes.len()];
        } else {
            self.filtered_notes =
                filter_notes(&self.all_notes, &self.search_query, &mut self.matcher);
            self.match_indices = self
                .filtered_notes
                .iter()
                .map(|n| get_match_indices(&n.display, &self.search_query, &mut self.matcher))
                .collect();
        }
    }

    fn clamp_selection(&mut self) {
        if self.filtered_notes.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_notes.len() {
            self.selected = self.filtered_notes.len() - 1;
        }
    }

    pub fn move_selection_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_selection_down(&mut self) {
        if self.selected + 1 < self.filtered_notes.len() {
            self.selected += 1;
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.mode = Mode::Search;
        self.search_query.clear();
    }

    pub fn exit_search_mode(&mut self) {
        self.mode = Mode::Normal;
        self.search_query.clear();
        self.apply_filter();
        self.clamp_selection();
    }

    pub fn search_add_char(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_filter();
        self.selected = 0;
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.apply_filter();
        self.clamp_selection();
    }

    pub fn enter_create_mode(&mut self) {
        self.mode = Mode::Create;
        self.create_filename.clear();
    }

    pub fn exit_create_mode(&mut self) {
        self.mode = Mode::Normal;
        self.create_filename.clear();
    }

    pub fn create_add_char(&mut self, c: char) {
        self.create_filename.push(c);
    }

    pub fn create_backspace(&mut self) {
        self.create_filename.pop();
    }

    pub fn get_selected_path(&self) -> Option<PathBuf> {
        self.filtered_notes.get(self.selected).map(|n| n.path.clone())
    }

    /// Get preview content: from textarea when editing, else from selected note.
    pub fn get_preview_content(&self) -> String {
        if self.focus == Focus::Editor {
            if let Some(buf) = self.focused_buffer() {
                return buf.textarea.lines().join("\n");
            }
        }
        if let Some(note) = self.filtered_notes.get(self.selected) {
            note.content.clone()
        } else {
            String::new()
        }
    }

    pub fn get_preview_placeholder(&self) -> Option<&str> {
        if self.focus == Focus::Editor {
            return None;
        }
        let note = self.filtered_notes.get(self.selected)?;
        if note.content.is_empty() && !note.display.is_empty() {
            Some("(Preview unavailable: file unreadable)")
        } else {
            None
        }
    }

    pub fn reload_config(&mut self) -> Result<()> {
        self.config = load_config()?;
        self.resolved_keys = ResolvedKeys::from_config(&self.config.keys);
        let config_dir = crate::config::ensure_config_dir()?;
        let theme_raw = load_theme(&config_dir)?;
        self.theme = ResolvedTheme::resolve(&theme_raw, Some(&self.config.theme))?;
        self.notes_dir = expand_path(&self.config.notes_directory);
        if !self.current_dir.starts_with(&self.notes_dir) {
            self.current_dir = self.notes_dir.clone();
        }
        self.apply_editor_theme_to_all();
        self.spellchecker = if self.config.editor.enable_spellcheck
            && !self.config.editor.spellcheck_languages.is_empty()
        {
            Some(Spellchecker::new(&self.config.editor.spellcheck_languages))
        } else {
            None
        };
        Ok(())
    }

    /// Open or create today's daily note and switch editor to it.
    pub fn open_daily_note(&mut self) -> Result<()> {
        let date = Local::now().format("%Y-%m-%d").to_string();
        let folder = self.notes_dir.join(self.config.daily_notes_folder.trim());
        fs::create_dir_all(&folder)?;
        let path = folder.join(format!("{}.md", date));
        if !path.exists() {
            let header = format!("# Daily Note: {}\n\n", date);
            fs::write(&path, header)?;
        }
        self.load_file_into_editor(path)
    }

    /// Load file content into a new or existing tab and switch focus to Editor.
    pub fn load_file_into_editor(&mut self, path: PathBuf) -> Result<()> {
        self.load_file_into_editor_at_line(path, None)
    }

    /// Load file and optionally move cursor to the given 0-based line.
    pub fn load_file_into_editor_at_line(&mut self, path: PathBuf, goto_line: Option<usize>) -> Result<()> {
        // Check if already open
        if let Some(idx) = self.buffers.iter().position(|b| b.path.as_ref() == Some(&path)) {
            self.active_tab = idx;
            self.focus = Focus::Editor;
            self.editor_mode = EditorMode::Normal;
            if let Some(line) = goto_line {
                if let Some(buf) = self.buffers.get_mut(idx) {
                    let row = line.min(buf.textarea.lines().len().saturating_sub(1));
                    buf.textarea.move_cursor(CursorMove::Jump(row as u16, 0));
                }
            }
            return Ok(());
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };
        let mut buf = EditorBuffer::new(Some(path), lines);
        buf.textarea.set_max_histories(50);
        if let Some(line) = goto_line {
            let row = line.min(buf.textarea.lines().len().saturating_sub(1));
            buf.textarea.move_cursor(CursorMove::Jump(row as u16, 0));
        }
        Self::apply_theme_to_textarea(&self.theme, &mut buf.textarea, &self.config.editor);
        self.buffers.push(buf);
        self.active_tab = self.buffers.len() - 1;
        self.focus = Focus::Editor;
        self.editor_mode = EditorMode::Normal;
        if self.config.editor.show_backlinks {
            self.scan_backlinks();
        }
        Ok(())
    }

    /// Switch focus back to List. Auto-saves before switching.
    pub fn focus_list(&mut self) {
        let _ = self.save_all_buffers();
        self.focus = Focus::List;
    }

    /// Delete the selected note file. If it was open in the editor, clears buffers.
    pub fn delete_selected_note(&mut self) -> Result<()> {
        let entry = match self.filtered_notes.get(self.selected) {
            Some(e) => e,
            None => return Ok(()),
        };
        if entry.is_directory {
            self.message = Some("Use a file manager to delete directories".to_string());
            return Ok(());
        }
        let path = entry.path.clone();

        if path.ends_with("config.toml") || path.ends_with("theme.toml") {
            self.message = Some("Cannot delete config files".to_string());
            return Ok(());
        }

        self.buffers.retain(|b| b.path.as_ref() != Some(&path));
        if self.active_tab >= self.buffers.len() {
            self.active_tab = self.buffers.len().saturating_sub(1);
        }
        if self.split_right_tab.map(|i| i >= self.buffers.len()).unwrap_or(false) {
            self.split_right_tab = None;
        }
        fs::remove_file(&path)?;
        self.refresh_notes()?;
        if self.buffers.is_empty() {
            self.buffers.push(EditorBuffer::new(None, vec![String::new()]));
            self.active_tab = 0;
            self.focus = Focus::List;
            self.apply_editor_theme_to_all();
        }

        self.message = Some("Deleted".to_string());
        Ok(())
    }

    /// Save all buffers to disk (auto-save, no user message).
    pub fn save_all_buffers(&mut self) -> Result<()> {
        let mut need_reload = false;
        for buf in &mut self.buffers {
            if let Some(path) = &buf.path {
                let content = buf.textarea.lines().join("\n");
                fs::write(path, content)?;
                if path.ends_with("config.toml") || path.ends_with("theme.toml") {
                    need_reload = true;
                }
            }
        }
        self.editor_dirty = false;
        if need_reload {
            let _ = self.reload_config();
        }
        self.refresh_notes()?;
        Ok(())
    }

    /// Mark that the editor content has changed (for auto-save tracking).
    pub fn mark_editor_dirty(&mut self) {
        self.editor_dirty = true;
        self.last_keystroke_time = Some(Instant::now());
    }

    /// Check auto-save condition and save if needed. Returns true if a save was performed.
    pub fn check_auto_save(&mut self) -> Result<bool> {
        if !self.config.editor.auto_save || !self.editor_dirty {
            return Ok(false);
        }
        let last = match self.last_keystroke_time {
            Some(t) => t,
            None => return Ok(false),
        };
        let interval = Duration::from_secs(self.config.editor.auto_save_interval);
        if Instant::now().duration_since(last) < interval {
            return Ok(false);
        }
        self.save_all_buffers()?;
        self.save_indicator_until = Some(Instant::now() + Duration::from_secs(2));
        Ok(true)
    }

    /// Clear "Saved..." indicator when expired.
    pub fn tick_save_indicator(&mut self) {
        if let Some(until) = self.save_indicator_until {
            if Instant::now() >= until {
                self.save_indicator_until = None;
            }
        }
    }

    /// Save the current editor content to disk.
    pub fn save_editor(&mut self) -> Result<()> {
        self.save_all_buffers()
    }

    fn apply_theme_to_textarea(
        theme: &ResolvedTheme,
        textarea: &mut TextArea<'static>,
        editor_config: &crate::config::EditorConfig,
    ) {
        let editor_style = theme.editor_fg_style.patch(theme.editor_bg_style);
        textarea.set_style(editor_style);
        textarea.set_cursor_style(theme.editor_cursor_style);
        textarea.set_cursor_line_style(
            ratatui::style::Style::default()
                .add_modifier(ratatui::style::Modifier::UNDERLINED),
        );
        if editor_config.line_numbers {
            textarea.set_line_number_style(theme.editor_line_number_style);
        } else {
            textarea.remove_line_number();
        }
        let tab_len = editor_config.tab_width.clamp(1, 16);
        textarea.set_tab_length(tab_len);
        // Headers (# ), list markers (- ), unchecked (- [ ]), checked (- [x]), code blocks (```)
        let _ = textarea.set_search_pattern(
            r"(^#{1,6} )|(^[-*] )|(^[-*] \[ \])|(^[-*] \[[xX]\])|(^```)",
        );
        textarea.set_search_style(
            theme
                .editor_header_style
                .patch(theme.editor_list_style)
                .patch(theme.editor_checkbox_style)
                .patch(theme.editor_checkbox_checked_style)
                .patch(theme.editor_code_block_style),
        );
    }

    fn apply_editor_theme_to_all(&mut self) {
        for buf in self.buffers.iter_mut() {
            Self::apply_theme_to_textarea(&self.theme, &mut buf.textarea, &self.config.editor);
        }
    }

    /// Handle editor input in Normal mode (vim-like).
    pub fn editor_normal_input(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        if key_matches(key, &[self.resolved_keys.escape]) {
            self.editor_mode = EditorMode::Normal;
            self.g_pending = false;
            return true;
        }
        if self.g_pending {
            self.g_pending = false;
            match key.code {
                KeyCode::Char('t') => {
                    self.next_tab();
                    return true;
                }
                KeyCode::Char('T') => {
                    self.prev_tab();
                    return true;
                }
                KeyCode::Char('s') => {
                    self.toggle_split_view();
                    return true;
                }
                KeyCode::Char('q') => {
                    self.close_tab();
                    return true;
                }
                KeyCode::Char('d') => {
                    if let Some(link) = self.get_wiki_link_under_cursor() {
                        let _ = self.open_wiki_link(&link);
                    }
                    return true;
                }
                _ => {}
            }
        }
        if key.code == KeyCode::Char('g') {
            self.g_pending = true;
            return true;
        }
        if key_matches(key, &[self.resolved_keys.editor_back]) {
            self.focus_list();
            return true;
        }
        let Some(buf) = self.focused_buffer_mut() else { return false };
        match key.code {
            KeyCode::Char('i') => {
                self.editor_mode = EditorMode::Insert;
                return true;
            }
            KeyCode::Char('a') => {
                buf.textarea.move_cursor(CursorMove::Forward);
                self.editor_mode = EditorMode::Insert;
                return true;
            }
            KeyCode::Char('u') => {
                buf.textarea.undo();
                return true;
            }
            KeyCode::Char('h') => buf.textarea.move_cursor(CursorMove::Back),
            KeyCode::Char('j') => buf.textarea.move_cursor(CursorMove::Down),
            KeyCode::Char('k') => buf.textarea.move_cursor(CursorMove::Up),
            KeyCode::Char('l') => buf.textarea.move_cursor(CursorMove::Forward),
            KeyCode::Left => buf.textarea.move_cursor(CursorMove::Back),
            KeyCode::Right => buf.textarea.move_cursor(CursorMove::Forward),
            KeyCode::Up => buf.textarea.move_cursor(CursorMove::Up),
            KeyCode::Down => buf.textarea.move_cursor(CursorMove::Down),
            KeyCode::Home => buf.textarea.move_cursor(CursorMove::Head),
            KeyCode::End => buf.textarea.move_cursor(CursorMove::End),
            KeyCode::PageUp => buf.textarea.scroll(Scrolling::PageUp),
            KeyCode::PageDown => buf.textarea.scroll(Scrolling::PageDown),
            _ => return false,
        }
        true
    }

    // Telescope (Space+f)
    pub fn enter_telescope(&mut self) {
        self.focus = Focus::Search;
        self.telescope_notes = find_md_files_recursive(&self.notes_dir);
        self.telescope_filtered = self.telescope_notes.clone();
        self.telescope_query.clear();
        self.telescope_selected = 0;
        self.apply_telescope_filter();
    }

    pub fn exit_telescope(&mut self) {
        self.focus = if self.has_open_buffers() {
            Focus::Editor
        } else {
            Focus::List
        };
    }

    pub fn telescope_add_char(&mut self, c: char) {
        self.telescope_query.push(c);
        self.apply_telescope_filter();
        self.telescope_selected = 0;
    }

    pub fn telescope_backspace(&mut self) {
        self.telescope_query.pop();
        self.apply_telescope_filter();
        self.telescope_selected = self.telescope_selected.saturating_sub(1).min(
            self.telescope_filtered.len().saturating_sub(1),
        );
    }

    fn apply_telescope_filter(&mut self) {
        self.telescope_filtered =
            filter_telescope_notes(&self.telescope_notes, &self.telescope_query, &mut self.telescope_matcher);
        self.telescope_match_indices = self
            .telescope_filtered
            .iter()
            .map(|n| get_telescope_match_indices(&n.display, &self.telescope_query, &mut self.telescope_matcher))
            .collect();
        if self.telescope_selected >= self.telescope_filtered.len() {
            self.telescope_selected = self.telescope_filtered.len().saturating_sub(1);
        }
    }

    pub fn telescope_move_up(&mut self) {
        if self.telescope_selected > 0 {
            self.telescope_selected -= 1;
        }
    }

    pub fn telescope_move_down(&mut self) {
        if self.telescope_selected + 1 < self.telescope_filtered.len() {
            self.telescope_selected += 1;
        }
    }

    pub fn get_telescope_selected_path(&self) -> Option<PathBuf> {
        self.telescope_filtered
            .get(self.telescope_selected)
            .map(|n| n.path.clone())
    }

    // Command palette (Ctrl+p)
    pub fn enter_command_palette(&mut self) {
        self.focus = Focus::CommandPalette;
        self.command_palette_query.clear();
        self.command_palette_filtered = CommandAction::all().to_vec();
        self.command_palette_selected = 0;
    }

    pub fn exit_command_palette(&mut self) {
        self.focus = if self.has_open_buffers() {
            Focus::Editor
        } else {
            Focus::List
        };
    }

    pub fn command_palette_add_char(&mut self, c: char) {
        self.command_palette_query.push(c);
        self.apply_command_palette_filter();
    }

    pub fn command_palette_backspace(&mut self) {
        self.command_palette_query.pop();
        self.apply_command_palette_filter();
    }

    fn apply_command_palette_filter(&mut self) {
        let q = self.command_palette_query.to_lowercase();
        self.command_palette_filtered = CommandAction::all()
            .iter()
            .filter(|a| a.label().to_lowercase().contains(&q))
            .copied()
            .collect();
        self.command_palette_selected = 0.min(
            self.command_palette_filtered.len().saturating_sub(1),
        );
    }

    pub fn command_palette_move_up(&mut self) {
        if self.command_palette_selected > 0 {
            self.command_palette_selected -= 1;
        }
    }

    pub fn command_palette_move_down(&mut self) {
        if self.command_palette_selected + 1 < self.command_palette_filtered.len() {
            self.command_palette_selected += 1;
        }
    }

    pub fn get_command_palette_action(&self) -> Option<CommandAction> {
        self.command_palette_filtered.get(self.command_palette_selected).copied()
    }

    // Rename popup (r)
    pub fn enter_rename(&mut self) {
        if let Some(entry) = self.filtered_notes.get(self.selected) {
            let name = entry
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            self.rename_input = name;
            self.focus = Focus::Rename;
        }
    }

    pub fn exit_rename(&mut self) {
        self.focus = Focus::List;
        self.rename_input.clear();
    }

    pub fn rename_add_char(&mut self, c: char) {
        self.rename_input.push(c);
    }

    pub fn rename_backspace(&mut self) {
        self.rename_input.pop();
    }

    pub fn rename_selected_note(&mut self) -> Result<()> {
        let entry = match self.filtered_notes.get(self.selected) {
            Some(e) => e,
            None => return Ok(()),
        };
        let old_path = entry.path.clone();
        let is_dir = entry.is_directory;
        let name = self.rename_input.trim();
        if name.is_empty() {
            self.message = Some("Name cannot be empty".to_string());
            return Ok(());
        }
        let name = if is_dir {
            name.to_string()
        } else if name.ends_with(".md") {
            name.to_string()
        } else {
            format!("{}.md", name)
        };
        let parent = old_path.parent().unwrap_or(&self.current_dir);
        let new_path = parent.join(&name);
        if new_path.exists() && new_path != old_path {
            self.message = Some("File already exists".to_string());
            return Ok(());
        }
        let was_editing = self.buffers.iter().any(|b| b.path.as_ref() == Some(&old_path));
        fs::rename(&old_path, &new_path)?;
        self.refresh_notes()?;
        if was_editing {
            let _ = self.load_file_into_editor(new_path);
        }
        self.exit_rename();
        self.message = Some("Renamed".to_string());
        Ok(())
    }

    // Create directory popup (Shift+n)
    pub fn enter_create_directory(&mut self) {
        self.directory_input.clear();
        self.focus = Focus::CreatingDirectory;
    }

    pub fn exit_create_directory(&mut self) {
        self.focus = Focus::List;
        self.directory_input.clear();
    }

    pub fn directory_add_char(&mut self, c: char) {
        self.directory_input.push(c);
    }

    pub fn directory_backspace(&mut self) {
        self.directory_input.pop();
    }

    pub fn create_directory(&mut self) -> Result<()> {
        let name = self.directory_input.trim().to_string();
        if name.is_empty() {
            self.message = Some("Directory name cannot be empty".to_string());
            return Ok(());
        }
        let path = self.current_dir.join(&name);
        if path.exists() {
            self.message = Some("Directory already exists".to_string());
            return Ok(());
        }
        fs::create_dir(&path)
            .map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;
        self.exit_create_directory();
        self.refresh_notes()?;
        self.message = Some(format!("Created directory: {}", name));
        Ok(())
    }

    // Zen mode
    pub fn toggle_zen_mode(&mut self) {
        self.zen_mode = !self.zen_mode;
    }

    // Git status
    pub fn git_status(&self) -> GitStatus {
        get_git_status(&self.notes_dir)
    }

    // Checkbox toggle (Ctrl+Space)
    #[allow(dead_code)]
    fn toggle_checkbox_at_cursor(&mut self) {
        let idx = self.focused_buffer_index();
        let (row, col, lines) = {
            let buf = match self.buffers.get_mut(idx) {
                Some(b) => b,
                None => return,
            };
            let (r, c) = buf.textarea.cursor();
            let l = buf.textarea.lines().to_vec();
            (r, c, l)
        };
        let line = match lines.get(row) {
            Some(l) => l.clone(),
            None => return,
        };
        let re_unchecked = match Regex::new(r"^(\s*[-*]\s+)\[\s?\]") {
            Ok(r) => r,
            Err(_) => return,
        };
        let re_checked = match Regex::new(r"^(\s*[-*]\s+)\[[xX]\]") {
            Ok(r) => r,
            Err(_) => return,
        };
        let new_line = if re_unchecked.is_match(&line) {
            re_unchecked.replace(&line, "${1}[x]").into_owned()
        } else if re_checked.is_match(&line) {
            re_checked.replace(&line, "${1}[ ]").into_owned()
        } else {
            return;
        };
        let mut new_lines = lines;
        new_lines[row] = new_line.clone();
        let new_col = col.min(new_line.len());
        let theme = self.theme.clone();
        if let Some(buf) = self.buffers.get_mut(idx) {
            buf.textarea = TextArea::new(new_lines);
            buf.textarea.set_max_histories(50);
            Self::apply_theme_to_textarea(&theme, &mut buf.textarea, &self.config.editor);
            let r = row as u16;
            let c = new_col.min(u16::MAX as usize) as u16;
            buf.textarea.move_cursor(CursorMove::Jump(r, c));
        }
    }

    // Wiki link: [[Filename]] under cursor
    pub fn get_wiki_link_under_cursor(&self) -> Option<String> {
        let buf = self.focused_buffer()?;
        let (row, col) = buf.textarea.cursor();
        let lines = buf.textarea.lines();
        let line = lines.get(row)?;
        let re = Regex::new(r"\[\[([^\]]+)\]\]").ok()?;
        for cap in re.captures_iter(line) {
            let m = cap.get(0)?;
            let start = m.start();
            let end = m.end();
            if col >= start && col <= end {
                return Some(cap.get(1)?.as_str().to_string());
            }
        }
        None
    }

    pub fn open_wiki_link(&mut self, link: &str) -> Result<()> {
        let _ = self.save_editor();
        let name = if link.ends_with(".md") {
            link.to_string()
        } else {
            format!("{}.md", link)
        };
        let path = self.editing_path()
            .as_ref()
            .and_then(|p| p.parent())
            .unwrap_or(&self.current_dir)
            .join(&name);
        if path.exists() {
            self.load_file_into_editor(path)?;
        } else {
            let path = self.current_dir.join(&name);
            if path.exists() {
                self.load_file_into_editor(path)?;
            } else {
                fs::File::create(&path)?;
                self.load_file_into_editor(path)?;
            }
        }
        Ok(())
    }

    /// Scan for backlinks to the current file. Returns paths of files containing [[current_file_name]].
    pub fn scan_backlinks(&mut self) {
        self.backlinks.clear();
        self.backlinks_selected = 0;
        let current_file_name = match self.editing_path() {
            Some(p) => p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()),
            None => return,
        };
        let Some(target_name) = current_file_name else { return };
        let pattern = format!("[[{}]]", target_name);
        let current_path = self.editing_path();

        for entry in WalkDir::new(&self.notes_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() || path.extension().map_or(true, |e| e != "md") {
                continue;
            }
            if current_path.as_ref() == Some(&path.to_path_buf()) {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path) {
                if content.contains(&pattern) {
                    self.backlinks.push(path.to_path_buf());
                }
            }
        }
        self.backlinks.sort();
    }

    pub fn backlinks_move_up(&mut self) {
        if self.backlinks_selected > 0 {
            self.backlinks_selected -= 1;
        }
    }

    pub fn backlinks_move_down(&mut self) {
        if self.backlinks_selected + 1 < self.backlinks.len() {
            self.backlinks_selected += 1;
        }
    }

    pub fn open_selected_backlink(&mut self) -> Result<()> {
        if let Some(path) = self.backlinks.get(self.backlinks_selected).cloned() {
            self.load_file_into_editor(path)?;
        }
        Ok(())
    }

    // Tag Explorer
    pub fn enter_tag_explorer(&mut self) {
        self.tag_explorer_active = true;
        self.tag_explorer_view = TagExplorerView::TagList;
        self.focus = Focus::TagExplorer;
        self.scan_all_tags();
    }

    pub fn exit_tag_explorer(&mut self) {
        self.tag_explorer_active = false;
        self.focus = Focus::List;
    }

    pub fn scan_all_tags(&mut self) {
        use std::collections::HashSet;
        let mut tags = HashSet::new();
        let re = Regex::new(r"#(\w+)").unwrap();

        for entry in WalkDir::new(&self.notes_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() || path.extension().map_or(true, |e| e != "md") {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path) {
                for cap in re.captures_iter(&content) {
                    if let Some(tag) = cap.get(1) {
                        tags.insert(tag.as_str().to_string());
                    }
                }
            }
        }

        self.all_tags = tags.into_iter().collect();
        self.all_tags.sort();
        self.tag_selected = 0;
        self.tag_files.clear();
        self.tag_file_selected = 0;
    }

    pub fn tag_list_move_up(&mut self) {
        if self.tag_selected > 0 {
            self.tag_selected -= 1;
        }
    }

    pub fn tag_list_move_down(&mut self) {
        if self.tag_selected + 1 < self.all_tags.len() {
            self.tag_selected += 1;
        }
    }

    pub fn tag_file_move_up(&mut self) {
        if self.tag_file_selected > 0 {
            self.tag_file_selected -= 1;
        }
    }

    pub fn tag_file_move_down(&mut self) {
        if self.tag_file_selected + 1 < self.tag_files.len() {
            self.tag_file_selected += 1;
        }
    }

    pub fn load_files_for_selected_tag(&mut self) {
        if let Some(tag) = self.all_tags.get(self.tag_selected) {
            self.tag_files.clear();
            self.tag_file_selected = 0;
            let pattern = format!("#{}", tag);

            for entry in WalkDir::new(&self.notes_dir)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if !path.is_file() || path.extension().map_or(true, |e| e != "md") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(path) {
                    if content.contains(&pattern) {
                        self.tag_files.push(path.to_path_buf());
                    }
                }
            }
            self.tag_files.sort();
            self.tag_explorer_view = TagExplorerView::FileList;
        }
    }

    pub fn open_selected_tag_file(&mut self) -> Result<()> {
        if let Some(path) = self.tag_files.get(self.tag_file_selected).cloned() {
            self.exit_tag_explorer();
            self.load_file_into_editor(path)?;
        }
        Ok(())
    }

    // Global Task Board
    pub fn enter_task_view(&mut self) {
        self.task_view_active = true;
        self.focus = Focus::TaskView;
        self.scan_tasks();
    }

    pub fn exit_task_view(&mut self) {
        self.task_view_active = false;
        self.focus = Focus::List;
    }

    /// Recursively scan workspace for lines starting with `- [ ]` (unchecked tasks).
    pub fn scan_tasks(&mut self) {
        self.tasks.clear();
        self.task_selected = 0;

        for entry in WalkDir::new(&self.notes_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() || path.extension().map_or(true, |e| e != "md") {
                continue;
            }
            let path_buf = path.to_path_buf();
            if let Ok(content) = fs::read_to_string(path) {
                for (zero_based_line, line) in content.lines().enumerate() {
                    if line.trim_start().starts_with("- [ ]") {
                        let content = line.trim_start().trim_start_matches("- [ ]").trim().to_string();
                        self.tasks.push(TaskEntry {
                            path: path_buf.clone(),
                            line_number: zero_based_line,
                            content,
                        });
                    }
                }
            }
        }
    }

    pub fn task_move_up(&mut self) {
        if self.task_selected > 0 {
            self.task_selected -= 1;
        }
    }

    pub fn task_move_down(&mut self) {
        if self.task_selected + 1 < self.tasks.len() {
            self.task_selected += 1;
        }
    }

    pub fn open_selected_task(&mut self) -> Result<()> {
        if let Some(task) = self.tasks.get(self.task_selected) {
            let path = task.path.clone();
            let line = task.line_number;
            self.exit_task_view();
            self.load_file_into_editor_at_line(path, Some(line))?;
        }
        Ok(())
    }

    // Templates
    pub fn enter_template_picker(&mut self) {
        self.template_picker_active = true;
        self.template_picker_selected = 0;
    }

    pub fn exit_template_picker(&mut self) {
        self.template_picker_active = false;
    }

    pub fn template_picker_move_up(&mut self) {
        if self.template_picker_selected > 0 {
            self.template_picker_selected -= 1;
        }
    }

    pub fn template_picker_move_down(&mut self) {
        let max = Template::all().len().saturating_sub(1);
        if self.template_picker_selected < max {
            self.template_picker_selected += 1;
        }
    }

    pub fn get_selected_template(&self) -> Template {
        Template::all()
            .get(self.template_picker_selected)
            .copied()
            .unwrap_or(Template::Empty)
    }

    pub fn create_note_with_template(&mut self, template: Template) -> Result<Option<PathBuf>> {
        let name = self.create_filename.clone();
        let path = self.create_note_from_filename(&name, template)?;
        self.exit_create_mode();
        self.exit_template_picker();
        Ok(path)
    }

    fn create_note_from_filename(&mut self, name: &str, template: Template) -> Result<Option<PathBuf>> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(None);
        }
        let name = if name.ends_with(".md") {
            name.to_string()
        } else {
            format!("{}.md", name)
        };
        let path = self.current_dir.join(&name);
        if path.exists() {
            self.message = Some("File already exists".to_string());
            return Ok(None);
        }
        let content = template.content();
        fs::write(&path, content)?;
        self.message = None;
        Ok(Some(path))
    }

    pub fn insert_date_at_cursor(&mut self) {
        let idx = self.focused_buffer_index();
        let (date, row, col, mut lines) = {
            let buf = match self.buffers.get_mut(idx) {
                Some(b) => b,
                None => return,
            };
            let date = Local::now().format("%Y-%m-%d").to_string();
            let (r, c) = buf.textarea.cursor();
            let l = buf.textarea.lines().to_vec();
            (date, r, c, l)
        };
        if let Some(line) = lines.get_mut(row) {
            let mut s = line.clone();
            if col <= s.len() {
                s.insert_str(col, &date);
            } else {
                s.push_str(&date);
            }
            lines[row] = s;
            let theme = self.theme.clone();
            if let Some(buf) = self.buffers.get_mut(idx) {
                buf.textarea = TextArea::new(lines);
                buf.textarea.set_max_histories(50);
                Self::apply_theme_to_textarea(&theme, &mut buf.textarea, &self.config.editor);
                let r = row as u16;
                let c = (col + date.len()).min(u16::MAX as usize) as u16;
                buf.textarea.move_cursor(CursorMove::Jump(r, c));
            }
        }
    }

    pub fn git_push(&mut self) -> Result<()> {
        Command::new("git")
            .arg("push")
            .current_dir(&self.notes_dir)
            .status()?;
        self.message = Some("Git push done".to_string());
        Ok(())
    }

    /// Toggle split view.
    pub fn toggle_split_view(&mut self) {
        self.editor_layout = match self.editor_layout {
            EditorLayout::Single => {
                if self.buffers.len() >= 2 {
                    self.split_right_tab = Some((self.active_tab + 1) % self.buffers.len());
                    self.split_focus_left = true;
                    EditorLayout::SplitVertical
                } else {
                    EditorLayout::Single
                }
            }
            EditorLayout::SplitVertical => {
                self.split_right_tab = None;
                EditorLayout::Single
            }
        };
    }

    /// Export current buffer to PDF via Pandoc.
    pub fn export_to_pdf(&mut self) -> Result<()> {
        let buf = self.focused_buffer();
        let path = match buf.and_then(|b| b.path.as_ref()) {
            Some(p) if p.extension().map_or(false, |e| e == "md") => p.clone(),
            _ => {
                self.message = Some("No Markdown file open".to_string());
                return Ok(());
            }
        };
        let _ = self.save_editor();
        let output = path.with_extension("pdf");
        let output_str = output.to_string_lossy();
        let input_str = path.to_string_lossy();
        let status = Command::new("pandoc")
            .arg(&*input_str)
            .arg("-o")
            .arg(&*output_str)
            .status();
        match status {
            Ok(s) if s.success() => {
                self.message = Some(format!("Exported to {}", output.display()));
            }
            Ok(_) => {
                self.message = Some("Pandoc failed".to_string());
            }
            Err(_) => {
                self.message = Some("Pandoc not found - install pandoc".to_string());
            }
        }
        Ok(())
    }

    /// Switch to next tab.
    pub fn next_tab(&mut self) {
        if !self.buffers.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.buffers.len();
        }
    }

    /// Switch to previous tab.
    pub fn prev_tab(&mut self) {
        if !self.buffers.is_empty() {
            self.active_tab = self
                .active_tab
                .checked_sub(1)
                .unwrap_or(self.buffers.len() - 1);
        }
    }

    /// Close current tab.
    pub fn close_tab(&mut self) {
        if self.buffers.len() <= 1 {
            return;
        }
        let _ = self.save_editor();
        self.buffers.remove(self.focused_buffer_index());
        if self.active_tab >= self.buffers.len() {
            self.active_tab = self.buffers.len() - 1;
        }
        if self.split_right_tab.map(|i| i >= self.buffers.len()).unwrap_or(false) {
            self.split_right_tab = None;
            self.editor_layout = EditorLayout::Single;
        }
    }
}

fn load_entries(dir: &PathBuf) -> Result<Vec<NoteEntry>> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => return Err(anyhow::anyhow!("Cannot read directory {}: {}", dir.display(), e)),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            let display = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            dirs.push(NoteEntry::dir(path, format!("{}/", display)));
        } else if meta.is_file() {
            if path.extension().map_or(false, |e| e == "md") {
                let display = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let (content, searchable) = read_note_content(&path, &display);
                files.push(NoteEntry {
                    path,
                    display,
                    content,
                    searchable,
                    is_directory: false,
                });
            }
        }
    }

    dirs.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));
    files.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));

    let mut result = dirs;
    result.append(&mut files);
    Ok(result)
}

fn read_note_content(path: &PathBuf, display: &str) -> (String, String) {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (String::new(), display.to_string()),
    };

    let mut buf = Vec::with_capacity(MAX_CONTENT_BYTES + 1);
    let mut take = file.take(MAX_CONTENT_BYTES as u64);
    if take.read_to_end(&mut buf).is_err() {
        return (String::new(), display.to_string());
    }

    let mut content = String::from_utf8_lossy(&buf).into_owned();
    if buf.len() >= MAX_CONTENT_BYTES {
        content.push_str("\n\n(Content truncated - file too large)");
    }
    let searchable = format!("{}\n{}", display, content);
    (content, searchable)
}
