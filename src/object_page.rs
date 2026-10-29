use crate::ui::Viewport;
use crate::{
    object::{self, ObjectDescription},
    ui::{ACCENT, MUTED, hint_row_count, page_block, render_key_hints},
};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use std::{path::Path, sync::mpsc};
pub struct ObjectPage {
    input: String,
    editing: bool,
    pending: bool,
    error: String,
    result: Option<ObjectDescription>,
    scroll: u16,
    sender: mpsc::Sender<Result<ObjectDescription, String>>,
    receiver: mpsc::Receiver<Result<ObjectDescription, String>>,
}
impl ObjectPage {
    pub fn open(revision: &str) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            input: revision.into(),
            editing: true,
            pending: false,
            error: String::new(),
            result: None,
            scroll: 0,
            sender,
            receiver,
        }
    }
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        while let Ok(result) = self.receiver.try_recv() {
            changed = true;
            self.pending = false;
            match result {
                Ok(result) => {
                    self.result = Some(result);
                    self.editing = false;
                    self.scroll = 0;
                    self.error.clear();
                }
                Err(error) => self.error = error,
            }
        }
        changed
    }
    pub fn paste(&mut self, text: &str) {
        if self.editing && !self.pending {
            self.input.push_str(text);
        }
    }
    pub fn key(&mut self, root: &Path, key: KeyCode) -> bool {
        if self.pending {
            return key != KeyCode::Esc;
        }
        if self.editing {
            match key {
                KeyCode::Esc => return false,
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(ch) if !ch.is_control() => self.input.push(ch),
                KeyCode::Enter => {
                    let root = root.to_path_buf();
                    let revision = self.input.clone();
                    let sender = self.sender.clone();
                    self.pending = true;
                    self.error.clear();
                    std::thread::spawn(move || {
                        let _ = sender.send(object::inspect(&root, &revision));
                    });
                }
                _ => {}
            }
        } else {
            match key {
                KeyCode::Esc | KeyCode::Char('e') => self.editing = true,
                KeyCode::Char('H') => return false,
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                _ => {}
            }
        }
        true
    }
    pub fn render(
        &self,
        frame: &mut Viewport<'_, '_>,
        border: ratatui::symbols::border::Set<'static>,
    ) {
        let hints: &[(&str, &str)] = if self.pending {
            &[("Working", "inspecting object"), ("Esc", "return")]
        } else if self.editing {
            &[("Enter", "inspect"), ("Esc", "history")]
        } else {
            &[
                ("j/k", "scroll"),
                ("e/Esc", "edit revision"),
                ("H", "history"),
            ]
        };
        let areas = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(hint_row_count(hints, frame.area().width)),
        ])
        .split(frame.area());
        let mut lines = Vec::new();
        if self.editing {
            lines.push(Line::from(vec![
                Span::styled("Object revision: ", Style::default().fg(MUTED)),
                Span::styled(format!("{}_", self.input), Style::default().fg(ACCENT)),
            ]));
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "Enter a local commit, tag, tree, or blob revision (for example HEAD:path).",
                Style::default().fg(MUTED),
            ));
            lines.push(Line::raw(""));
            lines.extend(
                (if self.pending {
                    "Inspecting object..."
                } else {
                    &self.error
                })
                .lines()
                .map(|line| {
                    Line::styled(
                        line.to_owned(),
                        Style::default().fg(Color::Rgb(251, 191, 36)),
                    )
                }),
            );
        } else if let Some(result) = &self.result {
            for (label, value) in [
                ("Object", result.id.clone()),
                ("Type", result.kind.clone()),
                ("Size", format!("{} bytes", result.size)),
            ] {
                lines.push(Line::from(vec![
                    Span::styled(format!("{label}: "), Style::default().fg(MUTED)),
                    Span::styled(value, Style::default().fg(ACCENT)),
                ]));
            }
            if let Some(description) = &result.description {
                lines.push(Line::from(vec![
                    Span::styled("Describe: ", Style::default().fg(MUTED)),
                    Span::styled(description.clone(), Style::default().fg(ACCENT)),
                ]));
            }
            if result.truncated {
                lines.push(Line::styled(
                    "[preview truncated at 1 MB or 5000 lines]",
                    Style::default().fg(Color::Rgb(251, 191, 36)),
                ));
            }
            lines.push(Line::raw(""));
            lines.extend(
                result
                    .preview
                    .lines()
                    .map(|line| Line::raw(line.to_owned())),
            );
        }
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: false })
                .scroll((if self.editing { 0 } else { self.scroll }, 0))
                .block(page_block(
                    if self.editing {
                        "INSPECT GIT OBJECT"
                    } else {
                        "OBJECT DESCRIPTION"
                    },
                    border,
                )),
            areas[0],
        );
        render_key_hints(frame, areas[1], hints);
    }
}
