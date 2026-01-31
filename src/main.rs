// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

mod app;
mod config;
mod frontmatter;
mod git;
mod handlers;
mod markdown;
mod search;
mod spellcheck;
mod telescope;
mod templates;
mod theme;
mod ui;

use anyhow::Result;
use app::{App, CommandAction, EditorLayout, EditorMode, Focus, Mode, TagExplorerView};
use handlers::key_matches;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
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
    let poll_timeout = Duration::from_millis(500);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        app.tick_save_indicator();

        if !event::poll(poll_timeout)? {
            if app.check_auto_save()? {
                continue;
            }
            continue;
        }

        let Ok(Event::Key(key)) = event::read() else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let k = &app.resolved_keys;

            // Global
            if key_matches(key, &[k.zen_mode]) {
                app.toggle_zen_mode();
                continue;
            }
            if key_matches(key, &[k.search]) {
                app.enter_telescope();
                continue;
            }
            if key_matches(key, &[k.command_palette]) {
                app.enter_command_palette();
                continue;
            }
            if key_matches(key, &[k.daily_note]) {
                let _ = app.open_daily_note();
                continue;
            }
            if key_matches(key, &[k.task_board]) {
                app.enter_task_view();
                continue;
            }

            // Focus-specific handling
            match app.focus {
                Focus::Search => {
                    if key_matches(key, &[k.escape]) {
                        app.exit_telescope();
                    } else if key_matches(key, &[k.enter]) {
                        if let Some(path) = app.get_telescope_selected_path() {
                            let _ = app.load_file_into_editor(path);
                            app.exit_telescope();
                        }
                    } else if key_matches(key, &[k.backspace]) {
                        app.telescope_backspace();
                    } else if key_matches(key, &[k.move_up, k.move_up_alt]) {
                        app.telescope_move_up();
                    } else if key_matches(key, &[k.move_down, k.move_down_alt]) {
                        app.telescope_move_down();
                    } else if let crossterm::event::KeyCode::Char(c) = key.code {
                        app.telescope_add_char(c);
                    }
                }
                Focus::CommandPalette => {
                    if key_matches(key, &[k.escape]) {
                        app.exit_command_palette();
                    } else if key_matches(key, &[k.enter]) {
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
                                    app.mark_editor_dirty();
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
                    } else if key_matches(key, &[k.backspace]) {
                        app.command_palette_backspace();
                    } else if key_matches(key, &[k.move_up, k.move_up_alt]) {
                        app.command_palette_move_up();
                    } else if key_matches(key, &[k.move_down, k.move_down_alt]) {
                        app.command_palette_move_down();
                    } else if let crossterm::event::KeyCode::Char(c) = key.code {
                        app.command_palette_add_char(c);
                    }
                }
                Focus::Rename => {
                    if key_matches(key, &[k.escape]) {
                        app.exit_rename();
                    } else if key_matches(key, &[k.enter]) {
                        let _ = app.rename_selected_note();
                    } else if key_matches(key, &[k.backspace]) {
                        app.rename_backspace();
                    } else if let crossterm::event::KeyCode::Char(c) = key.code {
                        app.rename_add_char(c);
                    }
                }
                Focus::Backlinks => {
                    if key_matches(key, &[k.escape]) {
                        app.focus = Focus::Editor;
                    } else if key_matches(key, &[k.move_up, k.move_up_alt]) {
                        app.backlinks_move_up();
                    } else if key_matches(key, &[k.move_down, k.move_down_alt]) {
                        app.backlinks_move_down();
                    } else if key_matches(key, &[k.enter]) {
                        let _ = app.open_selected_backlink();
                    }
                }
                Focus::CreatingDirectory => {
                    if key_matches(key, &[k.escape]) {
                        app.exit_create_directory();
                    } else if key_matches(key, &[k.enter]) {
                        let _ = app.create_directory();
                    } else if key_matches(key, &[k.backspace]) {
                        app.directory_backspace();
                    } else if let crossterm::event::KeyCode::Char(c) = key.code {
                        app.directory_add_char(c);
                    }
                }
                Focus::TaskView => {
                    if key_matches(key, &[k.escape]) {
                        app.exit_task_view();
                    } else if key_matches(key, &[k.move_up, k.move_up_alt]) {
                        app.task_move_up();
                    } else if key_matches(key, &[k.move_down, k.move_down_alt]) {
                        app.task_move_down();
                    } else if key_matches(key, &[k.enter]) {
                        let _ = app.open_selected_task();
                    }
                }
                Focus::TagExplorer => {
                    if key_matches(key, &[k.escape]) {
                        app.exit_tag_explorer();
                    } else if key_matches(key, &[k.move_up, k.move_up_alt]) {
                        if app.tag_explorer_view == TagExplorerView::TagList {
                            app.tag_list_move_up();
                        } else {
                            app.tag_file_move_up();
                        }
                    } else if key_matches(key, &[k.move_down, k.move_down_alt]) {
                        if app.tag_explorer_view == TagExplorerView::TagList {
                            app.tag_list_move_down();
                        } else {
                            app.tag_file_move_down();
                        }
                    } else if key_matches(key, &[k.enter]) {
                        if app.tag_explorer_view == TagExplorerView::TagList {
                            app.load_files_for_selected_tag();
                        } else {
                            let _ = app.open_selected_tag_file();
                        }
                    } else if key_matches(key, &[k.backspace, k.move_left, k.move_left_alt]) {
                        if app.tag_explorer_view == TagExplorerView::FileList {
                            app.tag_explorer_view = TagExplorerView::TagList;
                        }
                    }
                }
                Focus::List => {
                    if app.template_picker_active {
                        if key_matches(key, &[k.escape]) {
                            app.exit_template_picker();
                        } else if key_matches(key, &[k.enter]) {
                            if let Some(path) = app.create_note_with_template(app.get_selected_template())? {
                                let _ = app.load_file_into_editor(path);
                            }
                        } else if key_matches(key, &[k.move_up, k.move_up_alt]) {
                            app.template_picker_move_up();
                        } else if key_matches(key, &[k.move_down, k.move_down_alt]) {
                            app.template_picker_move_down();
                        }
                    } else {
                        match app.mode {
                            Mode::Normal => {
                                if key_matches(key, &[k.quit]) {
                                    let _ = app.save_editor();
                                    break;
                                }
                                if key_matches(key, &[k.move_up, k.move_up_alt]) {
                                    app.move_selection_up();
                                } else if key_matches(key, &[k.move_down, k.move_down_alt]) {
                                    app.move_selection_down();
                                } else if key_matches(key, &[k.search]) {
                                    app.enter_search_mode();
                                } else if key_matches(key, &[k.list_create_note]) {
                                    app.enter_create_mode();
                                } else if key_matches(key, &[k.list_create_dir]) {
                                    app.enter_create_directory();
                                } else if key_matches(key, &[k.list_tag_explorer]) {
                                    app.enter_tag_explorer();
                                } else if key_matches(key, &[k.list_rename]) {
                                    app.enter_rename();
                                } else if key_matches(key, &[k.list_edit_config]) {
                                    if let Ok(config_path) = config::config_file_path() {
                                        let _ = app.load_file_into_editor(config_path);
                                    }
                                } else if key_matches(key, &[k.list_delete, k.delete]) {
                                    let _ = app.delete_selected_note();
                                } else if key_matches(key, &[k.list_parent, k.list_parent_alt, k.move_left, k.move_left_alt]) {
                                    app.go_to_parent_dir();
                                } else if key_matches(key, &[k.enter]) {
                                    if !app.enter_selected_directory() {
                                        if let Some(path) = app.get_selected_path() {
                                            let _ = app.load_file_into_editor(path);
                                        }
                                    }
                                }
                            }
                            Mode::Search => {
                                if key_matches(key, &[k.escape]) {
                                    app.exit_search_mode();
                                } else if key_matches(key, &[k.enter]) {
                                    if app.enter_selected_directory() {
                                        app.exit_search_mode();
                                    } else if let Some(path) = app.get_selected_path() {
                                        let _ = app.load_file_into_editor(path);
                                        app.exit_search_mode();
                                    }
                                } else if key_matches(key, &[k.backspace]) {
                                    app.search_backspace();
                                } else if let crossterm::event::KeyCode::Char(c) = key.code {
                                    app.search_add_char(c);
                                }
                            }
                            Mode::Create => {
                                if key_matches(key, &[k.escape]) {
                                    app.exit_create_mode();
                                } else if key_matches(key, &[k.enter]) {
                                    app.enter_template_picker();
                                } else if key_matches(key, &[k.backspace]) {
                                    app.create_backspace();
                                } else if let crossterm::event::KeyCode::Char(c) = key.code {
                                    app.create_add_char(c);
                                }
                            }
                        }
                    }
                }
                Focus::Editor => {
                    if key_matches(key, &[k.editor_pdf]) {
                        let _ = app.export_to_pdf();
                        continue;
                    }
                    if key_matches(key, &[k.editor_backlinks]) && app.config.editor.show_backlinks {
                        app.focus = Focus::Backlinks;
                        continue;
                    }
                    if app.editor_layout == EditorLayout::SplitVertical
                        && app.split_right_tab.is_some()
                        && key_matches(key, &[k.editor_split_focus])
                    {
                        app.split_focus_left = !app.split_focus_left;
                        continue;
                    }

                    if app.editor_mode == EditorMode::Normal
                        && (key_matches(key, &[k.enter]) || key_matches(key, &[k.editor_wiki_link]))
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
                            if key_matches(key, &[k.escape]) {
                                app.editor_mode = EditorMode::Normal;
                            } else {
                                app.mark_editor_dirty();
                                if let Some(buf) = app.focused_buffer_mut() {
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
