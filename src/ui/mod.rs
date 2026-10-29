mod branches;
mod cockpit;
mod commit;
mod diff;
mod help;
mod hints;
mod panels;
mod render;
mod screen;
mod theme;

pub use help::{HELP, render_help_overlay};
pub(crate) use hints::{hint_row_count, render_key_hints};
pub use render::render;
pub use screen::{Cockpit, HelpPage, Screen};
pub use theme::ASCII_BORDER;
pub(crate) use theme::{ACCENT, MUTED, page_block, selection_style};

use crate::worktree::Entry;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::{
    borrow::Cow,
    cell::RefCell,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
    sync::OnceLock,
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::{SyntaxReference, SyntaxSet},
};
use unicode_width::UnicodeWidthChar;

use self::diff::diff_line_style;

#[derive(Clone)]
struct IdeDiffLine {
    number: Option<u32>,
    code: String,
    spans: Vec<(String, Style)>,
    background: Option<Color>,
    inline_change: Option<(usize, usize)>,
    inline_background: Option<Color>,
}

#[derive(Clone)]
struct IdeDiffRow {
    before: IdeDiffLine,
    after: IdeDiffLine,
}

type IdeDiffCacheEntry = (u64, usize, String, Rc<Vec<IdeDiffRow>>);

thread_local! {
    static IDE_DIFF_CACHE: RefCell<Option<IdeDiffCacheEntry>> = const { RefCell::new(None) };
}

fn syntax_assets() -> (&'static SyntaxSet, &'static ThemeSet) {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    (
        SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines),
        THEMES.get_or_init(ThemeSet::load_defaults),
    )
}

fn cached_ide_diff_rows(
    id: u64,
    message: &str,
    worktree_entry: Option<&Entry>,
) -> Rc<Vec<IdeDiffRow>> {
    let path = diff_path(message, worktree_entry);
    let path_key = path.to_string_lossy().into_owned();
    let message_pointer = message.as_ptr() as usize;

    IDE_DIFF_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((cached_id, cached_pointer, cached_path, rows)) = cache.as_ref()
            && *cached_id == id
            && *cached_pointer == message_pointer
            && *cached_path == path_key
        {
            return Rc::clone(rows);
        }

        let rows = Rc::new(build_ide_diff_rows(message, &path));
        *cache = Some((id, message_pointer, path_key, Rc::clone(&rows)));
        rows
    })
}

fn diff_path(message: &str, worktree_entry: Option<&Entry>) -> PathBuf {
    if let Some(entry) = worktree_entry {
        return PathBuf::from(&entry.path);
    }

    message
        .lines()
        .filter_map(|line| {
            line.strip_prefix("+++ ")
                .or_else(|| line.strip_prefix("--- "))
        })
        .find(|path| *path != "/dev/null")
        .map(|path| {
            let path = path.trim_matches('"');
            PathBuf::from(
                path.strip_prefix("b/")
                    .or_else(|| path.strip_prefix("a/"))
                    .unwrap_or(path),
            )
        })
        .unwrap_or_default()
}

fn diff_syntax<'a>(path: &Path, syntaxes: &'a SyntaxSet) -> &'a SyntaxReference {
    let extension = if path.file_name().is_some_and(|name| name == "Cargo.lock") {
        Some("toml")
    } else {
        path.extension().and_then(|extension| extension.to_str())
    };
    extension
        .and_then(|extension| syntaxes.find_syntax_by_extension(extension))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
}

fn build_ide_diff_rows(message: &str, path: &Path) -> Vec<IdeDiffRow> {
    let (syntaxes, themes) = syntax_assets();
    let syntax = diff_syntax(path, syntaxes);
    let theme = themes
        .themes
        .get("base16-ocean.dark")
        .or_else(|| themes.themes.values().next())
        .expect("Syntect default themes are present");
    let mut before_highlighter = HighlightLines::new(syntax, theme);
    let mut after_highlighter = HighlightLines::new(syntax, theme);
    let mut rows = Vec::new();

    for row in crate::diff::numbered_side_by_side(message) {
        if row.old_text.starts_with("@@ ") || row.new_text.starts_with("@@ ") {
            before_highlighter = HighlightLines::new(syntax, theme);
            after_highlighter = HighlightLines::new(syntax, theme);
            continue;
        }
        if row.old_number.is_none() && row.new_number.is_none() {
            continue;
        }

        let mut before = highlighted_ide_line(
            row.old_number,
            row.old_text,
            &mut before_highlighter,
            syntaxes,
        );
        let mut after = highlighted_ide_line(
            row.new_number,
            row.new_text,
            &mut after_highlighter,
            syntaxes,
        );
        if before.background.is_some() && after.background.is_some() {
            let (before_range, after_range) = inline_change_ranges(&before.code, &after.code);
            before.inline_change = before_range;
            after.inline_change = after_range;
            before.inline_background = before.background.map(inline_diff_background);
            after.inline_background = after.background.map(inline_diff_background);
        }
        rows.push(IdeDiffRow { before, after });
    }
    rows
}

fn highlighted_ide_line(
    number: Option<u32>,
    diff_text: &str,
    highlighter: &mut HighlightLines<'_>,
    syntaxes: &SyntaxSet,
) -> IdeDiffLine {
    let (code, background) = source_line(diff_text);
    if number.is_none() {
        return IdeDiffLine {
            number,
            code: String::new(),
            spans: Vec::new(),
            background,
            inline_change: None,
            inline_background: None,
        };
    }

    let input = format!("{code}\n");
    let spans = highlighter
        .highlight_line(&input, syntaxes)
        .map(|ranges| {
            ranges
                .into_iter()
                .filter_map(|(style, text)| {
                    let text = text.trim_end_matches('\n').trim_end_matches('\r');
                    (!text.is_empty()).then(|| {
                        (
                            text.to_owned(),
                            Style::default().fg(Color::Rgb(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            )),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_else(|_| vec![(code.to_owned(), Style::default())]);

    IdeDiffLine {
        number,
        code: code.to_owned(),
        spans,
        background,
        inline_change: None,
        inline_background: None,
    }
}

fn inline_diff_background(background: Color) -> Color {
    match background {
        Color::Rgb(63, 39, 44) => Color::Rgb(112, 54, 64),
        Color::Rgb(24, 58, 43) => Color::Rgb(39, 112, 74),
        _ => background,
    }
}

type InlineRange = Option<(usize, usize)>;

fn inline_change_ranges(before: &str, after: &str) -> (InlineRange, InlineRange) {
    let before: Vec<_> = before.chars().collect();
    let after: Vec<_> = after.chars().collect();
    let mut prefix = 0;
    while prefix < before.len() && prefix < after.len() && before[prefix] == after[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < before.len().saturating_sub(prefix)
        && suffix < after.len().saturating_sub(prefix)
        && before[before.len() - suffix - 1] == after[after.len() - suffix - 1]
    {
        suffix += 1;
    }

    let before_end = before.len() - suffix;
    let after_end = after.len() - suffix;
    (
        (prefix < before_end).then_some((prefix, before_end)),
        (prefix < after_end).then_some((prefix, after_end)),
    )
}

fn source_line(diff_text: &str) -> (&str, Option<Color>) {
    if let Some(text) = diff_text.strip_prefix('+')
        && !diff_text.starts_with("+++")
    {
        (text, Some(Color::Rgb(24, 58, 43)))
    } else if let Some(text) = diff_text.strip_prefix('-')
        && !diff_text.starts_with("---")
    {
        (text, Some(Color::Rgb(63, 39, 44)))
    } else if let Some(text) = diff_text.strip_prefix(' ') {
        (text, None)
    } else {
        (diff_text, None)
    }
}

fn aligned_ide_diff_lines(
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

fn aligned_raw_diff_lines(
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

fn numbered_wrapped_line(number: Option<u32>, text: &str, style: Style) -> Line<'static> {
    if number.is_none() && text.is_empty() {
        return Line::default();
    }
    Line::from(vec![
        Span::styled(format_number(number), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(text.to_owned(), style),
    ])
}

fn aligned_ide_unified_lines(rows: &[IdeDiffRow], content_width: usize) -> Vec<Line<'static>> {
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

fn ide_unified_code_lines(
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

fn ide_line(
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

fn wrap_styled_line(line: &IdeDiffLine, width: usize) -> Vec<Vec<(String, Style)>> {
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

fn wrap_diff_line(text: &str, width: usize) -> Vec<String> {
    wrap_line_ranges(text, width)
        .into_iter()
        .map(|range| text[range].to_owned())
        .collect()
}

fn wrap_line_ranges(text: &str, width: usize) -> Vec<Range<usize>> {
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

fn styled_numbered_unified(old: Option<u32>, new: Option<u32>, text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format_number(old), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(format_number(new), Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(text.to_owned(), diff_line_style(text)),
    ])
}

fn format_number(number: Option<u32>) -> Cow<'static, str> {
    number.map_or(Cow::Borrowed("     "), |number| {
        Cow::Owned(format!("{number:>5}"))
    })
}

pub(crate) struct PreparedComparisonFile {
    rows: Vec<IdeDiffRow>,
}

pub(crate) fn prepare_comparison_file(patch: &str, path: &Path) -> PreparedComparisonFile {
    PreparedComparisonFile {
        rows: build_ide_diff_rows(patch, path),
    }
}

/// Build already-wrapped rows for one file in a stacked comparison.
pub(crate) fn comparison_file_lines(
    patch: &str,
    prepared: &PreparedComparisonFile,
    content_width: usize,
    raw: bool,
    split: bool,
) -> (Vec<Line<'static>>, Vec<Line<'static>>) {
    let (mut left, mut right) = if split {
        if raw {
            aligned_raw_diff_lines(&crate::diff::numbered_side_by_side(patch), content_width)
        } else {
            aligned_ide_diff_lines(&prepared.rows, content_width)
        }
    } else if raw {
        let mut lines = Vec::new();
        for row in crate::diff::numbered_lines(patch) {
            for (index, chunk) in wrap_diff_line(row.text, content_width).iter().enumerate() {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(
                            "{} {} ",
                            format_number(if index == 0 { row.old_number } else { None }),
                            format_number(if index == 0 { row.new_number } else { None })
                        ),
                        Style::default().fg(MUTED),
                    ),
                    Span::styled(chunk.clone(), diff_line_style(row.text)),
                ]));
            }
        }
        (lines, Vec::new())
    } else {
        (
            aligned_ide_unified_lines(&prepared.rows, content_width),
            Vec::new(),
        )
    };
    if left.is_empty() && right.is_empty() {
        left = patch
            .lines()
            .flat_map(|line| wrap_diff_line(line, content_width))
            .map(|line| Line::styled(line, Style::default().fg(MUTED)))
            .collect();
        if left.is_empty() {
            left.push(Line::styled(
                "No textual changes.",
                Style::default().fg(MUTED),
            ));
        }
        if split {
            right = left.clone();
        }
    }
    (left, right)
}

const ORDINARY_LAYOUT_CACHE_BYTES: usize = 8 * 1024 * 1024;

struct OrdinaryLayout {
    width: u16,
    raw: bool,
    split: bool,
    left: Vec<Line<'static>>,
    right: Vec<Line<'static>>,
    positions: Vec<usize>,
    bytes: usize,
}

pub(crate) fn retained_line_bytes(lines: &[Line<'static>], capacity: usize) -> usize {
    capacity * std::mem::size_of::<Line<'static>>()
        + lines
            .iter()
            .map(|line| {
                line.spans.capacity() * std::mem::size_of::<Span<'static>>()
                    + line
                        .spans
                        .iter()
                        .map(|span| match &span.content {
                            Cow::Owned(text) => text.capacity(),
                            Cow::Borrowed(_) => 0,
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
}

/// Immutable diff content prepared by a background loader, with four layouts at one width.
pub struct PreparedDiff {
    text: String,
    prepared: PreparedComparisonFile,
    hunk_count: usize,
    layouts: RefCell<Vec<OrdinaryLayout>>,
}

impl PreparedDiff {
    pub fn new(text: String, path: Option<&Path>) -> Self {
        let inferred = diff_path(&text, None);
        let prepared = prepare_comparison_file(&text, path.unwrap_or(&inferred));
        let hunk_count = text.lines().filter(|line| line.starts_with("@@ ")).count();
        Self {
            text,
            prepared,
            hunk_count,
            layouts: RefCell::new(Vec::new()),
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    fn layout(&self, width: u16, raw: bool, split: bool) -> std::cell::Ref<'_, OrdinaryLayout> {
        let mut layouts = self.layouts.borrow_mut();
        if layouts.first().is_some_and(|layout| layout.width != width) {
            layouts.clear();
        }
        let index = if let Some(index) = layouts
            .iter()
            .position(|layout| layout.raw == raw && layout.split == split)
        {
            index
        } else {
            let content_width = if split {
                usize::from(width / 2).saturating_sub(8)
            } else if raw {
                usize::from(width).saturating_sub(14)
            } else {
                usize::from(width).saturating_sub(16)
            }
            .max(1);
            let (left, right) =
                comparison_file_lines(&self.text, &self.prepared, content_width, raw, split);
            let positions = diff_hunk_line_positions(&self.text, width, raw, split);
            let bytes = retained_line_bytes(&left, left.capacity())
                + retained_line_bytes(&right, right.capacity())
                + positions.capacity() * std::mem::size_of::<usize>();
            // Preserve the active layout even if it alone exceeds the budget.
            // Superseded layouts are expendable; their content can be rebuilt.
            while !layouts.is_empty()
                && layouts.iter().map(|layout| layout.bytes).sum::<usize>() + bytes
                    > ORDINARY_LAYOUT_CACHE_BYTES
            {
                layouts.remove(0);
            }
            layouts.push(OrdinaryLayout {
                width,
                raw,
                split,
                left,
                right,
                positions,
                bytes,
            });
            layouts.len() - 1
        };
        drop(layouts);
        std::cell::Ref::map(self.layouts.borrow(), |layouts| &layouts[index])
    }
    #[cfg(feature = "test-support")]
    pub fn layout_cache_usage(&self) -> (usize, usize, usize) {
        let layouts = self.layouts.borrow();
        (
            layouts.len(),
            layouts.iter().map(|layout| layout.bytes).sum(),
            layouts.last().map_or(0, |layout| layout.bytes),
        )
    }
    pub fn hunk_positions(&self, width: u16, raw: bool, split: bool) -> Vec<usize> {
        self.layout(width, raw, split).positions.clone()
    }
}
