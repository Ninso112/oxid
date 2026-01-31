// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

mod app;
mod config;
mod frontmatter;
mod git;
mod markdown;
mod search;
mod spellcheck;
mod telescope;
mod templates;
mod theme;
mod ui;

use anyhow::Result;
use app::{App, CommandAction, EditorLayout, EditorMode, Focus, Mode};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use tui_textarea::Input;

#[derive(Parser, Debug)]
#[command(name = "oxid")]
#[command(author = "Oxid Contributors")]
#[command(version)]
#[command(about = "A fast, keyboard-driven TUI note editor for Linux")]
struct CliArgs {}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let alt = key.modifiers.contains(KeyModifiers::ALT);

            // Global: F11 Zen Mode
            if key.code == KeyCode::F(11) {
                app.toggle_zen_mode();
                continue;
            }

            // Global: / Telescope search (config.keys.search)
            if key.code == KeyCode::Char('/') && !ctrl && !alt
                && app.config.keys.search == "/"
            {
                app.enter_telescope();
                continue;
            }

            // Global: Ctrl+p Command Palette
            if key.code == KeyCode::Char('p') && ctrl {
                app.enter_command_palette();
                continue;
            }

            // Focus-specific handling
            match app.focus {
                Focus::Search => {
                    match key.code {
                        KeyCode::Esc => app.exit_telescope(),
                        KeyCode::Enter => {
                            if let Some(path) = app.get_telescope_selected_path() {
                                let _ = app.load_file_into_editor(path);
                                app.exit_telescope();
                            }
                        }
                        KeyCode::Backspace => app.telescope_backspace(),
                        KeyCode::Up | KeyCode::Char('k') => app.telescope_move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.telescope_move_down(),
                        KeyCode::Char(c) => app.telescope_add_char(c),
                        _ => {}
                    }
                }
                Focus::CommandPalette => {
                    match key.code {
                        KeyCode::Esc => app.exit_command_palette(),
                        KeyCode::Enter => {
                            if let Some(action) = app.get_command_palette_action() {
                                match action {
                                    CommandAction::RenameFile => {
                                        app.exit_command_palette();
                                        app.focus = Focus::List;
                                        app.enter_rename();
                                    }
                                    CommandAction::DeleteFile => {
                                        app.exit_command_palette();
                                        app.focus = Focus::List;
                                        let _ = app.delete_selected_note();
                                    }
                                    CommandAction::InsertDate => {
                                        app.exit_command_palette();
                                        app.focus = Focus::Editor;
                                        app.insert_date_at_cursor();
                                    }
                                    CommandAction::ToggleZenMode => {
                                        app.toggle_zen_mode();
                                        app.exit_command_palette();
                                    }
                                    CommandAction::ToggleSplitView => {
                                        app.toggle_split_view();
                                        app.exit_command_palette();
                                    }
                                    CommandAction::ExportPdf => {
                                        let _ = app.export_to_pdf();
                                        app.exit_command_palette();
                                    }
                                    CommandAction::GitPush => {
                                        let _ = app.git_push();
                                        app.exit_command_palette();
                                    }
                                }
                            }
                        }
                        KeyCode::Backspace => app.command_palette_backspace(),
                        KeyCode::Up | KeyCode::Char('k') => app.command_palette_move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.command_palette_move_down(),
                        KeyCode::Char(c) => app.command_palette_add_char(c),
                        _ => {}
                    }
                }
                Focus::Rename => {
                    match key.code {
                        KeyCode::Esc => app.exit_rename(),
                        KeyCode::Enter => {
                            let _ = app.rename_selected_note();
                        }
                        KeyCode::Backspace => app.rename_backspace(),
                        KeyCode::Char(c) => app.rename_add_char(c),
                        _ => {}
                    }
                }
                Focus::CreatingDirectory => {
                    match key.code {
                        KeyCode::Esc => app.exit_create_directory(),
                        KeyCode::Enter => {
                            let _ = app.create_directory();
                        }
                        KeyCode::Backspace => app.directory_backspace(),
                        KeyCode::Char(c) => app.directory_add_char(c),
                        _ => {}
                    }
                }
                Focus::List => {
                    if app.template_picker_active {
                        match key.code {
                            KeyCode::Esc => app.exit_template_picker(),
                            KeyCode::Enter => {
                                if let Some(path) = app.create_note_with_template(app.get_selected_template())? {
                                    let _ = app.load_file_into_editor(path);
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => app.template_picker_move_up(),
                            KeyCode::Down | KeyCode::Char('j') => app.template_picker_move_down(),
                            _ => {}
                        }
                    } else {
                        match app.mode {
                            Mode::Normal => {
                                match key.code {
                                    KeyCode::Char('q') => {
                                        let _ = app.save_editor();
                                        break;
                                    }
                                    KeyCode::Up | KeyCode::Char('k') => app.move_selection_up(),
                                    KeyCode::Down | KeyCode::Char('j') => app.move_selection_down(),
                                    KeyCode::Char('/') => app.enter_search_mode(),
                                    KeyCode::Char('n') => app.enter_create_mode(),
                                    KeyCode::Char('N') => app.enter_create_directory(),
                                    KeyCode::Char('r') => app.enter_rename(),
                                    KeyCode::Char('c') => {
                                        if let Ok(config_path) = config::config_file_path() {
                                            let _ = app.load_file_into_editor(config_path);
                                        }
                                    }
                                    KeyCode::Char('d') | KeyCode::Delete => {
                                        let _ = app.delete_selected_note();
                                    }
                                    KeyCode::Enter => {
                                        if let Some(path) = app.get_selected_path() {
                                            let _ = app.load_file_into_editor(path);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Mode::Search => {
                                match key.code {
                                    KeyCode::Esc => app.exit_search_mode(),
                                    KeyCode::Enter => {
                                        if let Some(path) = app.get_selected_path() {
                                            let _ = app.load_file_into_editor(path);
                                            app.exit_search_mode();
                                        }
                                    }
                                    KeyCode::Backspace => app.search_backspace(),
                                    KeyCode::Char(c) => app.search_add_char(c),
                                    _ => {}
                                }
                            }
                            Mode::Create => {
                                match key.code {
                                    KeyCode::Esc => app.exit_create_mode(),
                                    KeyCode::Enter => {
                                        app.enter_template_picker();
                                    }
                                    KeyCode::Backspace => app.create_backspace(),
                                    KeyCode::Char(c) => app.create_add_char(c),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Focus::Editor => {
                    // Ctrl+e: PDF export
                    if key.code == KeyCode::Char('e') && ctrl {
                        let _ = app.export_to_pdf();
                        continue;
                    }
                    // Split pane focus switch
                    if app.editor_layout == EditorLayout::SplitVertical
                        && app.split_right_tab.is_some()
                    {
                        if key.code == KeyCode::Tab {
                            app.split_focus_left = !app.split_focus_left;
                            continue;
                        }
                    }

                    // Enter in Normal mode: Wiki link navigation
                    if key.code == KeyCode::Enter
                        && app.editor_mode == EditorMode::Normal
                    {
                        if let Some(link) = app.get_wiki_link_under_cursor() {
                            let _ = app.open_wiki_link(&link);
                            continue;
                        }
                    }

                    match app.editor_mode {
                        EditorMode::Normal => {
                            app.editor_normal_input(key);
                        }
                        EditorMode::Insert => {
                            if key.code == KeyCode::Esc {
                                app.editor_mode = EditorMode::Normal;
                            } else if let Some(buf) = app.focused_buffer_mut() {
                                let input: Input = key.into();
                                buf.textarea.input_without_shortcuts(input);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let _args = CliArgs::parse();

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
