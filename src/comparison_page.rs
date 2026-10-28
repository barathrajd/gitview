use crate::comparison::{self, Comparison};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use std::{
    cell::{Cell, RefCell},
    path::Path,
    sync::mpsc,
};
#[derive(PartialEq)]
enum Mode {
    Input,
    Review,
}
type PreparedResult = Result<(Comparison, Vec<crate::ui::PreparedComparisonFile>), String>;
type FileLines = (Vec<Line<'static>>, Vec<Line<'static>>);
struct RenderCache {
    width: u16,
    raw: bool,
    split: bool,
    files: Vec<FileLines>,
}
pub struct ComparisonPage {
    fields: [String; 3],
    field: usize,
    mode: Mode,
    pending: bool,
    feedback: String,
    result: Option<Comparison>,
    prepared: Vec<crate::ui::PreparedComparisonFile>,
    collapsed: Vec<bool>,
    selected: usize,
    scroll: Cell<usize>,
    jump: Cell<bool>,
    raw: bool,
    side_by_side: bool,
    cache: RefCell<Vec<RenderCache>>,
    sender: mpsc::Sender<PreparedResult>,
    receiver: mpsc::Receiver<PreparedResult>,
}
impl ComparisonPage {
    pub fn open(target: &str, path: Option<&Path>) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            fields: [
                format!("{target}^"),
                target.into(),
                path.map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ],
            field: 0,
            mode: Mode::Input,
            pending: false,
            feedback: String::new(),
            result: None,
            prepared: vec![],
            collapsed: vec![],
            selected: 0,
            scroll: Cell::new(0),
            jump: Cell::new(false),
            raw: false,
            side_by_side: true,
            cache: RefCell::new(Vec::new()),
            sender,
            receiver,
        }
    }
    pub fn tick(&mut self) {
        while let Ok(result) = self.receiver.try_recv() {
            self.pending = false;
            match result {
                Ok((result, prepared)) => {
                    self.prepared = prepared;
                    self.collapsed = vec![false; result.files.len()];
                    self.selected = 0;
                    self.result = Some(result);
                    self.mode = Mode::Review;
                    self.scroll.set(0);
                    self.cache.replace(Vec::new());
                    self.feedback.clear();
                }
                Err(error) => self.feedback = error,
            }
        }
    }
    pub fn paste(&mut self, text: &str) {
        if self.mode == Mode::Input && !self.pending {
            self.fields[self.field].push_str(text);
        }
    }
    pub fn key(&mut self, root: &Path, key: KeyCode) -> bool {
        if self.pending {
            return key != KeyCode::Esc;
        }
        if self.mode == Mode::Input {
            match key {
                KeyCode::Esc => return false,
                KeyCode::Tab => self.field = (self.field + 1) % 3,
                KeyCode::BackTab => self.field = (self.field + 2) % 3,
                KeyCode::Backspace => {
                    self.fields[self.field].pop();
                }
                KeyCode::Char(ch) if !ch.is_control() => self.fields[self.field].push(ch),
                KeyCode::Enter => {
                    let fields = self.fields.clone();
                    let root = root.to_path_buf();
                    let sender = self.sender.clone();
                    self.pending = true;
                    self.feedback.clear();
                    std::thread::spawn(move || {
                        let path = (!fields[2].is_empty()).then(|| Path::new(&fields[2]));
                        let result =
                            comparison::load(&root, &fields[0], &fields[1], path).map(|result| {
                                let prepared = result
                                    .files
                                    .iter()
                                    .map(|file| {
                                        crate::ui::prepare_comparison_file(
                                            &file.patch,
                                            Path::new(&file.path),
                                        )
                                    })
                                    .collect();
                                (result, prepared)
                            });
                        let _ = sender.send(result);
                    });
                }
                _ => {}
            }
        } else {
            match key {
                KeyCode::Char('H') => return false,
                KeyCode::Esc | KeyCode::Char('h') => self.mode = Mode::Input,
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll.set(self.scroll.get().saturating_add(1))
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll.set(self.scroll.get().saturating_sub(1))
                }
                KeyCode::Char(']') => {
                    self.selected = (self.selected + 1).min(self.collapsed.len().saturating_sub(1));
                    self.jump.set(true);
                }
                KeyCode::Char('[') => {
                    self.selected = self.selected.saturating_sub(1);
                    self.jump.set(true);
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(collapsed) = self.collapsed.get_mut(self.selected) {
                        *collapsed = !*collapsed;
                        self.jump.set(true);
                    }
                }
                KeyCode::Char('C' | 'E') => {
                    self.collapsed.fill(key == KeyCode::Char('C'));
                    self.scroll.set(0);
                    self.selected = 0;
                }
                KeyCode::Char('v') => {
                    self.raw = !self.raw;
                    self.jump.set(true);
                }
                KeyCode::Tab => {
                    self.side_by_side = !self.side_by_side;
                    self.jump.set(true);
                }
                _ => {}
            }
        }
        true
    }
    pub fn render(&self, frame: &mut Frame<'_>, border: ratatui::symbols::border::Set<'static>) {
        use crate::ui::{
            ACCENT, MUTED, hint_row_count, page_block, render_key_hints, selection_style,
        };
        let hints: &[(&str, &str)] = if self.pending {
            &[("Working", "loading comparison"), ("Esc", "return")]
        } else if self.mode == Mode::Input {
            &[
                ("Tab/Shift+Tab", "field"),
                ("Enter", "compare"),
                ("Esc", "return"),
            ]
        } else {
            &[
                ("[/]", "file"),
                ("Enter/Space", "fold"),
                ("C/E", "fold/unfold all"),
                ("j/k", "scroll"),
                ("v", "IDE/raw"),
                ("Tab", "split/unified"),
                ("Esc", "edit"),
                ("H", "history"),
            ]
        };
        let areas = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(hint_row_count(hints, frame.area().width)),
        ])
        .split(frame.area());
        render_key_hints(frame, areas[1], hints);
        if self.mode == Mode::Review {
            if let Some(result) = &self.result {
                self.render_review(frame, areas[0], result, border);
            }
            return;
        }
        let mut lines = self
            .fields
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let active = index == self.field;
                let style = if active {
                    selection_style()
                } else {
                    Style::default()
                };
                Line::from(vec![
                    Span::styled(
                        format!(
                            "{} {}: ",
                            if active { "›" } else { " " },
                            [
                                "Base revision",
                                "Target revision",
                                "Literal path (optional)"
                            ][index]
                        ),
                        style.fg(if active { ACCENT } else { MUTED }),
                    ),
                    Span::styled(format!("{value}{}", if active { "_" } else { "" }), style),
                ])
                .style(style)
            })
            .collect::<Vec<_>>();
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Target minus base. Revisions must resolve locally to commits.",
            Style::default().fg(MUTED),
        ));
        lines.push(Line::styled(
            "A path restricts both summary and patch. No fetch is performed.",
            Style::default().fg(MUTED),
        ));
        lines.push(Line::raw(""));
        lines.extend(
            (if self.pending {
                "Loading comparison..."
            } else {
                &self.feedback
            })
            .lines()
            .map(|line| {
                Line::styled(
                    line.to_owned(),
                    Style::default().fg(Color::Rgb(251, 191, 36)),
                )
            }),
        );
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: false })
                .block(page_block("COMPARE COMMIT SNAPSHOTS", border)),
            areas[0],
        );
    }
    fn render_review(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        result: &Comparison,
        border: ratatui::symbols::border::Set<'static>,
    ) {
        use crate::ui::{ACCENT, MUTED, comparison_file_lines, page_block, selection_style};
        let areas = Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).split(area);
        let context = vec![
            Line::from(vec![
                Span::styled("Base: ", Style::default().fg(MUTED)),
                Span::styled(result.base.clone(), Style::default().fg(ACCENT)),
            ]),
            Line::from(vec![
                Span::styled("Target: ", Style::default().fg(MUTED)),
                Span::styled(result.target.clone(), Style::default().fg(ACCENT)),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(MUTED)),
                Span::styled(
                    format!(
                        "{} · {} files{}",
                        result
                            .path
                            .as_ref()
                            .map(|path| path.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "all paths".into()),
                        result.files.len(),
                        if result.truncated {
                            " · OUTPUT TRUNCATED"
                        } else {
                            ""
                        }
                    ),
                    Style::default().fg(ACCENT),
                ),
            ]),
        ];
        let title = format!(
            "COMPARISON REVIEW · {} · {}",
            if self.side_by_side {
                "BEFORE / AFTER"
            } else {
                "UNIFIED"
            },
            if self.raw { "RAW" } else { "IDE" }
        );
        frame.render_widget(
            Paragraph::new(context).block(page_block(title, border)),
            areas[0],
        );
        let block = page_block("FILES", border);
        let inner = block.inner(areas[1]);
        frame.render_widget(block, areas[1]);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        if result.files.is_empty() {
            frame.render_widget(
                Paragraph::new(if result.truncated {
                    "No file records available within output limit."
                } else {
                    "No differences."
                })
                .style(Style::default().fg(MUTED)),
                inner,
            );
            return;
        }
        let mut cache = self.cache.borrow_mut();
        // Retain the four style/layout combinations at the current width.
        if cache
            .first()
            .is_some_and(|cache| cache.width != inner.width)
        {
            cache.clear();
        }
        let cache_index = if let Some(index) = cache
            .iter()
            .position(|cache| cache.raw == self.raw && cache.split == self.side_by_side)
        {
            index
        } else {
            let width = if self.side_by_side {
                usize::from(inner.width.saturating_sub(1) / 2).saturating_sub(6)
            } else {
                usize::from(inner.width).saturating_sub(12)
            }
            .max(1);
            let files = result
                .files
                .iter()
                .zip(&self.prepared)
                .map(|(file, prepared)| {
                    comparison_file_lines(&file.patch, prepared, width, self.raw, self.side_by_side)
                })
                .collect();
            cache.push(RenderCache {
                width: inner.width,
                raw: self.raw,
                split: self.side_by_side,
                files,
            });
            cache.len() - 1
        };
        let cache = &cache[cache_index];
        let height = |index: usize| {
            if self.collapsed[index] {
                0
            } else {
                cache.files[index].0.len().max(cache.files[index].1.len())
            }
        };
        let total = (0..result.files.len())
            .map(|index| 1 + height(index))
            .sum::<usize>();
        if self.jump.replace(false) {
            self.scroll
                .set((0..self.selected).map(|index| 1 + height(index)).sum());
        }
        // Allow the final file header to remain at the top when jumping to it.
        self.scroll
            .set(self.scroll.get().min(total.saturating_sub(1)));
        let rows = (0..result.files.len()).flat_map(|index| {
            std::iter::once((index, None))
                .chain((0..height(index)).map(move |line| (index, Some(line))))
        });
        for (visible, (index, line)) in rows
            .skip(self.scroll.get())
            .take(usize::from(inner.height))
            .enumerate()
        {
            let row = Rect::new(inner.x, inner.y + visible as u16, inner.width, 1);
            if let Some(line) = line {
                let (left, right) = &cache.files[index];
                if self.side_by_side {
                    let width = row.width.saturating_sub(1) / 2;
                    if let Some(text) = left.get(line) {
                        frame.render_widget(
                            Paragraph::new(text.clone()),
                            Rect::new(row.x, row.y, width, 1),
                        );
                    }
                    frame.render_widget(
                        Paragraph::new(border.vertical_left).style(Style::default().fg(MUTED)),
                        Rect::new(row.x + width, row.y, 1, 1),
                    );
                    if let Some(text) = right.get(line) {
                        frame.render_widget(
                            Paragraph::new(text.clone()),
                            Rect::new(
                                row.x + width + 1,
                                row.y,
                                row.width.saturating_sub(width + 1),
                                1,
                            ),
                        );
                    }
                } else if let Some(text) = left.get(line) {
                    frame.render_widget(Paragraph::new(text.clone()), row);
                }
            } else {
                let file = &result.files[index];
                let style = if index == self.selected {
                    selection_style()
                } else {
                    Style::default().bg(Color::Rgb(30, 41, 59))
                };
                frame.render_widget(
                    Paragraph::new(format!(
                        "{} {}",
                        if self.collapsed[index] { "▸" } else { "▾" },
                        file.path.to_string_lossy()
                    ))
                    .style(style.fg(Color::Rgb(251, 191, 36))),
                    row,
                );
                let counts = match (file.additions, file.deletions) {
                    (Some(added), Some(deleted)) => vec![
                        Span::styled(format!(" +{added}"), style.fg(Color::Rgb(74, 222, 128))),
                        Span::styled(format!(" -{deleted} "), style.fg(Color::Rgb(248, 113, 113))),
                    ],
                    _ => vec![Span::styled(" binary ", style.fg(MUTED))],
                };
                let text = Line::from(counts);
                let width = (text.width() as u16).min(row.width);
                frame.render_widget(
                    Paragraph::new(text).style(style),
                    Rect::new(row.right().saturating_sub(width), row.y, width, 1),
                );
            }
        }
    }
}
