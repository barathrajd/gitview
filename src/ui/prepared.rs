use ratatui::{
    style::Style,
    text::{Line, Span},
};
use std::{borrow::Cow, cell::RefCell, path::Path};

use super::diff::diff_line_style;
use super::diff_layout::{
    aligned_ide_diff_lines, aligned_ide_unified_lines, aligned_raw_diff_lines,
    diff_hunk_line_positions, format_number, wrap_diff_line,
};
use super::diff_rows::{IdeDiffRow, build_ide_diff_rows, diff_path};
use super::theme::MUTED;

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

pub(super) const ORDINARY_LAYOUT_CACHE_BYTES: usize = 8 * 1024 * 1024;

pub(super) struct OrdinaryLayout {
    pub(super) width: u16,
    pub(super) raw: bool,
    pub(super) split: bool,
    pub(super) left: Vec<Line<'static>>,
    pub(super) right: Vec<Line<'static>>,
    pub(super) positions: Vec<usize>,
    pub(super) bytes: usize,
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
    pub(super) hunk_count: usize,
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
    pub(super) fn layout(
        &self,
        width: u16,
        raw: bool,
        split: bool,
    ) -> std::cell::Ref<'_, OrdinaryLayout> {
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
