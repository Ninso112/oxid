// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - A fast, keyboard-driven note manager TUI for Linux

use crate::app::{App, EditorLayout, Focus, Mode};
use crate::git::GitStatus;
use crate::markdown::render_markdown;
use crate::templates::Template;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

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
        .title(" / │ Open File ")
        .borders(Borders::ALL)
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
        .title(" Ctrl+p │ Command Palette ")
        .borders(Borders::ALL)
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
        .title(" r │ Rename File ")
        .borders(Borders::ALL)
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

fn draw_create_directory_popup(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Shift+n │ New Directory ")
        .borders(Borders::ALL)
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
                app.theme.list_text_selected_style
            } else {
                app.theme.list_text_normal_style
            };
            let line = if app.mode == Mode::Search && !app.search_query.is_empty() {
                build_highlighted_line(
                    &note.display,
                    app.match_indices.get(i).cloned().unwrap_or_default(),
                    base_style,
                    app.theme.search_match_style,
                )
            } else {
                Line::from(Span::styled(note.display.as_str(), base_style))
            };
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Notes ")
            .borders(Borders::ALL)
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

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let (title, content) = if app.focus == Focus::Editor {
        (
            " Editor ",
            vec![
                Span::styled("i/a ", app.theme.help_text_style),
                Span::styled("insert", app.theme.highlight_style),
                Span::styled(" | Esc ", app.theme.help_text_style),
                Span::styled("normal", app.theme.highlight_style),
                Span::styled(" | Ctrl+Space ", app.theme.help_text_style),
                Span::styled("checkboxes", app.theme.highlight_style),
                Span::styled(" | q ", app.theme.help_text_style),
                Span::styled("back", app.theme.highlight_style),
            ],
        )
    } else {
        match app.mode {
            Mode::Normal => (
                " Normal ",
                vec![
                    Span::styled("/ ", app.theme.help_text_style),
                    Span::styled("search", app.theme.highlight_style),
                    Span::styled(" | Ctrl+p ", app.theme.help_text_style),
                    Span::styled("commands", app.theme.highlight_style),
                    Span::styled(" | r ", app.theme.help_text_style),
                    Span::styled("rename", app.theme.highlight_style),
                    Span::styled(" | N ", app.theme.help_text_style),
                    Span::styled("mkdir", app.theme.highlight_style),
                    Span::styled(" | F11 ", app.theme.help_text_style),
                    Span::styled("zen", app.theme.highlight_style),
                    Span::styled(" | q ", app.theme.help_text_style),
                    Span::styled("quit", app.theme.highlight_style),
                ],
            ),
            Mode::Search => (
                " Search ",
                vec![
                    Span::styled(&app.search_query, app.theme.highlight_style),
                    Span::styled(" | Esc ", app.theme.help_text_style),
                    Span::styled("back", app.theme.highlight_style),
                    Span::styled(" | Enter ", app.theme.help_text_style),
                    Span::styled("edit", app.theme.highlight_style),
                ],
            ),
            Mode::Create => (
                " New Note ",
                vec![
                    Span::styled("Filename: ", app.theme.help_text_style),
                    Span::styled(&app.create_filename, app.theme.highlight_style),
                    Span::styled(" | Enter ", app.theme.help_text_style),
                    Span::styled("template", app.theme.highlight_style),
                    Span::styled(" | Esc ", app.theme.help_text_style),
                    Span::styled("cancel", app.theme.highlight_style),
                ],
            ),
        }
    };

    let mut spans = content;

    // Git status indicator
    match app.git_status() {
        GitStatus::Clean => spans.push(Span::styled(
            " | Git: Clean ",
            ratatui::style::Style::default().fg(ratatui::style::Color::Green),
        )),
        GitStatus::Dirty => spans.push(Span::styled(
            " | Git: Dirty ",
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow),
        )),
        GitStatus::Unknown => {}
    }

    let mut lines = vec![Line::from(spans)];

    if let Some(msg) = &app.message {
        lines.push(Line::from(Span::styled(
            msg.as_str(),
            app.theme.text_style.add_modifier(Modifier::ITALIC),
        )));
    }

    let footer = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(app.theme.border_style),
        );
    frame.render_widget(footer, area);
}
