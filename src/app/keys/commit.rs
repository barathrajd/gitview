use std::io;

use crate::app::{
    helpers::*,
    model::App,
    run::{Env, Flow, Host},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(in crate::app) fn handle<H: Host>(
    app: &mut App,
    env: &mut Env<'_, H>,
    key: KeyEvent,
) -> io::Result<Flow> {
    let Env {
        terminal,
        host,
        root,
        ..
    } = env;
    if app.commit_message.is_some() {
        if app.mutation_pending {
            return Ok(Flow::Handled);
        }
        let mut submit = false;
        match key.code {
            KeyCode::Esc => {
                app.commit_message = None;
                app.commit_description.clear();
                app.commit_description_focused = false;
                app.commit_expected = None;
                app.amend_expected = None;
                app.commit_amend = false;
                app.commit_feedback.clear();
            }
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let message = combined_commit_message(
                    app.commit_message.as_deref().unwrap_or_default(),
                    &app.commit_description,
                );
                match crate::editor::configured_editor(root) {
                    Err(error) => app.commit_feedback = error,
                    Ok(editor) => {
                        let result = host.run_external(terminal, || {
                            crate::editor::edit_with_command(root, &editor, &message)
                        })?;
                        match result {
                            Ok(edited) => {
                                submit = commit_after_editor(
                                    app.commit_message.as_deref().unwrap_or_default(),
                                    &app.commit_description,
                                    &edited,
                                );
                                let (subject, description) = split_commit_message(&edited);
                                app.commit_message = Some(subject);
                                app.commit_description = description;
                                app.commit_feedback = if submit {
                                    String::new()
                                } else {
                                    "Returned from editor; review the message before committing."
                                        .into()
                                };
                            }
                            Err(error) => app.commit_feedback = error,
                        }
                    }
                }
            }
            KeyCode::Tab => app.commit_description_focused = !app.commit_description_focused,
            KeyCode::Backspace => {
                if app.commit_description_focused {
                    app.commit_description.pop();
                } else if let Some(message) = &mut app.commit_message {
                    message.pop();
                }
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.commit_description_focused {
                    app.commit_description.push('\n');
                }
            }
            KeyCode::Enter | KeyCode::Char('\r' | '\n') => submit = true,
            KeyCode::Char(character) if !character.is_control() => {
                if app.commit_description_focused {
                    app.commit_description.push(character);
                } else if let Some(message) = &mut app.commit_message {
                    message.push(character);
                }
                app.commit_feedback.clear();
            }
            _ => {}
        }
        if submit {
            let message = app.commit_message.as_deref().unwrap_or_default();
            if message.trim().is_empty() {
                app.commit_feedback = "Enter a commit message.".into();
            } else if (app.commit_amend && app.amend_expected.is_none())
                || (!app.commit_amend && app.commit_expected.is_none())
            {
                app.commit_feedback = if app.commit_amend {
                    "Amend preview unavailable; reopen the composer to review changes.".into()
                } else {
                    "Commit preview unavailable; reopen the composer to review staged changes."
                        .into()
                };
            } else {
                let subject = message.to_owned();
                let description = app.commit_description.clone();
                if app.commit_amend {
                    app.amend_confirm = true;
                    app.commit_feedback.clear();
                } else {
                    let message = combined_commit_message(&subject, &description);
                    let staged = app.commit_expected.clone();
                    let path = root.to_path_buf();
                    let sender = app.commit_sender.clone();
                    app.mutation_pending = true;
                    app.commit_feedback.clear();
                    std::thread::spawn(move || {
                        let result = staged
                            .as_ref()
                            .ok_or_else(|| "Staged changes are unavailable".to_owned())
                            .and_then(|preview| {
                                crate::commit::create_commit(&path, &message, preview)
                            });
                        let _ = sender.send((subject, description, false, result));
                    });
                }
            }
        }
        return Ok(Flow::Handled);
    }
    Ok(Flow::Next)
}
