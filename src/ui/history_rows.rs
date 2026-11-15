//! Structured History rows: coloured graph lanes, then aligned columns for
//! the short hash, ref badges, subject, author, and compact age. On narrow
//! panels the age drops first, then the author; long ref badges shorten.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::history::{Entry, MatchFinder};

use super::{ACCENT, MUTED};

/// Lane colours, cycled by a lane's colour index.
const LANES: [Color; 6] = [
    Color::Rgb(94, 234, 212),
    Color::Rgb(251, 191, 36),
    Color::Rgb(147, 197, 253),
    Color::Rgb(244, 114, 182),
    Color::Rgb(163, 230, 53),
    Color::Rgb(251, 146, 60),
];
/// The widest the author column gets.
pub(crate) const AUTHOR_WIDTH: usize = 14;
const AGE_WIDTH: usize = 4;
/// The widest a ref badge's name gets before it shortens.
const BADGE_WIDTH: usize = 24;
/// Columns ref badges leave for the subject.
const SUBJECT_RESERVE: usize = 24;
/// Below these widths the age, then the author, drop out.
const AGE_MIN_WIDTH: usize = 72;
const AUTHOR_MIN_WIDTH: usize = 56;

/// How rows lay out in one frame.
pub(crate) struct RowLayout {
    /// Columns the graph takes, the same on every row.
    pub graph_width: usize,
    /// Columns available for the row inside the panel.
    pub width: usize,
    /// The author column: the longest author, at most [`AUTHOR_WIDTH`].
    pub author_width: usize,
    pub ascii: bool,
    /// The current time, for ages.
    pub now: i64,
}

/// One History row. `row` is the entry's search text; matched characters of
/// `finder`'s query are underlined in the field they fall in.
pub(crate) fn history_line(
    row: &str,
    entry: &Entry,
    layout: &RowLayout,
    finder: &mut MatchFinder,
) -> Line<'static> {
    let positions = finder.positions(row);
    let field = |range: &std::ops::Range<usize>| -> (&str, Vec<usize>) {
        let text = &row[range.clone()];
        let local = positions
            .iter()
            .filter(|&&p| range.contains(&p))
            .map(|&p| p - range.start)
            .collect();
        (text, local)
    };
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut used = 0;

    for cell in &entry.graph.cells {
        let glyph = if layout.ascii {
            cell.glyph.ascii()
        } else {
            cell.glyph.unicode()
        };
        let color = LANES[cell.color % LANES.len()];
        let style = if cell.glyph == crate::graph::Glyph::Node {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        spans.push(Span::styled(glyph.to_string(), style));
    }
    let graph = entry.graph.cells.len();
    spans.push(Span::raw(
        " ".repeat(layout.graph_width.saturating_sub(graph) + 1),
    ));
    used += layout.graph_width.max(graph) + 1;

    let (hash, hash_matches) = field(&entry.hash);
    push_field(
        &mut spans,
        hash,
        &hash_matches,
        Style::default().fg(Color::Rgb(250, 204, 21)),
        usize::MAX,
    );
    spans.push(Span::raw(" "));
    used += hash.width() + 1;

    let show_age = layout.width >= AGE_MIN_WIDTH;
    let show_author = layout.width >= AUTHOR_MIN_WIDTH;
    let author_width = layout.author_width.min(AUTHOR_WIDTH);
    let right =
        usize::from(show_author) * (author_width + 1) + usize::from(show_age) * (AGE_WIDTH + 1);
    // The subject and badges share what is left; the subject keeps at least
    // a few columns.
    let mut room = layout.width.saturating_sub(used + right);

    // Badges leave the subject at least SUBJECT_RESERVE columns (or its
    // whole width, if shorter); badges that don't fit fold into `+N`.
    let subject_width = row[entry.subject.clone()].width();
    let reserve = subject_width.min(SUBJECT_RESERVE);
    for (index, range) in entry.refs.iter().enumerate() {
        let (name, matches) = field(range);
        let left = entry.refs.len() - index - 1;
        // Room for this badge, keeping space for a `+N` after it.
        let fold = if left > 0 {
            format!("+{left} ").width()
        } else {
            0
        };
        let available = room.saturating_sub(reserve + fold + 3);
        let budget = BADGE_WIDTH.min(available).min(name.width());
        if budget < name.width().min(4) || budget == 0 {
            let more = format!("+{} ", entry.refs.len() - index);
            if room >= reserve + more.width() {
                room -= more.width();
                spans.push(Span::styled(more, Style::default().fg(MUTED)));
            }
            break;
        }
        spans.push(Span::styled("(", Style::default().fg(MUTED)));
        let shown = push_field(&mut spans, name, &matches, badge_style(name), budget);
        spans.push(Span::styled(") ", Style::default().fg(MUTED)));
        room = room.saturating_sub(shown + 3);
    }

    let (subject, subject_matches) = field(&entry.subject);
    let shown = push_field(
        &mut spans,
        subject,
        &subject_matches,
        Style::default(),
        room,
    );
    let mut pad = room.saturating_sub(shown);

    if show_author {
        let (author, author_matches) = field(&entry.author);
        spans.push(Span::raw(" ".repeat(pad + 1)));
        let shown = push_field(
            &mut spans,
            author,
            &author_matches,
            Style::default().fg(MUTED),
            author_width,
        );
        pad = author_width.saturating_sub(shown);
    }
    if show_age {
        let age = crate::history::compact_age(entry.commit.time, layout.now);
        spans.push(Span::raw(
            " ".repeat(pad + 1 + AGE_WIDTH.saturating_sub(age.width())),
        ));
        spans.push(Span::styled(age, Style::default().fg(MUTED)));
    }
    Line::from(spans)
}

/// Push `text` cut to `max` columns (ending in `…` when cut), underlining
/// the bytes at `matches`. Returns the columns used.
fn push_field(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    matches: &[usize],
    style: Style,
    max: usize,
) -> usize {
    if max == 0 {
        return 0;
    }
    let (cut, ellipsis) = if text.width() > max {
        let mut width = 0;
        let end = text
            .char_indices()
            .find(|&(_, c)| {
                width += c.width().unwrap_or(0);
                width > max - 1
            })
            .map_or(text.len(), |(index, _)| index);
        (&text[..end], true)
    } else {
        (text, false)
    };
    let matched = style.fg(ACCENT).add_modifier(Modifier::UNDERLINED);
    let mut run = String::new();
    let mut run_matched = false;
    for (index, character) in cut.char_indices() {
        let is_match = matches.binary_search(&index).is_ok();
        if is_match != run_matched && !run.is_empty() {
            let text = std::mem::take(&mut run);
            spans.push(Span::styled(
                text,
                if run_matched { matched } else { style },
            ));
        }
        run_matched = is_match;
        run.push(character);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, if run_matched { matched } else { style }));
    }
    if ellipsis {
        spans.push(Span::styled("…", style));
    }
    cut.width() + usize::from(ellipsis)
}

/// HEAD, tags, remote branches, and local branches each get a colour.
fn badge_style(name: &str) -> Style {
    let color = if name.starts_with("HEAD") {
        Color::Rgb(94, 234, 212)
    } else if name.starts_with("tag: ") {
        Color::Rgb(250, 204, 21)
    } else if name.contains('/') {
        Color::Rgb(147, 197, 253)
    } else {
        Color::Rgb(163, 230, 53)
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit_log::Commit;

    fn entry(refs: &[&str], subject: &str, author: &str, time: i64) -> (String, Entry) {
        let commit = Commit {
            hash: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            parents: Vec::new(),
            refs: refs.iter().map(|r| (*r).to_owned()).collect(),
            author: author.to_owned(),
            time,
            subject: subject.to_owned(),
        };
        let structured = crate::history::structured_from(vec![commit]);
        (
            structured.rows[0].clone(),
            structured.entries[0].clone().unwrap(),
        )
    }

    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn layout(width: usize) -> RowLayout {
        RowLayout {
            graph_width: 3,
            width,
            author_width: AUTHOR_WIDTH,
            ascii: false,
            now: 10_000,
        }
    }

    #[test]
    fn a_wide_row_aligns_every_column() {
        let (row, entry) = entry(&["HEAD -> main"], "Fix parser", "Alice", 10_000 - 7_200);
        let line = history_line(&row, &entry, &layout(80), &mut MatchFinder::new(""));
        let text = text(&line);
        assert!(
            text.starts_with("●   0123456 (HEAD -> main) Fix parser"),
            "{text:?}"
        );
        assert!(text.ends_with("Alice            2h"), "{text:?}");
        assert_eq!(text.width(), 80, "{text:?}");
    }

    #[test]
    fn narrow_rows_drop_the_age_then_the_author() {
        let (row, entry) = entry(&[], "Fix parser", "Alice", 0);
        let medium = text(&history_line(
            &row,
            &entry,
            &layout(60),
            &mut MatchFinder::new(""),
        ));
        assert!(medium.contains("Alice"), "{medium:?}");
        assert!(!medium.contains("2h"), "{medium:?}");
        let narrow = text(&history_line(
            &row,
            &entry,
            &layout(40),
            &mut MatchFinder::new(""),
        ));
        assert!(!narrow.contains("Alice"), "{narrow:?}");
        assert!(narrow.contains("Fix parser"), "{narrow:?}");
    }

    #[test]
    fn long_badges_and_subjects_shorten_with_an_ellipsis() {
        let long = "feature/a-very-long-branch-name-that-keeps-going";
        let (row, entry) = entry(&[long], &"abcdefghij".repeat(10), "Alice", 0);
        let text = text(&history_line(
            &row,
            &entry,
            &layout(80),
            &mut MatchFinder::new(""),
        ));
        // The badge shortens so the subject keeps 24 columns.
        assert!(text.contains("(feature/a-very-long-…)"), "{text:?}");
        assert!(text.contains(" abcdefghijabcdefghijabc… "), "{text:?}");
        assert_eq!(text.width(), 80, "{text:?}");
    }

    #[test]
    fn many_badges_fold_into_a_count_and_leave_the_subject_room() {
        let refs = [
            "HEAD -> main",
            "topic-beta",
            "release-alpha",
            "fix-beta",
            "feature-alpha",
        ];
        let (row, entry) = entry(&refs, "Change library", "Test", 0);
        let text = text(&history_line(
            &row,
            &entry,
            &layout(96),
            &mut MatchFinder::new(""),
        ));
        assert!(text.contains("Change library"), "{text:?}");
        assert!(text.contains("(HEAD -> main)"), "{text:?}");
        assert!(text.contains(" +"), "{text:?}");
        assert_eq!(text.width(), 96, "{text:?}");
    }

    #[test]
    fn matches_are_underlined_in_their_field() {
        let (row, entry) = entry(&[], "Fix parser", "Alice", 0);
        let line = history_line(&row, &entry, &layout(80), &mut MatchFinder::new("parser"));
        let underlined: String = line
            .spans
            .iter()
            .filter(|span| span.style.add_modifier.contains(Modifier::UNDERLINED))
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(underlined, "parser");
    }

    #[test]
    fn ascii_rows_use_ascii_lanes() {
        let (row, entry) = entry(&[], "Fix", "A", 0);
        let mut ascii = layout(80);
        ascii.ascii = true;
        let text = text(&history_line(
            &row,
            &entry,
            &ascii,
            &mut MatchFinder::new(""),
        ));
        assert!(text.starts_with("*   0123456"), "{text:?}");
        assert!(text.is_ascii(), "{text:?}");
    }
}

#[cfg(test)]
mod author_width_tests {
    use super::*;

    #[test]
    fn a_short_author_column_leaves_room_for_the_subject() {
        let commit = crate::commit_log::Commit {
            hash: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            parents: Vec::new(),
            refs: Vec::new(),
            author: "Test".to_owned(),
            time: 0,
            subject: "A subject of exactly forty columns wide.".to_owned(),
        };
        let structured = crate::history::structured_from(vec![commit]);
        let entry = structured.entries[0].clone().unwrap();
        let layout = RowLayout {
            graph_width: 1,
            width: 60,
            author_width: 4,
            ascii: false,
            now: 0,
        };
        let line = history_line(
            &structured.rows[0],
            &entry,
            &layout,
            &mut MatchFinder::new(""),
        );
        let text: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        // The whole subject fits, and the author column is only four wide.
        assert!(text.contains("forty columns wide."), "{text:?}");
        assert!(text.ends_with(" Test"), "{text:?}");
        assert_eq!(text.width(), 60, "{text:?}");
    }
}
