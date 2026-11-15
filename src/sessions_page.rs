//! The Sessions page: list agent sessions, review one, and merge or discard
//! it after a confirmation. Git work runs off the render loop.

use crate::motion::Motion;
use crate::sessions::{self, DiscardPreview, Health, Review, Session, SessionStatus};
use crate::ui::Viewport;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span, Text},
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

/// The Sessions section's rows: each session's name and state, the
/// selection, and an open `/` search's query and prompt line.
#[derive(Clone, Debug, Default)]
pub struct SessionSection {
    pub rows: Vec<(String, String)>,
    pub selected: usize,
    pub search: Option<(String, String)>,
    pub loading: bool,
}

pub struct SessionsPage {
    entries: Vec<SessionStatus>,
    selected: usize,
    mode: Mode,
    feedback: String,
    pending: bool,
    /// The list is reloading; unlike `pending`, keys still work.
    refreshing: bool,
    search: Option<crate::list_search::InlineSearch>,
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
            refreshing: false,
            search: None,
            sender,
            receiver,
        };
        page.run(root.to_path_buf(), Request::Refresh);
        page
    }

    /// Reload the list, unless Git is already working.
    pub fn refresh(&mut self, root: &Path) {
        if !self.pending && !self.refreshing {
            self.run(root.to_path_buf(), Request::Refresh);
        }
    }

    /// True on the plain list, where workspace keys such as Tab pass
    /// through, even while the list reloads; a review, confirmation, or
    /// search takes every key.
    pub fn in_list(&self) -> bool {
        matches!(self.mode, Mode::List) && self.search.is_none()
    }

    /// Keys the list uses; [`SessionsPage::key`] handles them.
    pub fn uses_list_key(key: KeyCode) -> bool {
        matches!(
            key,
            KeyCode::Esc
                | KeyCode::Enter
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Char('j' | 'k' | 'd' | 'r' | '/' | 'n' | 'N')
        )
    }

    /// The page opens in the centre panel for a review or a confirmation;
    /// the list itself is the sidebar's Sessions section.
    pub fn shows_centre(&self) -> bool {
        !matches!(self.mode, Mode::List)
    }

    /// Back to the list, as the agent's control channel asks.
    pub fn back_to_list(&mut self) {
        self.mode = Mode::List;
        self.search = None;
    }

    /// The last merge, discard, or error message.
    pub fn feedback(&self) -> &str {
        &self.feedback
    }

    /// What the sidebar's Sessions section draws.
    pub fn section(&self) -> SessionSection {
        SessionSection {
            rows: self
                .entries
                .iter()
                .map(|entry| (entry.session.name.clone(), describe_short(entry)))
                .collect(),
            selected: self.selected,
            search: self
                .search
                .as_ref()
                .map(|search| (search.query.clone(), search.line(self.selected))),
            loading: self.refreshing && self.entries.is_empty(),
        }
    }

    fn run(&mut self, root: PathBuf, request: Request) {
        // A reload leaves the list usable; reviews and writes block keys.
        if matches!(request, Request::Refresh) {
            self.refreshing = true;
        } else {
            self.pending = true;
        }
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
            if matches!(reply, Reply::Listed(_)) {
                self.refreshing = false;
            } else {
                self.pending = false;
            }
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
        !self.pending
            && self.search.is_none()
            && matches!(self.mode, Mode::List | Mode::Review { .. })
    }

    /// Session names, which `/` matches.
    fn names(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.session.name.clone())
            .collect()
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

    /// Return false when the user leaves the page. `/` searches session
    /// names; Enter saves the query in `last_search` for `n`/`N`.
    pub fn key(&mut self, root: &Path, key: KeyCode, last_search: &mut String) -> bool {
        if self.pending {
            return true;
        }
        if self.search.is_some() {
            let names = self.names();
            let Some(search) = &mut self.search else {
                return true;
            };
            if !search.key(key, &names, &mut self.selected, last_search) {
                self.search = None;
            }
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
                    KeyCode::Char('/') => {
                        self.search = Some(crate::list_search::InlineSearch::start(self.selected));
                    }
                    KeyCode::Char(letter @ ('n' | 'N')) => {
                        if let Some(index) = crate::list_search::step(
                            &self.names(),
                            last_search,
                            self.selected,
                            letter == 'n',
                        ) {
                            self.selected = index;
                        }
                    }
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
        use crate::ui::{ACCENT, MUTED, page_block, render_key_hints, selection_style};
        let hints: &[(&str, &str)] = if self.pending {
            &[("Working", "Git is running")]
        } else if self.search.is_some() {
            &[("Enter", "stay"), ("Esc", "back")]
        } else {
            match &self.mode {
                Mode::List => &[
                    ("j/k", "select"),
                    ("Enter", "review"),
                    ("/", "search"),
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
            Constraint::Length(crate::ui::hints::hint_height(frame, hints)),
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
                let mut finder = crate::history::MatchFinder::new(
                    self.search
                        .as_ref()
                        .map_or("", |search| search.query.as_str()),
                );
                for (index, entry) in self.entries.iter().enumerate() {
                    let selected = index == self.selected;
                    let name = &entry.session.name;
                    let mut spans = vec![Span::raw(if selected { "› " } else { "  " })];
                    spans.extend(crate::ui::highlighted_spans(name, &finder.positions(name)));
                    spans.push(Span::raw(format!("  {}", describe(entry))));
                    lines.push(Line::from(spans).style(if selected {
                        selection_style()
                    } else {
                        Style::default()
                    }));
                }
                if let Some(search) = &self.search {
                    lines.push(Line::raw(""));
                    lines.push(Line::styled(
                        search.line(self.selected),
                        Style::default().fg(ACCENT),
                    ));
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

/// The state in a few words for the narrow Sessions section: `2 ahead`,
/// `clean`, `+ edits`, or what needs cleaning up. The review shows the
/// full description.
fn describe_short(entry: &SessionStatus) -> String {
    match entry.health {
        Health::MissingWorktree => "no worktree".into(),
        Health::MissingBranch => "no branch".into(),
        Health::Unrecorded => "left behind".into(),
        Health::Ready => {
            let mut text = match entry.ahead {
                0 => "clean".to_owned(),
                count => format!("{count} ahead"),
            };
            if entry.dirty {
                text.push_str(" + edits");
            }
            text
        }
    }
}

#[cfg(test)]
mod short_tests {
    use super::*;

    fn entry(health: Health, ahead: usize, dirty: bool) -> SessionStatus {
        SessionStatus {
            session: Session {
                name: "session-1".into(),
                branch: "gitview/session-1".into(),
                base_branch: "main".into(),
                path: PathBuf::from("/tmp/session-1"),
                base_commit: String::new(),
            },
            health,
            dirty,
            ahead,
        }
    }

    #[test]
    fn short_states_fit_the_sidebar() {
        assert_eq!(describe_short(&entry(Health::Ready, 0, false)), "clean");
        assert_eq!(describe_short(&entry(Health::Ready, 2, false)), "2 ahead");
        assert_eq!(
            describe_short(&entry(Health::Ready, 1, true)),
            "1 ahead + edits"
        );
        assert_eq!(
            describe_short(&entry(Health::MissingWorktree, 0, false)),
            "no worktree"
        );
    }
}
