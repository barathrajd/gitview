use crate::tags::{self, CreatePreview, Tag};
use crate::ui::Viewport;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
#[derive(Clone)]
enum Action {
    Create(CreatePreview),
    Delete(Tag),
    Refresh,
}
enum ReadRequest {
    Create(String),
    Verify(Tag),
}
enum ReadResult {
    Create(Result<CreatePreview, String>),
    Verify(Result<tags::Verification, String>, Result<Vec<Tag>, String>),
}
#[derive(Clone, Copy)]
enum ReadKind {
    Create,
    Verify,
}
type Reply = (Result<String, String>, Result<Vec<Tag>, String>);
pub struct TagPage {
    entries: Vec<Tag>,
    selected: usize,
    input: Option<String>,
    confirm: Option<Action>,
    feedback: String,
    pending: bool,
    pending_read: Option<ReadKind>,
    read_worker: crate::read_worker::LatestRead<ReadRequest, ReadResult>,
    verification: Option<String>,
    scroll: u16,
    /// The verification's scroll limit and visible rows, and the list's
    /// visible rows, from the last render.
    view: std::cell::Cell<(usize, usize, usize)>,
    sender: mpsc::Sender<Reply>,
    receiver: mpsc::Receiver<Reply>,
}
impl TagPage {
    pub fn open(root: &Path) -> Self {
        let (sender, receiver) = mpsc::channel();
        let read_root = root.to_path_buf();
        let read_worker = crate::read_worker::LatestRead::new(move |request| match request {
            ReadRequest::Create(name) => {
                ReadResult::Create(tags::preview_create(&read_root, &name))
            }
            ReadRequest::Verify(tag) => {
                ReadResult::Verify(tags::verify(&read_root, &tag), tags::list(&read_root))
            }
        });
        let mut page = Self {
            entries: vec![],
            selected: 0,
            input: None,
            confirm: None,
            feedback: String::new(),
            pending: false,
            pending_read: None,
            read_worker,
            verification: None,
            scroll: 0,
            view: std::cell::Cell::new((0, 1, 1)),
            sender,
            receiver,
        };
        page.run(root.to_path_buf(), Action::Refresh);
        page
    }
    fn run(&mut self, root: PathBuf, action: Action) {
        self.pending = true;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let message = match action {
                Action::Refresh => Ok(String::new()),
                Action::Create(preview) => {
                    crate::writes::serialized(|| tags::create(&root, &preview))
                        .map(|()| "Lightweight tag created.".into())
                }
                Action::Delete(tag) => crate::writes::serialized(|| tags::delete(&root, &tag))
                    .map(|()| "Local tag deleted.".into()),
            };
            let _ = sender.send((message, tags::list(&root)));
        });
    }
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        while let Ok((message, entries)) = self.receiver.try_recv() {
            changed = true;
            self.pending = false;
            match message {
                Ok(text) => self.feedback = text,
                Err(error) => self.feedback = error,
            }
            match entries {
                Ok(entries) => self.entries = entries,
                Err(error) => self.feedback = error,
            }
            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
            if self.verification.is_some() {
                self.verification = Some(self.feedback.clone());
            }
        }
        if let Some(result) = self.read_worker.take_result()
            && self.pending_read.take().is_some()
        {
            changed = true;
            match result {
                ReadResult::Create(Ok(preview)) => {
                    self.confirm = Some(Action::Create(preview));
                    self.input = None;
                    self.feedback.clear();
                }
                ReadResult::Create(Err(error)) => self.feedback = error,
                ReadResult::Verify(result, entries) => {
                    match entries {
                        Ok(entries) => {
                            self.entries = entries;
                            self.selected = self.selected.min(self.entries.len().saturating_sub(1));
                        }
                        Err(error) => self.feedback = error,
                    }
                    self.scroll = 0;
                    self.verification = Some(match result {
                        Ok(result) => format!(
                            "{}\n{}",
                            if result.verified {
                                "Git signature verification succeeded."
                            } else {
                                "Git signature verification failed or this tag has no verifiable signature."
                            },
                            result.output
                        ),
                        Err(error) => error,
                    });
                }
            }
        }
        changed
    }
    pub fn paste(&mut self, text: &str) {
        if !self.pending
            && self.pending_read.is_none()
            && let Some(input) = &mut self.input
        {
            input.push_str(text);
        }
    }
    /// True on the tag list or a verification, so vim motions apply. The
    /// name prompt and confirmations keep plain keys.
    pub fn takes_motions(&self) -> bool {
        self.pending_read.is_none()
            && !self.pending
            && self.input.is_none()
            && self.confirm.is_none()
    }

    /// Scroll the verification or move the list selection by a motion.
    pub fn motion(&mut self, motion: crate::motion::Motion) {
        let (max, page, list_page) = self.view.get();
        if self.verification.is_some() {
            let offset = motion.scroll(usize::from(self.scroll).min(max), max, page);
            self.scroll = u16::try_from(offset).unwrap_or(u16::MAX);
        } else if let Some(selected) =
            motion.select(Some(self.selected), self.entries.len(), list_page)
        {
            self.selected = selected;
        }
    }

    /// Return false when the user leaves the tag page.
    pub fn key(&mut self, root: &Path, key: KeyCode) -> bool {
        if self.pending_read.is_some() {
            if key == KeyCode::Esc {
                self.read_worker.cancel();
                self.pending_read = None;
            }
            return true;
        }
        if self.pending {
            return true;
        }
        if self.verification.is_some() {
            match key {
                KeyCode::Esc => self.verification = None,
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                _ => {}
            }
            return true;
        }
        if let Some(input) = &mut self.input {
            match key {
                KeyCode::Esc => self.input = None,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Enter => {
                    self.pending_read = Some(ReadKind::Create);
                    self.read_worker.request(ReadRequest::Create(input.clone()));
                }
                KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                _ => {}
            }
            return true;
        }
        if self.confirm.is_some() {
            match key {
                KeyCode::Char('y') => {
                    let action = self.confirm.take().unwrap();
                    self.run(root.to_path_buf(), action);
                }
                KeyCode::Esc | KeyCode::Char('n') => self.confirm = None,
                _ => {}
            }
            return true;
        }
        match key {
            KeyCode::Esc|KeyCode::Char('q')=>return false,
            KeyCode::Char('c')=>{self.input=Some(String::new());self.feedback.clear();},
            KeyCode::Char('d')=>if let Some(tag)=self.entries.get(self.selected) {self.confirm=Some(Action::Delete(tag.clone()));},
            KeyCode::Char('v')=>if let Some(tag)=self.entries.get(self.selected) {self.pending_read=Some(ReadKind::Verify);self.read_worker.request(ReadRequest::Verify(tag.clone()));},
            KeyCode::Char('r')=>self.run(root.to_path_buf(),Action::Refresh),
            KeyCode::Down|KeyCode::Char('j')=>self.selected=(self.selected+1).min(self.entries.len().saturating_sub(1)),
            KeyCode::Up|KeyCode::Char('k')=>self.selected=self.selected.saturating_sub(1),
            KeyCode::Char('?')=>self.feedback="c: create lightweight tag at HEAD; d: confirm local deletion; v: verify signature; r: refresh; Esc: return to branches.".into(), _=>{}
        }
        true
    }
    pub fn render(
        &self,
        frame: &mut Viewport<'_, '_>,
        border: ratatui::symbols::border::Set<'static>,
    ) {
        if let Some(kind) = self.pending_read {
            crate::ui::render_in(
                frame,
                crate::ui::Screen::PreviewLoading {
                    message: match kind {
                        ReadKind::Create => "Reviewing lightweight tag target...",
                        ReadKind::Verify => "Verifying tag signature...",
                    },
                },
                border,
            );
            return;
        }
        use crate::ui::{
            ACCENT, MUTED, hint_row_count, page_block, render_key_hints, selection_style,
        };
        let hints: &[(&str, &str)] = if self.pending {
            &[("Working", "operation in progress")]
        } else if self.verification.is_some() {
            &[("j/k", "scroll"), ("Esc", "back")]
        } else if self.input.is_some() {
            &[("Enter", "review target"), ("Esc", "cancel")]
        } else if self.confirm.is_some() {
            &[("y", "confirm"), ("n/Esc", "cancel")]
        } else {
            &[
                ("j/k", "select"),
                ("c", "create"),
                ("d", "delete"),
                ("v", "verify"),
                ("r", "refresh"),
                ("?", "help"),
                ("Esc", "branches"),
            ]
        };
        let areas = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(hint_row_count(hints, frame.area().width)),
        ])
        .split(frame.area());
        let (title, body) = if let Some(text) = &self.verification {
            (
                "TAG SIGNATURE VERIFICATION",
                Text::from(if self.pending {
                    "Verifying reviewed tag object...".to_owned()
                } else {
                    text.clone()
                }),
            )
        } else if let Some(input) = &self.input {
            (
                "CREATE LIGHTWEIGHT TAG",
                Text::from(vec![
                    Line::from(vec![
                        Span::styled("Tag name: ", Style::default().fg(MUTED)),
                        Span::styled(format!("{input}_"), Style::default().fg(ACCENT)),
                    ]),
                    Line::raw(""),
                    Line::styled(
                        self.feedback.clone(),
                        Style::default().fg(Color::Rgb(251, 191, 36)),
                    ),
                ]),
            )
        } else if let Some(action) = &self.confirm {
            let text = match action {
                Action::Create(preview) => format!(
                    "Create lightweight tag {} at commit {}?\n\nThis tag will have no signature. Existing tags are never overwritten.",
                    preview.name(),
                    preview.target()
                ),
                Action::Delete(tag) => format!(
                    "Delete local tag {}?\n\nReviewed object: {}\nRemote tags are unaffected. A changed object prevents deletion.",
                    tag.name.to_string_lossy(),
                    tag.object
                ),
                _ => String::new(),
            };
            ("CONFIRM TAG OPERATION", Text::from(text))
        } else {
            let mut lines = Vec::new();
            if self.pending {
                lines.push(Line::styled(
                    "Loading tags / operation in progress...",
                    Style::default().fg(MUTED),
                ));
            }
            if self.entries.is_empty() && !self.pending {
                lines.push(Line::styled("No local tags.", Style::default().fg(MUTED)));
            }
            let visible = usize::from(areas[0].height.saturating_sub(6)).max(1);
            let (max, page, _) = self.view.get();
            self.view.set((max, page, visible));
            let start = self.selected.saturating_sub(visible.saturating_sub(1));
            for (index, tag) in self.entries.iter().enumerate().skip(start).take(visible) {
                let selected = index == self.selected;
                let text = format!(
                    "{} {}  [{}]  {}",
                    if selected { "›" } else { " " },
                    tag.name.to_string_lossy(),
                    if tag.kind == "tag" {
                        "annotated"
                    } else {
                        "lightweight"
                    },
                    tag.object
                );
                lines.push(Line::styled(
                    text,
                    if selected {
                        selection_style()
                    } else {
                        Style::default()
                    },
                ));
            }
            lines.push(Line::raw(""));
            lines.extend(self.feedback.lines().map(|line| {
                Line::styled(
                    line.to_owned(),
                    Style::default().fg(Color::Rgb(251, 191, 36)),
                )
            }));
            ("LOCAL TAGS", Text::from(lines))
        };
        let block = page_block(title, border);
        if self.verification.is_some() {
            let inner = block.inner(areas[0]);
            let page = usize::from(inner.height).max(1);
            let (_, _, list_page) = self.view.get();
            self.view.set((
                crate::ui::wrapped_rows(&body, inner.width).saturating_sub(page),
                page,
                list_page,
            ));
        }
        frame.render_widget(
            Paragraph::new(body)
                .wrap(Wrap { trim: false })
                .scroll((
                    if self.verification.is_some() {
                        self.scroll
                    } else {
                        0
                    },
                    0,
                ))
                .block(block),
            areas[0],
        );
        render_key_hints(frame, areas[1], hints);
    }
}
