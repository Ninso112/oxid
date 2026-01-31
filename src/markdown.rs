// SPDX-License-Identifier: GPL-3.0-or-later
// oxid - Markdown rendering for preview pane

use crate::theme::ResolvedTheme;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

/// Render markdown content to ratatui Lines with theme styling.
pub fn render_markdown(content: &str, theme: &ResolvedTheme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut block_stack: Vec<BlockStyle> = vec![BlockStyle::Paragraph];
    let mut list_item_counter: Option<u64> = None;
    let mut list_item_prefix = "• ".to_string();
    let mut task_list_checked: Option<bool> = None;

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(content, opts);

    for event in parser {
        match event {
            Event::Start(tag) => {
                match tag {
                    Tag::Heading(_, _, _) => {
                        flush_line(&mut current_line, &mut lines);
                        block_stack.push(BlockStyle::Heading);
                    }
                    Tag::CodeBlock(CodeBlockKind::Fenced(_) | CodeBlockKind::Indented) => {
                        flush_line(&mut current_line, &mut lines);
                        block_stack.push(BlockStyle::CodeBlock);
                    }
                    Tag::List(numbering) => {
                        flush_line(&mut current_line, &mut lines);
                        list_item_counter = numbering;
                        block_stack.push(BlockStyle::List);
                    }
                    Tag::Item => {
                        if !current_line.is_empty() {
                            flush_line(&mut current_line, &mut lines);
                        }
                        task_list_checked = None;
                        list_item_prefix = match list_item_counter {
                            Some(n) => {
                                let prefix = format!("{}. ", n);
                                list_item_counter = Some(n + 1);
                                prefix
                            }
                            None => "• ".to_string(),
                        };
                        block_stack.push(BlockStyle::ListItem);
                    }
                    Tag::Paragraph => {
                        if !matches!(block_stack.last(), Some(BlockStyle::ListItem)) {
                            flush_line(&mut current_line, &mut lines);
                        }
                        block_stack.push(BlockStyle::Paragraph);
                    }
                    Tag::Strong | Tag::Emphasis => {
                        block_stack.push(BlockStyle::Bold);
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                match tag {
                    Tag::Heading(_, _, _)
                    | Tag::CodeBlock(_)
                    | Tag::List(_)
                    | Tag::Paragraph => {
                        flush_line(&mut current_line, &mut lines);
                        let _ = block_stack.pop();
                    }
                    Tag::Item => {
                        flush_line(&mut current_line, &mut lines);
                        task_list_checked = None;
                        let _ = block_stack.pop();
                    }
                    Tag::Strong | Tag::Emphasis => {
                        let _ = block_stack.pop();
                    }
                    _ => {}
                }
            }
            Event::TaskListMarker(checked) => {
                task_list_checked = Some(checked);
                if matches!(block_stack.last(), Some(BlockStyle::ListItem))
                    && current_line.is_empty()
                {
                    current_line.push(Span::styled(
                        list_item_prefix.clone(),
                        theme.md_list_marker_style,
                    ));
                }
                let marker = if checked { "[x] " } else { "[ ] " };
                let style = if checked {
                    theme.editor_checkbox_checked_style
                        .patch(theme.preview_text_style)
                } else {
                    theme.editor_checkbox_style.patch(theme.preview_text_style)
                };
                current_line.push(Span::styled(marker.to_string(), style));
            }
            Event::Text(text) => {
                let base_style = block_style(&block_stack, theme);
                let style = if let Some(checked) = task_list_checked {
                    if checked {
                        theme.editor_checkbox_checked_style.patch(base_style)
                    } else {
                        theme.editor_checkbox_style.patch(base_style)
                    }
                } else {
                    base_style
                };
                let prefix = if matches!(block_stack.last(), Some(BlockStyle::ListItem))
                    && current_line.is_empty()
                {
                    list_item_prefix.clone()
                } else {
                    String::new()
                };
                if !prefix.is_empty() {
                    current_line.push(Span::styled(
                        prefix,
                        theme.md_list_marker_style,
                    ));
                }
                current_line.push(Span::styled(text.to_string(), style));
                task_list_checked = None;
            }
            Event::Code(text) => {
                let style = theme
                    .preview_text_style
                    .patch(theme.md_code_bg_style);
                current_line.push(Span::styled(text.to_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut current_line, &mut lines);
            }
            Event::Rule => {
                flush_line(&mut current_line, &mut lines);
                lines.push(Line::from(Span::styled(
                    "─".repeat(20),
                    theme.preview_text_style,
                )));
            }
            _ => {}
        }
    }

    flush_line(&mut current_line, &mut lines);

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "",
            theme.preview_text_style,
        )));
    }

    lines
}

#[derive(Clone, Copy, PartialEq)]
enum BlockStyle {
    Paragraph,
    Heading,
    CodeBlock,
    List,
    ListItem,
    Bold,
}

fn block_style(stack: &[BlockStyle], theme: &ResolvedTheme) -> ratatui::style::Style {
    for s in stack.iter().rev() {
        match s {
            BlockStyle::Heading => return theme.md_header_fg_style,
            BlockStyle::CodeBlock => {
                return theme
                    .preview_text_style
                    .patch(theme.md_code_bg_style);
            }
            BlockStyle::Bold => {
                return theme
                    .preview_text_style
                    .add_modifier(Modifier::BOLD);
            }
            _ => {}
        }
    }
    theme.preview_text_style
}

fn flush_line(spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(std::mem::take(spans)));
    }
}
