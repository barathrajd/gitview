use crate::ui::Viewport;
use crate::worktree::{Entry, Section};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Paragraph, Wrap},
};

use super::diff_layout::{
    aligned_ide_diff_lines, aligned_ide_unified_lines, aligned_raw_diff_lines,
    styled_numbered_unified,
};
use super::diff_rows::cached_ide_diff_rows;
use super::hints::{hint_row_count, render_key_hints};
use super::prepared::PreparedDiff;
use super::theme::page_block;

pub(super) fn render_diff(
    frame: &mut Viewport<'_, '_>,
    diff: &(u64, Option<Result<String, String>>),
    scroll: u16,
    selected_hunk: usize,
    raw: bool,
    side_by_side: bool,
    worktree_entry: Option<&Entry>,
    border: ratatui::symbols::border::Set<'static>,
    prepared: Option<&PreparedDiff>,
) {
    let message = match &diff.1 {
        None => "Loading diff...",
        Some(Err(error)) => error,
        Some(Ok(text)) if text.is_empty() => "No textual diff",
        Some(Ok(text)) => text,
    };
    let hunk_count = prepared.map_or_else(
        || {
            diff.1
                .as_ref()
                .and_then(|result| result.as_ref().ok())
                .map(|text| text.lines().filter(|line| line.starts_with("@@ ")).count())
                .unwrap_or(0)
        },
        |prepared| prepared.hunk_count,
    );
    let selectable_hunks =
        worktree_entry.is_some_and(|entry| entry.section != Section::Untracked) && hunk_count > 0;
    let hints = if worktree_entry.is_some() {
        let compact = frame.area().width <= 80;
        let mut hints = vec![
            (
                "v",
                if raw {
                    if compact { "IDE" } else { "IDE diff" }
                } else if compact {
                    "raw"
                } else {
                    "raw patch"
                },
            ),
            (
                "Tab",
                if side_by_side {
                    if compact { "unified" } else { "unified view" }
                } else if compact {
                    "split"
                } else {
                    "side-by-side"
                },
            ),
            ("s", if compact { "stage" } else { "stage hunk" }),
            ("u", if compact { "unstage" } else { "unstage hunk" }),
            ("S", "stage all"),
            ("U", "unstage all"),
        ];
        if selectable_hunks {
            hints.insert(0, ("[/]", "hunk"));
        }
        hints
    } else {
        vec![
            ("v", if raw { "IDE diff" } else { "raw patch" }),
            (
                "Tab",
                if side_by_side {
                    "unified view"
                } else {
                    "side-by-side"
                },
            ),
            ("Up/Down or j/k", "scroll"),
            ("Esc/h", "back"),
        ]
    };
    let sections = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(hint_row_count(&hints, frame.area().width)),
    ])
    .split(frame.area());
    if let Some(Ok(text)) = &diff.1 {
        if side_by_side {
            let panes =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(sections[0]);
            let content_width = usize::from(panes[0].width.min(panes[1].width))
                .saturating_sub(2)
                .saturating_sub(6)
                .max(1);
            let (left, right, empty_hint) = if let Some(prepared) = prepared {
                let layout = prepared.layout(frame.area().width, raw, true);
                (
                    layout
                        .left
                        .iter()
                        .skip(usize::from(scroll))
                        .take(usize::from(panes[0].height.saturating_sub(2)))
                        .cloned()
                        .collect(),
                    layout
                        .right
                        .iter()
                        .skip(usize::from(scroll))
                        .take(usize::from(panes[1].height.saturating_sub(2)))
                        .cloned()
                        .collect(),
                    "",
                )
            } else if raw {
                let (left, right) = aligned_raw_diff_lines(
                    &crate::diff::numbered_side_by_side(text),
                    content_width,
                );
                (left, right, "No file lines · Tab for unified raw patch")
            } else {
                let rows = cached_ide_diff_rows(diff.0, message, worktree_entry);
                let (left, right) = aligned_ide_diff_lines(&rows, content_width);
                (left, right, "No source lines · press v for raw patch")
            };
            let left = if left.is_empty() {
                vec![Line::raw(empty_hint)]
            } else {
                left
            };
            let right = if right.is_empty() {
                vec![Line::raw(empty_hint)]
            } else {
                right
            };
            frame.render_widget(
                Paragraph::new(Text::from(left))
                    .scroll((if prepared.is_some() { 0 } else { scroll }, 0))
                    .wrap(Wrap { trim: false })
                    .block(page_block(
                        if selectable_hunks {
                            format!(
                                "BEFORE · HUNK {}/{}",
                                selected_hunk.saturating_add(1),
                                hunk_count
                            )
                        } else {
                            "BEFORE".to_owned()
                        },
                        border,
                    )),
                panes[0],
            );
            frame.render_widget(
                Paragraph::new(Text::from(right))
                    .scroll((if prepared.is_some() { 0 } else { scroll }, 0))
                    .wrap(Wrap { trim: false })
                    .block(page_block(
                        if selectable_hunks {
                            format!(
                                "AFTER · HUNK {}/{}",
                                selected_hunk.saturating_add(1),
                                hunk_count
                            )
                        } else {
                            "AFTER".to_owned()
                        },
                        border,
                    )),
                panes[1],
            );
        } else {
            let content_width = usize::from(sections[0].width)
                .saturating_sub(2)
                .saturating_sub(14)
                .max(1);
            let (lines, title) = if let Some(prepared) = prepared {
                let layout = prepared.layout(frame.area().width, raw, false);
                (
                    layout
                        .left
                        .iter()
                        .skip(usize::from(scroll))
                        .take(usize::from(sections[0].height.saturating_sub(2)))
                        .cloned()
                        .collect(),
                    if raw { "RAW PATCH" } else { "UNIFIED IDE DIFF" },
                )
            } else if raw {
                (
                    crate::diff::numbered_lines(text)
                        .iter()
                        .map(|line| {
                            styled_numbered_unified(line.old_number, line.new_number, line.text)
                        })
                        .collect(),
                    "RAW PATCH",
                )
            } else {
                let rows = cached_ide_diff_rows(diff.0, message, worktree_entry);
                (
                    aligned_ide_unified_lines(&rows, content_width),
                    "UNIFIED IDE DIFF",
                )
            };
            frame.render_widget(
                Paragraph::new(Text::from(lines))
                    .scroll((if prepared.is_some() { 0 } else { scroll }, 0))
                    .wrap(Wrap { trim: false })
                    .block(page_block(title, border)),
                sections[0],
            );
        }
    } else {
        let lines: Vec<Line> = crate::diff::numbered_lines(message)
            .iter()
            .map(|line| styled_numbered_unified(line.old_number, line.new_number, line.text))
            .collect();
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .scroll((if prepared.is_some() { 0 } else { scroll }, 0))
                .wrap(Wrap { trim: false })
                .block(page_block(
                    if raw && selectable_hunks {
                        format!(
                            "RAW PATCH · HUNK {}/{}",
                            selected_hunk.saturating_add(1),
                            hunk_count
                        )
                    } else if raw {
                        "RAW PATCH".to_owned()
                    } else {
                        "DIFF".to_owned()
                    },
                    border,
                )),
            sections[0],
        );
    }
    render_key_hints(frame, sections[1], &hints);
}

pub(super) fn diff_line_style(line: &str) -> Style {
    if line.starts_with('+') && !line.starts_with("+++") {
        Style::default()
            .fg(Color::Rgb(134, 239, 172))
            .bg(Color::Rgb(24, 58, 43))
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default()
            .fg(Color::Rgb(252, 165, 165))
            .bg(Color::Rgb(63, 39, 44))
    } else {
        Style::default()
    }
}
