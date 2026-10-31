//! The Sessions page: list agent sessions, review one, and merge or discard
//! it after a confirmation. Git work runs off the render loop.

use crate::motion::Motion;
use crate::sessions::{self, DiscardPreview, Health, Review, Session, SessionStatus};
use crate::ui::Viewport;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Text},
    widgets::{Paragraph, Wrap},
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

enum Request {
    Refresh,
    Review(Session),
    PreviewDiscard(Session),
    Merge(Review),
    Discard(DiscardPreview),
}

enum Reply {
    Listed(Result<Vec<SessionStatus>, String>),
    Reviewed(Result<Review, String>),
    DiscardPreviewed(Result<DiscardPreview, String>),
    /// A merge or discard finished: its message and the new list.
    Done(Result<String, String>, Result<Vec<SessionStatus>, String>),
}

enum Mode {
    List,
    Review { review: Review, scroll: u16 },
    ConfirmMerge(Review),
    ConfirmDiscard(DiscardPreview),
}

pub struct SessionsPage {
    entries: Vec<SessionStatus>,
    selected: usize,
    mode: Mode,
    feedback: String,
    pending: bool,
    sender: mpsc::Sender<Reply>,
    receiver: mpsc::Receiver<Reply>,
}

impl SessionsPage {
    pub fn open(root: &Path) -> Self {
        let (sender, receiver) = mpsc::channel();
        let mut page = Self {
            entries: Vec::new(),
            selected: 0,
            mode: Mode::List,
            feedback: String::new(),
            pending: false,
            sender,
            receiver,
        };
        page.run(root.to_path_buf(), Request::Refresh);
        page
    }

    fn run(&mut self, root: PathBuf, request: Request) {
        self.pending = true;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let reply = match request {
                Request::Refresh => Reply::Listed(sessions::list(&root)),
                Request::Review(session) => Reply::Reviewed(sessions::review(&root, &session)),
                Request::PreviewDiscard(session) => {
                    Reply::DiscardPreviewed(sessions::preview_discard(&root, &session))
                }
                Request::Merge(review) => {
                    let name = review.session.name.clone();
                    let base = review.session.base_branch.clone();
                    let result = crate::writes::serialized(|| sessions::merge(&root, &review))
                        .map(|()| format!("Merged {name} into {base}."));
                    Reply::Done(result, sessions::list(&root))
                }
                Request::Discard(preview) => {
                    let name = preview.session.name.clone();
                    let result = crate::writes::serialized(|| sessions::discard(&root, &preview))
                        .map(|()| format!("Discarded {name}."));
                    Reply::Done(result, sessions::list(&root))
                }
            };
            let _ = sender.send(reply);
        });
    }

    /// Apply finished work; true when the page changed.
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        while let Ok(reply) = self.receiver.try_recv() {
            changed = true;
            self.pending = false;
            match reply {
                Reply::Listed(entries) => self.set_entries(entries),
                Reply::Reviewed(Ok(review)) => self.mode = Mode::Review { review, scroll: 0 },
                Reply::DiscardPreviewed(Ok(preview)) => self.mode = Mode::ConfirmDiscard(preview),
                Reply::Reviewed(Err(error)) | Reply::DiscardPreviewed(Err(error)) => {
                    self.feedback = error;
                }
                Reply::Done(result, entries) => {
                    self.mode = Mode::List;
                    self.feedback = result.unwrap_or_else(|error| error);
                    self.set_entries(entries);
                }
            }
        }
        changed
    }

    fn set_entries(&mut self, entries: Result<Vec<SessionStatus>, String>) {
        match entries {
            Ok(entries) => self.entries = entries,
            Err(error) => self.feedback = error,
        }
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    /// True when the page browses a list or a review, so vim motions apply.
    /// Confirmations and a running Git job take plain keys only.
    pub fn takes_motions(&self) -> bool {
        !self.pending && matches!(self.mode, Mode::List | Mode::Review { .. })
    }

    /// Move the list selection or scroll the review, `page` rows at a time.
    pub fn motion(&mut self, motion: Motion, page: usize) {
        match &mut self.mode {
            Mode::List => {
                if let Some(selected) = motion.select(Some(self.selected), self.entries.len(), page)
                {
                    self.selected = selected;
                }
            }
            Mode::Review { review, scroll } => {
                let max = review_lines(review).saturating_sub(page);
                let offset = motion.scroll(usize::from(*scroll), max, page);
                *scroll = u16::try_from(offset).unwrap_or(u16::MAX);
            }
            Mode::ConfirmMerge(_) | Mode::ConfirmDiscard(_) => {}
        }
    }

    /// Return false when the user leaves the page.
    pub fn key(&mut self, root: &Path, key: KeyCode) -> bool {
        if self.pending {
            return true;
        }
        match std::mem::replace(&mut self.mode, Mode::List) {
            Mode::Review { review, scroll } => {
                self.mode = match key {
                    KeyCode::Esc => Mode::List,
                    KeyCode::Char('m') => Mode::ConfirmMerge(review),
                    KeyCode::Down | KeyCode::Char('j') => Mode::Review {
                        review,
                        scroll: scroll.saturating_add(1),
                    },
                    KeyCode::Up | KeyCode::Char('k') => Mode::Review {
                        review,
                        scroll: scroll.saturating_sub(1),
                    },
                    _ => Mode::Review { review, scroll },
                };
            }
            Mode::ConfirmMerge(review) => match key {
                KeyCode::Char('y') => {
                    self.feedback.clear();
                    self.run(root.to_path_buf(), Request::Merge(review));
                }
                KeyCode::Char('n') | KeyCode::Esc => self.mode = Mode::Review { review, scroll: 0 },
                _ => self.mode = Mode::ConfirmMerge(review),
            },
            Mode::ConfirmDiscard(preview) => match key {
                KeyCode::Char('y') => {
                    self.feedback.clear();
                    self.run(root.to_path_buf(), Request::Discard(preview));
                }
                KeyCode::Char('n') | KeyCode::Esc => {}
                _ => self.mode = Mode::ConfirmDiscard(preview),
            },
            Mode::List => {
                let selected = self
                    .entries
                    .get(self.selected)
                    .map(|entry| entry.session.clone());
                match key {
                    KeyCode::Esc | KeyCode::Char('q') => return false,
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.selected =
                            (self.selected + 1).min(self.entries.len().saturating_sub(1));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.selected = self.selected.saturating_sub(1);
                    }
                    KeyCode::Enter => {
                        if let Some(session) = selected {
                            self.feedback.clear();
                            self.run(root.to_path_buf(), Request::Review(session));
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(session) = selected {
                            self.feedback.clear();
                            self.run(root.to_path_buf(), Request::PreviewDiscard(session));
                        }
                    }
                    KeyCode::Char('r') => self.run(root.to_path_buf(), Request::Refresh),
                    _ => {}
                }
            }
        }
        true
    }

    pub fn render(
        &self,
        frame: &mut Viewport<'_, '_>,
        border: ratatui::symbols::border::Set<'static>,
    ) {
        use crate::ui::{MUTED, hint_row_count, page_block, render_key_hints, selection_style};
        let hints: &[(&str, &str)] = if self.pending {
            &[("Working", "Git is running")]
        } else {
            match &self.mode {
                Mode::List => &[
                    ("j/k", "select"),
                    ("Enter", "review"),
                    ("d", "discard"),
                    ("r", "refresh"),
                    ("Esc", "back"),
                ],
                Mode::Review { .. } => &[("j/k", "scroll"), ("m", "merge"), ("Esc", "back")],
                Mode::ConfirmMerge(_) | Mode::ConfirmDiscard(_) => {
                    &[("y", "confirm"), ("n/Esc", "cancel")]
                }
            }
        };
        let areas = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(hint_row_count(hints, frame.area().width)),
        ])
        .split(frame.area());
        let muted = Style::default().fg(MUTED);
        let mut lines = Vec::new();
        if !self.feedback.is_empty() {
            lines.push(Line::raw(self.feedback.clone()));
            lines.push(Line::raw(""));
        }
        let (title, scroll) = match &self.mode {
            Mode::List => {
                if self.pending {
                    lines.push(Line::styled("Working...", muted));
                } else if self.entries.is_empty() {
                    lines.push(Line::styled(
                        "No agent sessions. Start one with gitview --session --claude.",
                        muted,
                    ));
                }
                for (index, entry) in self.entries.iter().enumerate() {
                    let text = format!(
                        "{} {}  {}",
                        if index == self.selected { "›" } else { " " },
                        entry.session.name,
                        describe(entry)
                    );
                    lines.push(if index == self.selected {
                        Line::styled(text, selection_style())
                    } else {
                        Line::raw(text)
                    });
                }
                ("AGENT SESSIONS", 0)
            }
            Mode::Review { review, scroll } => {
                let session = &review.session;
                lines.push(Line::raw(format!(
                    "{} on {}, from {}",
                    session.name, session.branch, session.base_branch
                )));
                lines.push(Line::raw(""));
                if review.commits.is_empty() {
                    lines.push(Line::styled("No commits yet.", muted));
                }
                lines.extend(
                    review
                        .commits
                        .iter()
                        .map(|commit| Line::raw(commit.clone())),
                );
                lines.push(Line::raw(""));
                lines.extend(review.diff.lines().map(|line| Line::raw(line.to_owned())));
                ("REVIEW SESSION", *scroll)
            }
            Mode::ConfirmMerge(review) => {
                let session = &review.session;
                lines.push(Line::raw(format!(
                    "Merge {} into {} with a merge commit?",
                    session.name, session.base_branch
                )));
                lines.push(Line::raw(""));
                lines.push(Line::raw(format!(
                    "{} commit(s) from {}. Gitview refuses if {} or the session changed since this review, or if either has uncommitted changes. A conflict stops the merge and changes nothing.",
                    review.commits.len(),
                    session.branch,
                    session.base_branch
                )));
                ("CONFIRM MERGE", 0)
            }
            Mode::ConfirmDiscard(preview) => {
                let session = &preview.session;
                lines.push(Line::raw(format!(
                    "Discard {}? Its worktree and the branch {} will be deleted.",
                    session.name, session.branch
                )));
                lines.push(Line::raw(""));
                if preview.unmerged.is_empty() && preview.uncommitted.is_empty() {
                    lines.push(Line::raw(
                        "Nothing will be lost: every commit is on another branch.",
                    ));
                } else {
                    lines.push(Line::raw("This work will be lost:"));
                    lines.extend(
                        preview
                            .unmerged
                            .iter()
                            .map(|commit| Line::raw(format!("  commit {commit}"))),
                    );
                    lines.extend(
                        preview
                            .uncommitted
                            .iter()
                            .map(|file| Line::raw(format!("  file   {file}"))),
                    );
                }
                ("CONFIRM DISCARD", 0)
            }
        };
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(page_block(title, border))
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0)),
            areas[0],
        );
        render_key_hints(frame, areas[1], hints);
    }
}

/// Unwrapped rows of the review: the header, commits, and diff.
fn review_lines(review: &Review) -> usize {
    3 + review.commits.len().max(1) + review.diff.lines().count()
}

fn describe(entry: &SessionStatus) -> String {
    let session = &entry.session;
    match entry.health {
        Health::MissingWorktree => "worktree folder is missing; discard to clean up".into(),
        Health::MissingBranch => "branch is missing; discard to clean up".into(),
        Health::Unrecorded => "left behind by a crash; discard to clean up".into(),
        Health::Ready => {
            let commits = match entry.ahead {
                1 => "1 commit".to_owned(),
                count => format!("{count} commits"),
            };
            let mut text = format!("{commits} ahead of {}", session.base_branch);
            if entry.dirty {
                text.push_str(", uncommitted work");
            }
            text
        }
    }
}
