use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::{borrow::Cow, ops::Range};
use unicode_width::UnicodeWidthChar;

use super::diff::diff_line_style;
use super::diff_rows::{IdeDiffLine, IdeDiffRow, source_line};
use super::theme::MUTED;

pub(super) fn aligned_ide_diff_lines(
    rows: &[IdeDiffRow],
    content_width: usize,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();

    for row in rows {
        let old_wrapped = wrap_styled_line(&row.before, content_width);
        let new_wrapped = wrap_styled_line(&row.after, content_width);
        let height = old_wrapped.len().max(new_wrapped.len());

        for line_index in 0..height {
            old_lines.push(ide_line(
                &row.before,
                (line_index == 0).then_some(row.before.number).flatten(),
                old_wrapped.get(line_index),
            ));
            new_lines.push(ide_line(
                &row.after,
                (line_index == 0).then_some(row.after.number).flatten(),
                new_wrapped.get(line_index),
            ));
        }
    }

    (old_lines, new_lines)
}

pub(super) fn aligned_raw_diff_lines(
    rows: &[crate::diff::NumberedDiffRow<'_>],
    content_width: usize,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let mut before = Vec::new();
    let mut after = Vec::new();

    for row in rows
        .iter()
        .filter(|row| row.old_number.is_some() || row.new_number.is_some())
    {
        let old_wrapped = wrap_diff_line(row.old_text, content_width);
        let new_wrapped = wrap_diff_line(row.new_text, content_width);
        let height = old_wrapped.len().max(new_wrapped.len());
        let old_style = diff_line_style(row.old_text);
        let new_style = diff_line_style(row.new_text);

        for line_index in 0..height {
            before.push(numbered_wrapped_line(
                (line_index == 0).then_some(row.old_number).flatten(),
                old_wrapped.get(line_index).map_or("", String::as_str),
                old_style,
            ));
            after.push(numbered_wrapped_line(
                (line_index == 0).then_some(row.new_number).flatten(),
                new_wrapped.get(line_index).map_or("", String::as_str),
                new_style,
            ));
        }
    }

    (before, after)
}

pub(super) fn numbered_wrapped_line(
    number: Option<u32>,
    text: &str,
    style: Style,
) -> Line<'static> {
    if number.is_none() && text.is_empty() {
        return Line::default();
    }
    Line::from(vec![
        Span::styled(format_number(number), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(text.to_owned(), style),
    ])
}

pub(super) fn aligned_ide_unified_lines(
    rows: &[IdeDiffRow],
    content_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for row in rows {
        if row.before.number.is_some()
            && row.after.number.is_some()
            && row.before.background.is_none()
            && row.after.background.is_none()
        {
            lines.extend(ide_unified_code_lines(
                &row.before,
                row.before.number,
                row.after.number,
                " ",
                content_width,
            ));
        } else {
            if row.before.number.is_some() {
                lines.extend(ide_unified_code_lines(
                    &row.before,
                    row.before.number,
                    None,
                    "-",
                    content_width,
                ));
            }
            if row.after.number.is_some() {
                lines.extend(ide_unified_code_lines(
                    &row.after,
                    None,
                    row.after.number,
                    "+",
                    content_width,
                ));
            }
        }
    }
    lines
}

pub(super) fn ide_unified_code_lines(
    source: &IdeDiffLine,
    old_number: Option<u32>,
    new_number: Option<u32>,
    marker: &'static str,
    content_width: usize,
) -> Vec<Line<'static>> {
    let wrapped = wrap_styled_line(source, content_width);
    wrapped
        .iter()
        .enumerate()
        .map(|(index, spans)| {
            let old_number = (index == 0).then_some(old_number).flatten();
            let new_number = (index == 0).then_some(new_number).flatten();
            let marker_style = match marker {
                "+" => Style::default().fg(Color::Green),
                "-" => Style::default().fg(Color::Red),
                _ => Style::default(),
            };
            let background = source.background;
            let with_background = |style: Style| match background {
                Some(color) if style.bg.is_none() => style.bg(color),
                None => style,
                Some(_) => style,
            };
            let mut line = vec![
                Span::styled(
                    format_number(old_number),
                    with_background(Style::default().fg(MUTED)),
                ),
                Span::styled(" ", with_background(Style::default())),
                Span::styled(
                    format_number(new_number),
                    with_background(Style::default().fg(MUTED)),
                ),
                Span::styled(" ", with_background(Style::default())),
                Span::styled(
                    if index == 0 { marker } else { " " },
                    with_background(marker_style),
                ),
                Span::styled(" ", with_background(Style::default())),
            ];
            line.extend(
                spans
                    .iter()
                    .map(|(text, style)| Span::styled(text.clone(), with_background(*style))),
            );
            Line::from(line)
        })
        .collect()
}

pub(super) fn ide_line(
    source: &IdeDiffLine,
    number: Option<u32>,
    wrapped: Option<&Vec<(String, Style)>>,
) -> Line<'static> {
    if number.is_none()
        && source.background.is_none()
        && wrapped.is_none_or(|spans| spans.is_empty())
    {
        return Line::default();
    }
    let background = source.background;
    let with_background = |style: Style| match background {
        Some(color) if style.bg.is_none() => style.bg(color),
        None => style,
        Some(_) => style,
    };
    let mut spans = vec![
        Span::styled(
            format_number(number),
            with_background(Style::default().fg(MUTED)),
        ),
        Span::styled(" ", with_background(Style::default())),
    ];
    if let Some(wrapped) = wrapped {
        spans.extend(
            wrapped
                .iter()
                .map(|(text, style)| Span::styled(text.clone(), with_background(*style))),
        );
    }
    Line::from(spans)
}

pub(super) fn wrap_styled_line(line: &IdeDiffLine, width: usize) -> Vec<Vec<(String, Style)>> {
    let ranges = wrap_line_ranges(&line.code, width);
    let mut lines = Vec::with_capacity(ranges.len());
    let mut character_index = 0;
    let mut byte_cursor = 0;
    let mut span_index = 0;
    let mut span_offset = 0;

    for range in ranges {
        let mut wrapped: Vec<(String, Style)> = Vec::new();
        while byte_cursor < range.end {
            let Some((text, base_style)) = line.spans.get(span_index) else {
                break;
            };
            let available = text.len().saturating_sub(span_offset);
            let take = available.min(range.end - byte_cursor);
            if take == 0 {
                span_index += 1;
                span_offset = 0;
                continue;
            }
            let segment = &text[span_offset..span_offset + take];
            for character in segment.chars() {
                let style = match line.inline_change {
                    Some((start, end)) if start <= character_index && character_index < end => {
                        base_style.bg(line.inline_background.unwrap_or(Color::Reset))
                    }
                    _ => *base_style,
                };
                if let Some((last_text, last_style)) = wrapped.last_mut()
                    && *last_style == style
                {
                    last_text.push(character);
                } else {
                    wrapped.push((character.to_string(), style));
                }
                character_index += 1;
            }
            span_offset += take;
            byte_cursor += take;
            if span_offset == text.len() {
                span_index += 1;
                span_offset = 0;
            }
        }
        lines.push(wrapped);
    }
    lines
}

pub fn diff_hunk_line_positions(
    diff: &str,
    terminal_width: u16,
    raw: bool,
    side_by_side: bool,
) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut visual_line = 0;

    if side_by_side {
        let content_width = usize::from(terminal_width / 2).saturating_sub(8).max(1);
        for row in crate::diff::numbered_side_by_side(diff) {
            if row.old_text.starts_with("@@ ") || row.new_text.starts_with("@@ ") {
                positions.push(visual_line);
            } else if row.old_number.is_some() || row.new_number.is_some() {
                let before = if raw {
                    row.old_text
                } else {
                    source_line(row.old_text).0
                };
                let after = if raw {
                    row.new_text
                } else {
                    source_line(row.new_text).0
                };
                visual_line += wrap_line_ranges(before, content_width)
                    .len()
                    .max(wrap_line_ranges(after, content_width).len());
            }
        }
    } else if raw {
        let content_width = usize::from(terminal_width).saturating_sub(14).max(1);
        for line in diff.lines() {
            if line.starts_with("@@ ") {
                positions.push(visual_line);
            }
            visual_line += wrap_line_ranges(line, content_width).len();
        }
    } else {
        let content_width = usize::from(terminal_width).saturating_sub(16).max(1);
        for row in crate::diff::numbered_side_by_side(diff) {
            if row.old_text.starts_with("@@ ") || row.new_text.starts_with("@@ ") {
                positions.push(visual_line);
            } else if row.old_number.is_some() || row.new_number.is_some() {
                let before = source_line(row.old_text).0;
                let after = source_line(row.new_text).0;
                if row.old_number.is_some()
                    && row.new_number.is_some()
                    && !row.old_text.starts_with('-')
                    && !row.new_text.starts_with('+')
                {
                    visual_line += wrap_line_ranges(before, content_width).len();
                } else {
                    if row.old_number.is_some() {
                        visual_line += wrap_line_ranges(before, content_width).len();
                    }
                    if row.new_number.is_some() {
                        visual_line += wrap_line_ranges(after, content_width).len();
                    }
                }
            }
        }
    }
    positions
}

pub(super) fn wrap_diff_line(text: &str, width: usize) -> Vec<String> {
    wrap_line_ranges(text, width)
        .into_iter()
        .map(|range| text[range].to_owned())
        .collect()
}

pub(super) fn wrap_line_ranges(text: &str, width: usize) -> Vec<Range<usize>> {
    if text.is_empty() {
        #[allow(clippy::single_range_in_vec_init)]
        return vec![0..0];
    }

    let width = width.max(1);
    let mut ranges = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let mut used_width = 0;
        let mut last_fit = start;
        let mut last_whitespace = None;
        let mut overflow = false;

        for (relative_index, character) in text[start..].char_indices() {
            let index = start + relative_index;
            let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
            if used_width + character_width > width {
                overflow = true;
                break;
            }
            used_width += character_width;
            last_fit = index + character.len_utf8();
            if character.is_whitespace() && index > start {
                last_whitespace = Some(last_fit);
            }
        }

        if !overflow {
            ranges.push(start..text.len());
            break;
        }

        let mut split_at = last_whitespace.unwrap_or(last_fit);
        if split_at <= start {
            split_at = start + text[start..].chars().next().unwrap().len_utf8();
        }
        ranges.push(start..split_at);
        start = split_at;
    }

    ranges
}

pub(super) fn styled_numbered_unified(
    old: Option<u32>,
    new: Option<u32>,
    text: &str,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format_number(old), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(format_number(new), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(text.to_owned(), diff_line_style(text)),
    ])
}

pub(super) fn format_number(number: Option<u32>) -> Cow<'static, str> {
    number.map_or(Cow::Borrowed("     "), |number| {
        Cow::Owned(format!("{number:>5}"))
    })
}
