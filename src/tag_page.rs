use crate::tags::{self, CreatePreview, Tag};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph, Wrap},
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
#[derive(Clone)]
enum Action {
    Create(CreatePreview),
    Delete(Tag),
    Verify(Tag),
    Refresh,
}
type Reply = (Result<String, String>, Result<Vec<Tag>, String>);
pub struct TagPage {
    entries: Vec<Tag>,
    selected: usize,
    input: Option<String>,
    confirm: Option<Action>,
    feedback: String,
    pending: bool,
    verification: Option<String>,
    scroll: u16,
    sender: mpsc::Sender<Reply>,
    receiver: mpsc::Receiver<Reply>,
}
impl TagPage {
    pub fn open(root: &Path) -> Self {
        let (sender, receiver) = mpsc::channel();
        let mut page = Self {
            entries: vec![],
            selected: 0,
            input: None,
            confirm: None,
            feedback: String::new(),
            pending: false,
            verification: None,
            scroll: 0,
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
            let message=match action {
                Action::Refresh=>Ok(String::new()),
                Action::Create(preview)=>tags::create(&root,&preview).map(|()| "Lightweight tag created.".into()),
                Action::Delete(tag)=>tags::delete(&root,&tag).map(|()| "Local tag deleted.".into()),
                Action::Verify(tag)=>tags::verify(&root,&tag).map(|result| format!("{}\n{}", if result.verified { "Git signature verification succeeded." } else { "Git signature verification failed or this tag has no verifiable signature." }, result.output)),
            };
            let _ = sender.send((message, tags::list(&root)));
        });
    }
    pub fn tick(&mut self) {
        while let Ok((message, entries)) = self.receiver.try_recv() {
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
    }
    pub fn paste(&mut self, text: &str) {
        if let Some(input) = &mut self.input {
            input.push_str(text);
        }
    }
    /// Return false when the user leaves the tag page.
    pub fn key(&mut self, root: &Path, key: KeyCode) -> bool {
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
                KeyCode::Enter => match tags::preview_create(root, input) {
                    Ok(preview) => {
                        self.confirm = Some(Action::Create(preview));
                        self.input = None;
                        self.feedback.clear();
                    }
                    Err(error) => self.feedback = error,
                },
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
            KeyCode::Char('v')=>if let Some(tag)=self.entries.get(self.selected) {let tag=tag.clone();self.verification=Some(String::new());self.scroll=0;self.run(root.to_path_buf(),Action::Verify(tag));},
            KeyCode::Char('r')=>self.run(root.to_path_buf(),Action::Refresh),
            KeyCode::Down|KeyCode::Char('j')=>self.selected=(self.selected+1).min(self.entries.len().saturating_sub(1)),
            KeyCode::Up|KeyCode::Char('k')=>self.selected=self.selected.saturating_sub(1),
            KeyCode::Char('?')=>self.feedback="c: create lightweight tag at HEAD; d: confirm local deletion; v: verify signature; r: refresh; Esc: return to branches.".into(), _=>{}
        }
        true
    }
    pub fn render(&self, frame: &mut Frame<'_>, border: ratatui::symbols::border::Set<'static>) {
        let areas =
            Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(frame.area());
        let (title, text, hints) = if let Some(text) = &self.verification {
            (
                "TAG SIGNATURE VERIFICATION",
                if self.pending {
                    "Verifying reviewed tag object...".into()
                } else {
                    text.clone()
                },
                "j/k scroll · Esc back",
            )
        } else if let Some(input) = &self.input {
            (
                "CREATE LIGHTWEIGHT TAG",
                format!("Tag name: {input}_\n\n{}", self.feedback),
                "Enter review target · Esc cancel",
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
            ("CONFIRM TAG OPERATION", text, "y confirm · n/Esc cancel")
        } else {
            let mut text = if self.pending {
                "Loading tags / operation in progress...\n".into()
            } else {
                String::new()
            };
            if self.entries.is_empty() && !self.pending {
                text.push_str("No local tags.\n");
            }
            let visible = usize::from(areas[0].height.saturating_sub(6)).max(1);
            let start = self.selected.saturating_sub(visible.saturating_sub(1));
            for (index, tag) in self.entries.iter().enumerate().skip(start).take(visible) {
                text.push_str(&format!(
                    "{} {}  [{}]  {}\n",
                    if index == self.selected { ">" } else { " " },
                    tag.name.to_string_lossy(),
                    if tag.kind == "tag" {
                        "annotated"
                    } else {
                        "lightweight"
                    },
                    tag.object
                ));
            }
            text.push_str(&format!("\n{}", self.feedback));
            (
                "LOCAL TAGS",
                text,
                "j/k select · c create · d delete · v verify\nr refresh · ? help · Esc return to branches",
            )
        };
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .scroll((
                    if self.verification.is_some() {
                        self.scroll
                    } else {
                        0
                    },
                    0,
                ))
                .block(
                    Block::bordered()
                        .border_set(border)
                        .title(title)
                        .border_style(Style::default().fg(Color::Cyan)),
                ),
            areas[0],
        );
        frame.render_widget(Paragraph::new(hints), areas[1]);
    }
}
