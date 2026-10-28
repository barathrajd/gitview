use crate::comparison::{self, Comparison};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use std::{path::Path, sync::mpsc};

static NEXT_DIFF_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
#[derive(PartialEq)]
enum Mode {
    Input,
    Summary,
    Diff,
}
pub struct ComparisonPage {
    fields: [String; 3],
    field: usize,
    mode: Mode,
    pending: bool,
    feedback: String,
    result: Option<Comparison>,
    diff: (u64, Option<Result<String, String>>),
    scroll: u16,
    raw: bool,
    side_by_side: bool,
    sender: mpsc::Sender<Result<Comparison, String>>,
    receiver: mpsc::Receiver<Result<Comparison, String>>,
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
            diff: (0, None),
            scroll: 0,
            raw: false,
            side_by_side: true,
            sender,
            receiver,
        }
    }
    pub fn tick(&mut self) {
        while let Ok(result) = self.receiver.try_recv() {
            self.pending = false;
            match result {
                Ok(result) => {
                    self.diff = (
                        NEXT_DIFF_ID.fetch_sub(1, std::sync::atomic::Ordering::Relaxed),
                        Some(Ok(result.patch.clone())),
                    );
                    self.result = Some(result);
                    self.mode = Mode::Summary;
                    self.scroll = 0;
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
        if key == KeyCode::Char('H') && self.mode != Mode::Input {
            return false;
        }
        if self.pending {
            return key != KeyCode::Esc;
        }
        match self.mode {
            Mode::Input => match key {
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
                        let _ = sender.send(comparison::load(&root, &fields[0], &fields[1], path));
                    });
                }
                _ => {}
            },
            Mode::Summary => match key {
                KeyCode::Esc | KeyCode::Char('h') => {
                    self.mode = Mode::Input;
                    self.scroll = 0;
                }
                KeyCode::Enter | KeyCode::Char('l') => {
                    self.mode = Mode::Diff;
                    self.scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                _ => {}
            },
            Mode::Diff => match key {
                KeyCode::Esc | KeyCode::Char('h') => {
                    self.mode = Mode::Summary;
                    self.scroll = 0;
                }
                KeyCode::Char('v') => {
                    self.raw = !self.raw;
                    self.scroll = 0;
                }
                KeyCode::Tab => {
                    self.side_by_side = !self.side_by_side;
                    self.scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                _ => {}
            },
        }
        true
    }
    pub fn render(&self, frame: &mut Frame<'_>, border: ratatui::symbols::border::Set<'static>) {
        if self.mode == Mode::Diff {
            crate::ui::render(
                frame,
                crate::ui::Screen::Diff {
                    diff: &self.diff,
                    scroll: self.scroll,
                    selected_hunk: 0,
                    raw: self.raw,
                    side_by_side: self.side_by_side,
                    worktree_entry: None,
                },
                border,
            );
            return;
        }
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
                ("Enter", "open diff"),
                ("j/k", "scroll"),
                ("Esc", "edit comparison"),
                ("H", "history"),
            ]
        };
        let areas = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(hint_row_count(hints, frame.area().width)),
        ])
        .split(frame.area());
        let (title, body) = if self.mode == Mode::Input {
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
            ("COMPARE COMMIT SNAPSHOTS", Text::from(lines))
        } else if let Some(result) = &self.result {
            let mut lines = Vec::new();
            for (label, value) in [
                ("Base", result.base.clone()),
                ("Target", result.target.clone()),
                (
                    "Path",
                    result
                        .path
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "all paths".into()),
                ),
            ] {
                lines.push(Line::from(vec![
                    Span::styled(format!("{label}: "), Style::default().fg(MUTED)),
                    Span::styled(value, Style::default().fg(ACCENT)),
                ]));
            }
            lines.push(Line::raw(""));
            lines.extend(
                (if result.summary.is_empty() {
                    "No differences."
                } else {
                    &result.summary
                })
                .lines()
                .map(|line| Line::raw(line.to_owned())),
            );
            ("COMPARISON SUMMARY", Text::from(lines))
        } else {
            ("COMPARISON", Text::from(self.feedback.clone()))
        };
        frame.render_widget(
            Paragraph::new(body)
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .block(page_block(title, border)),
            areas[0],
        );
        render_key_hints(frame, areas[1], hints);
    }
}
