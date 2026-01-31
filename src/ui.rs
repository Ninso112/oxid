// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

use crate::app::{App, EditorLayout, Focus, Mode};
use crate::git::GitStatus;
use crate::markdown::render_markdown;
use crate::templates::Template;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

fn border_type_from_config(border_style: &str) -> BorderType {
    match border_style.trim().to_lowercase().as_str() {
        "double" => BorderType::Double,
        "thick" => BorderType::Thick,
        "plain" => BorderType::Plain,
        _ => BorderType::Rounded,
    }
}

/// Build a Line with search match highlighting. Indices are character positions.
fn build_highlighted_line(
    text: &str,
    match_indices: Vec<u32>,
    base_style: Style,
    match_style: Style,
) -> Line<'static> {
    if match_indices.is_empty() {
        return Line::from(Span::styled(text.to_string(), base_style));
    }
    let match_set: std::collections::HashSet<u32> =
        match_indices.into_iter().collect();
    let mut spans = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let style = if match_set.contains(&(i as u32)) {
            match_style
        } else {
            base_style
        };
        spans.push(Span::styled(c.to_string(), style));
    }
    Line::from(spans)
}

/// Build a Line with substring match highlighting (case-insensitive) for preview pane.
fn build_preview_line_with_highlight(
    line: &str,
    query: &str,
    base_style: Style,
    match_style: Style,
) -> Line<'static> {
    if query.is_empty() {
        return Line::from(Span::styled(line.to_string(), base_style));
    }
    let query_lower = query.to_lowercase();
    let line_lower = line.to_lowercase();
    let match_len = query_lower.len();

    let mut spans = Vec::new();
    let mut remaining = line;
    let mut search_start = 0;

    while let Some(rel_start) = line_lower[search_start..].find(&query_lower) {
        let before_match = &remaining[..rel_start];
        let matched = &remaining[rel_start..rel_start + match_len];
        remaining = &remaining[rel_start + match_len..];
        search_start += rel_start + match_len;

        if !before_match.is_empty() {
            spans.push(Span::styled(before_match.to_string(), base_style));
        }
        spans.push(Span::styled(matched.to_string(), match_style));
    }
    if !remaining.is_empty() {
        spans.push(Span::styled(remaining.to_string(), base_style));
    }

    if spans.is_empty() {
        Line::from(Span::styled(line.to_string(), base_style))
    } else {
        Line::from(spans)
    }
}

/// Center a rect within area with given size.
fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let popup_height = area.height * percent_y / 100;
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    Rect {
        x: area.x + x,
        y: area.y + y,
        width: popup_width,
        height: popup_height,
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    frame.render_widget(
        Block::default().style(app.theme.app_background_style),
        area,
    );

    // Draw popups on top
    if app.focus == Focus::Search {
        draw_telescope_popup(frame, app, area);
        return;
    }
    if app.focus == Focus::CommandPalette {
        draw_command_palette_popup(frame, app, area);
        return;
    }
    if app.tag_explorer_active {
        draw_tag_explorer_popup(frame, app, area);
        return;
    }
    if app.task_view_active {
        draw_task_view_popup(frame, app, area);
        return;
    }
    if app.focus == Focus::Rename {
        draw_rename_popup(frame, app, area);
        return;
    }
    if app.focus == Focus::CreatingDirectory {
        draw_create_directory_popup(frame, app, area);
        return;
    }
    if app.template_picker_active {
        draw_template_picker_popup(frame, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, app, chunks[0]);
    draw_tab_bar(frame, app, chunks[1]);
    let main_area = chunks[2];

    if app.zen_mode {
        draw_editor_pane(frame, app, main_area);
    } else if app.config.editor.show_backlinks {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(main_area);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(40),
                Constraint::Percentage(40),
            ])
            .split(vertical_chunks[0]);

        draw_notes_list(frame, app, main_chunks[0]);
        if app.editor_layout == EditorLayout::SplitVertical {
            let editor_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);
            draw_editor_pane_at(frame, app, editor_chunks[0], app.active_tab);
            if let Some(right_idx) = app.split_right_tab {
                draw_editor_pane_at(frame, app, editor_chunks[1], right_idx);
            }
        } else {
            draw_editor_pane(frame, app, main_chunks[1]);
        }
        draw_preview_pane(frame, app, main_chunks[2]);
        draw_backlinks_pane(frame, app, vertical_chunks[1]);
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(40),
                Constraint::Percentage(40),
            ])
            .split(main_area);

        draw_notes_list(frame, app, main_chunks[0]);
        if app.editor_layout == EditorLayout::SplitVertical {
            let editor_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(main_chunks[1]);
            draw_editor_pane_at(frame, app, editor_chunks[0], app.active_tab);
            if let Some(right_idx) = app.split_right_tab {
                draw_editor_pane_at(frame, app, editor_chunks[1], right_idx);
            }
        } else {
            draw_editor_pane(frame, app, main_chunks[1]);
        }
        draw_preview_pane(frame, app, main_chunks[2]);
    }

    draw_footer(frame, app, chunks[3]);
}

fn draw_telescope_popup(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" {} │ Open File ", app.get_key_display_string("search")))
        .borders(Borders::ALL)
        .border_type(border_type_from_config(&app.config.ui.border_style))
        .border_style(app.theme.list_border_active_style);
    let popup_area = centered_rect(area, 70, 60);
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let query_line = Line::from(vec![
        Span::styled("> ", app.theme.highlight_style),
        Span::styled(&app.telescope_query, app.theme.text_style),
    ]);
    frame.render_widget(Paragraph::new(query_line), chunks[0]);

    let items: Vec<ListItem> = app
        .telescope_filtered
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let base_style = if i == app.telescope_selected {
                app.theme.list_text_selected_style
            } else {
                app.theme.list_text_normal_style
            };
            let line = if !app.telescope_query.is_empty() && !app.telescope_query.starts_with('#') {
                build_highlighted_line(
                    &note.display,
                    app.telescope_match_indices.get(i).cloned().unwrap_or_default(),
                    base_style,
                    app.theme.search_match_style,
                )
            } else {
                Line::from(Span::styled(note.display.as_str(), base_style))
            };
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

fn draw_command_palette_popup(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" {} │ Command Palette ", app.get_key_display_string("command_palette")))
        .borders(Borders::ALL)
        .border_type(border_type_from_config(&app.config.ui.border_style))
        .border_style(app.theme.list_border_active_style);
    let popup_area = centered_rect(area, 50, 40);
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    let query_line = Line::from(vec![
        Span::styled("> ", app.theme.highlight_style),
        Span::styled(&app.command_palette_query, app.theme.text_style),
    ]);
    frame.render_widget(Paragraph::new(query_line), chunks[0]);

    let items: Vec<ListItem> = app
        .command_palette_filtered
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == app.command_palette_selected {
                app.theme.list_text_selected_style
            } else {
                app.theme.list_text_normal_style
            };
            ListItem::new(Line::from(Span::styled(
                action.label(),
                style,
            )))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);
}

fn draw_rename_popup(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" {} │ Rename File ", app.get_key_display_string("list_rename")))
        .borders(Borders::ALL)
        .border_type(border_type_from_config(&app.config.ui.border_style))
        .border_style(app.theme.list_border_active_style);
    let popup_area = centered_rect(area, 50, 15);
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let content = Line::from(vec![
        Span::styled("New name: ", app.theme.help_text_style),
        Span::styled(&app.rename_input, app.theme.highlight_style),
    ]);
    frame.render_widget(Paragraph::new(content), inner);
}

fn draw_tag_explorer_popup(frame: &mut Frame, app: &App, area: Rect) {
    use crate::app::TagExplorerView;
    
    let popup_area = {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(area);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(vertical[1])[1]
    };

    frame.render_widget(Clear, popup_area);

    if app.tag_explorer_view == TagExplorerView::TagList {
        let items: Vec<ListItem> = app
            .all_tags
            .iter()
            .enumerate()
            .map(|(i, tag)| {
                let style = if i == app.tag_selected {
                    app.theme.list_text_selected_style
                } else {
                    app.theme.list_text_normal_style
                };
                ListItem::new(Line::from(Span::styled(format!("#{}", tag), style)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(format!(" Tag Explorer ({} tags) ", app.all_tags.len()))
                .borders(Borders::ALL)
                .border_type(border_type_from_config(&app.config.ui.border_style))
                .border_style(app.theme.border_style),
        );
        frame.render_widget(list, popup_area);
    } else {
        let selected_tag = app.all_tags.get(app.tag_selected).map(|s| s.as_str()).unwrap_or("");
        let items: Vec<ListItem> = app
            .tag_files
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let display = path
                    .strip_prefix(&app.notes_dir)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string()
                    });
                let style = if i == app.tag_file_selected {
                    app.theme.list_text_selected_style
                } else {
                    app.theme.list_text_normal_style
                };
                ListItem::new(Line::from(Span::styled(display, style)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(format!(" Files with #{} ({} files) ", selected_tag, app.tag_files.len()))
                .borders(Borders::ALL)
                .border_type(border_type_from_config(&app.config.ui.border_style))
                .border_style(app.theme.border_style),
        );
        frame.render_widget(list, popup_area);
    }
}

fn draw_task_view_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(area);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(vertical[1])[1]
    };

    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let rel_path = task
                .path
                .strip_prefix(&app.notes_dir)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| {
                    task.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                });
            let display = if task.content.is_empty() {
                format!("(empty) [{}]", rel_path)
            } else {
                format!("{} [{}]", task.content, rel_path)
            };
            let style = if i == app.task_selected {
                app.theme.list_text_selected_style
            } else {
                app.theme.list_text_normal_style
            };
            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(" Task Board ({} tasks) │ {}/{} move │ {} open │ {} close ", app.tasks.len(), app.get_key_display_string("move_down"), app.get_key_display_string("move_up"), app.get_key_display_string("enter"), app.get_key_display_string("escape")))
            .borders(Borders::ALL)
            .border_type(border_type_from_config(&app.config.ui.border_style))
            .border_style(app.theme.border_style),
    );
    frame.render_widget(list, popup_area);
}

fn draw_create_directory_popup(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" {} │ New Directory ", app.get_key_display_string("list_create_dir")))
        .borders(Borders::ALL)
        .border_type(border_type_from_config(&app.config.ui.border_style))
        .border_style(app.theme.list_border_active_style);
    let popup_area = centered_rect(area, 50, 15);
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let content = Line::from(vec![
        Span::styled("New directory name: ", app.theme.help_text_style),
        Span::styled(&app.directory_input, app.theme.highlight_style),
    ]);
    frame.render_widget(Paragraph::new(content), inner);
}

fn draw_template_picker_popup(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" New Note │ Choose Template ")
        .borders(Borders::ALL)
        .border_type(border_type_from_config(&app.config.ui.border_style))
        .border_style(app.theme.list_border_active_style);
    let popup_area = centered_rect(area, 40, 30);
    let inner = block.inner(popup_area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(block, popup_area);

    let items: Vec<ListItem> = Template::all()
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let style = if i == app.template_picker_selected {
                app.theme.list_text_selected_style
            } else {
                app.theme.list_text_normal_style
            };
            ListItem::new(Line::from(Span::styled(t.name(), style)))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.zen_mode {
        "⚡ Oxid - Zen Mode"
    } else {
        "⚡ Oxid - TUI Note Editor"
    };
    let header = Paragraph::new(title)
        .style(app.theme.header_style)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(app.theme.border_style),
        );
    frame.render_widget(header, area);
}

fn draw_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let tab_spans: Vec<Span> = app
        .buffers
        .iter()
        .enumerate()
        .flat_map(|(i, buf)| {
            let is_active = i == app.active_tab
                || (app.editor_layout == EditorLayout::SplitVertical
                    && app.split_right_tab == Some(i));
            let is_focused = i == app.focused_buffer_index();
            let style = if is_focused {
                app.theme.list_text_selected_style
            } else if is_active {
                app.theme.highlight_style
            } else {
                app.theme.list_text_normal_style
            };
            let name = buf.display_name();
            let sep = if i + 1 < app.buffers.len() {
                Span::styled(" │ ", app.theme.help_text_style)
            } else {
                Span::raw("")
            };
            vec![Span::styled(format!(" {} ", name), style), sep]
        })
        .collect();
    let line = if tab_spans.is_empty() {
        Line::from(Span::styled(" (no files open) ", app.theme.help_text_style))
    } else {
        Line::from(tab_spans)
    };
    let tab_bar = Paragraph::new(line)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_type(border_type_from_config(&app.config.ui.border_style))
                .border_style(app.theme.border_style),
        );
    frame.render_widget(tab_bar, area);
}

fn draw_notes_list(frame: &mut Frame, app: &App, area: Rect) {
    let list_border_style = match app.focus {
        Focus::List => app.theme.list_border_active_style,
        _ => app.theme.list_border_inactive_style,
    };

    let mode = match app.mode {
        Mode::Create => app.theme.list_border_inactive_style,
        _ => list_border_style,
    };

    let items: Vec<ListItem> = app
        .filtered_notes
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let base_style = if i == app.selected {
                if note.is_directory {
                    app.theme.list_directory_style.patch(app.theme.list_text_selected_style)
                } else {
                    app.theme.list_text_selected_style
                }
            } else if note.is_directory {
                app.theme.list_directory_style
            } else {
                app.theme.list_text_normal_style
            };
            let icon = app.file_icon(&note.path);
            let display_text = format!("{}{}", icon, note.display);
            let line = if app.mode == Mode::Search && !app.search_query.is_empty() {
                let offset = icon.chars().count() as u32;
                let shifted: Vec<u32> = app
                    .match_indices
                    .get(i)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|idx| idx + offset)
                    .collect();
                build_highlighted_line(
                    &display_text,
                    shifted,
                    base_style,
                    app.theme.search_match_style,
                )
            } else {
                Line::from(Span::styled(display_text, base_style))
            };
            ListItem::new(line)
        })
        .collect();

    let list_title = if app.current_dir == app.notes_dir {
        " Notes ".to_string()
    } else {
        format!(
            " Notes ({}) ",
            app.current_dir
                .strip_prefix(&app.notes_dir)
                .map(|p| format!(".../{}", p.display()))
                .unwrap_or_else(|_| app.current_dir.display().to_string())
        )
    };
    let border_type = border_type_from_config(&app.config.ui.border_style);
    let list = List::new(items).block(
        Block::default()
            .title(list_title)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(mode),
    );
    frame.render_widget(list, area);
}

fn draw_editor_pane(frame: &mut Frame, app: &App, area: Rect) {
    let buf_idx = app.active_tab;
    draw_editor_pane_at(frame, app, area, buf_idx);
}

fn draw_editor_pane_at(frame: &mut Frame, app: &App, area: Rect, buf_idx: usize) {
    let is_focused = app.focus == Focus::Editor && app.focused_buffer_index() == buf_idx;
    let editor_border_style = if is_focused {
        app.theme.preview_border_active_style
    } else {
        app.theme.preview_border_inactive_style
    };

    let buf = match app.buffers.get(buf_idx) {
        Some(b) => b,
        None => {
            let block = Block::default()
                .title(" Editor ")
                .borders(Borders::ALL)
                .border_style(editor_border_style);
            let placeholder = Paragraph::new("(Select a note with Enter)")
                .style(
                    app.theme.editor_fg_style
                        .patch(app.theme.editor_bg_style)
                        .add_modifier(Modifier::ITALIC),
                )
                .block(block);
            frame.render_widget(placeholder, area);
            return;
        }
    };

    let title = format!(" {} ", buf.display_name());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(border_type_from_config(&app.config.ui.border_style))
        .border_style(editor_border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(&buf.textarea, inner);
}

fn draw_preview_pane(frame: &mut Frame, app: &App, area: Rect) {
    let preview_border_style = match app.focus {
        Focus::Editor => app.theme.preview_border_active_style,
        _ => app.theme.preview_border_inactive_style,
    };

    let mode = match app.mode {
        Mode::Create => app.theme.preview_border_inactive_style,
        _ => preview_border_style,
    };

    let content = if let Some(placeholder) = app.get_preview_placeholder() {
        vec![Line::from(Span::styled(
            placeholder,
            app.theme.preview_text_style.add_modifier(Modifier::ITALIC),
        ))]
    } else {
        let preview_text = app.get_preview_content();
        if preview_text.is_empty() && app.filtered_notes.is_empty() {
            vec![Line::from(Span::styled(
                "(No notes - press n to create)",
                app.theme.preview_text_style.add_modifier(Modifier::ITALIC),
            ))]
        } else if preview_text.is_empty() {
            vec![Line::from(Span::styled(
                "(Select a note to preview)",
                app.theme.preview_text_style.add_modifier(Modifier::ITALIC),
            ))]
        } else if !app.search_query.is_empty() {
            preview_text
                .lines()
                .map(|l| {
                    build_preview_line_with_highlight(
                        l,
                        &app.search_query,
                        app.theme.preview_text_style,
                        app.theme.search_match_style,
                    )
                })
                .collect()
        } else {
            render_markdown(&preview_text, &app.theme)
        }
    };

    let paragraph = Paragraph::new(content)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Preview ")
                .borders(Borders::ALL)
                .border_style(mode),
        );
    frame.render_widget(paragraph, area);
}

fn draw_backlinks_pane(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == Focus::Backlinks {
        app.theme.preview_border_active_style
    } else {
        app.theme.preview_border_inactive_style
    };

    let items: Vec<ListItem> = app
        .backlinks
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let display = path
                .strip_prefix(&app.notes_dir)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                });
            let style = if i == app.backlinks_selected {
                app.theme.list_text_selected_style
            } else {
                app.theme.list_text_normal_style
            };
            ListItem::new(Line::from(Span::styled(display, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(" Backlinks ({}) ", app.backlinks.len()))
            .borders(Borders::ALL)
            .border_type(border_type_from_config(&app.config.ui.border_style))
            .border_style(border_style),
    );
    frame.render_widget(list, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let (title, content) = if app.focus == Focus::Backlinks {
        (
            " Backlinks ",
            vec![
                Span::styled(format!("{}/{} ", app.get_key_display_string("move_down"), app.get_key_display_string("move_up")), app.theme.help_text_style),
                Span::styled("navigate", app.theme.highlight_style),
                Span::styled(format!(" | {} ", app.get_key_display_string("enter")), app.theme.help_text_style),
                Span::styled("open", app.theme.highlight_style),
                Span::styled(format!(" | {} ", app.get_key_display_string("escape")), app.theme.help_text_style),
                Span::styled("back", app.theme.highlight_style),
            ],
        )
    } else if app.focus == Focus::Editor {
        (
            " Editor ",
            vec![
                Span::styled(format!("{}/{} ", app.get_key_display_string("editor_insert"), app.get_key_display_string("editor_append")), app.theme.help_text_style),
                Span::styled("insert", app.theme.highlight_style),
                Span::styled(format!(" | {} ", app.get_key_display_string("escape")), app.theme.help_text_style),
                Span::styled("normal", app.theme.highlight_style),
                Span::styled(format!(" | {} ", app.get_key_display_string("editor_back")), app.theme.help_text_style),
                Span::styled("back", app.theme.highlight_style),
            ],
        )
    } else {
        match app.mode {
            Mode::Normal => (
                " Normal ",
                vec![
                    Span::styled(format!("{} ", app.get_key_display_string("search")), app.theme.help_text_style),
                    Span::styled("search", app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("command_palette")), app.theme.help_text_style),
                    Span::styled("commands", app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("list_rename")), app.theme.help_text_style),
                    Span::styled("rename", app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("list_create_dir")), app.theme.help_text_style),
                    Span::styled("mkdir", app.theme.highlight_style),
                    Span::styled(format!(" | {}/{} ", app.get_key_display_string("move_left_alt"), app.get_key_display_string("move_left")), app.theme.help_text_style),
                    Span::styled("up", app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("zen_mode")), app.theme.help_text_style),
                    Span::styled("zen", app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("quit")), app.theme.help_text_style),
                    Span::styled("quit", app.theme.highlight_style),
                ],
            ),
            Mode::Search => (
                " Search ",
                vec![
                    Span::styled(&app.search_query, app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("escape")), app.theme.help_text_style),
                    Span::styled("back", app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("enter")), app.theme.help_text_style),
                    Span::styled("edit", app.theme.highlight_style),
                ],
            ),
            Mode::Create => (
                " New Note ",
                vec![
                    Span::styled("Filename: ", app.theme.help_text_style),
                    Span::styled(&app.create_filename, app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("enter")), app.theme.help_text_style),
                    Span::styled("template", app.theme.highlight_style),
                    Span::styled(format!(" | {} ", app.get_key_display_string("escape")), app.theme.help_text_style),
                    Span::styled("cancel", app.theme.highlight_style),
                ],
            ),
        }
    };

    let mut spans = content;

    // Git status indicator (uses theme statusbar styles)
    match app.git_status() {
        GitStatus::Clean => spans.push(Span::styled(
            " | Git: Clean ",
            app.theme.statusbar_fg_style,
        )),
        GitStatus::Dirty => spans.push(Span::styled(
            " | Git: Dirty ",
            app.theme.highlight_style,
        )),
        GitStatus::Unknown => {}
    }

    if app.save_indicator_until.is_some() {
        spans.push(Span::styled(
            " | Saved... ",
            app.theme.highlight_style.add_modifier(Modifier::ITALIC),
        ));
    }

    let mut lines = vec![Line::from(spans)];

    if let Some(msg) = &app.message {
        lines.push(Line::from(Span::styled(
            msg.as_str(),
            app.theme.text_style.add_modifier(Modifier::ITALIC),
        )));
    }

    let border_type = border_type_from_config(&app.config.ui.border_style);
    let footer = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(border_type)
                .border_style(app.theme.border_style)
                .style(app.theme.statusbar_bg_style),
        );
    frame.render_widget(footer, area);
}
