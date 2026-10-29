use crate::worktree::Entry;
use ratatui::style::{Color, Style};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::OnceLock,
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::{SyntaxReference, SyntaxSet},
};

#[derive(Clone)]
pub(super) struct IdeDiffLine {
    pub(super) number: Option<u32>,
    pub(super) code: String,
    pub(super) spans: Vec<(String, Style)>,
    pub(super) background: Option<Color>,
    pub(super) inline_change: Option<(usize, usize)>,
    pub(super) inline_background: Option<Color>,
}

#[derive(Clone)]
pub(super) struct IdeDiffRow {
    pub(super) before: IdeDiffLine,
    pub(super) after: IdeDiffLine,
}

pub(super) type IdeDiffCacheEntry = (u64, usize, String, Rc<Vec<IdeDiffRow>>);

thread_local! {
    static IDE_DIFF_CACHE: RefCell<Option<IdeDiffCacheEntry>> = const { RefCell::new(None) };
}

pub(super) fn syntax_assets() -> (&'static SyntaxSet, &'static ThemeSet) {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    (
        SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines),
        THEMES.get_or_init(ThemeSet::load_defaults),
    )
}

pub(super) fn cached_ide_diff_rows(
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

pub(super) fn diff_path(message: &str, worktree_entry: Option<&Entry>) -> PathBuf {
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

pub(super) fn diff_syntax<'a>(path: &Path, syntaxes: &'a SyntaxSet) -> &'a SyntaxReference {
    let extension = if path.file_name().is_some_and(|name| name == "Cargo.lock") {
        Some("toml")
    } else {
        path.extension().and_then(|extension| extension.to_str())
    };
    extension
        .and_then(|extension| syntaxes.find_syntax_by_extension(extension))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text())
}

pub(super) fn build_ide_diff_rows(message: &str, path: &Path) -> Vec<IdeDiffRow> {
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

pub(super) fn highlighted_ide_line(
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

pub(super) fn inline_diff_background(background: Color) -> Color {
    match background {
        Color::Rgb(63, 39, 44) => Color::Rgb(112, 54, 64),
        Color::Rgb(24, 58, 43) => Color::Rgb(39, 112, 74),
        _ => background,
    }
}

pub(super) type InlineRange = Option<(usize, usize)>;

pub(super) fn inline_change_ranges(before: &str, after: &str) -> (InlineRange, InlineRange) {
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

pub(super) fn source_line(diff_text: &str) -> (&str, Option<Color>) {
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
