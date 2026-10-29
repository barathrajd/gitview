use std::io;

use crate::app::{
    helpers::*,
    model::App,
    run::{Env, Flow, Host},
};
use crossterm::event::{KeyCode, KeyEvent};

pub(in crate::app) fn handle<H: Host>(
    app: &mut App,
    env: &mut Env<'_, H>,
    key: KeyEvent,
) -> io::Result<Flow> {
    let Env { root, .. } = env;
    if let Some(path_text) = app.ignore_path_input.as_mut() {
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Esc => {
                app.ignore_path_input = None;
                app.commit_feedback.clear();
            }
            KeyCode::Backspace => {
                path_text.pop();
            }
            KeyCode::Enter => {
                let user_path = std::path::PathBuf::from(&*path_text);
                app.pending_preview = Some(crate::preview::Context::IgnorePath);
                app.commit_feedback.clear();
                app.preview_worker
                    .request(crate::preview::Request::IgnorePath(user_path));
            }
            KeyCode::Char(ch) if !ch.is_control() => path_text.push(ch),
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if app.move_path_input {
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Esc => {
                app.move_path_input = false;
                app.move_source.clear();
                app.move_destination.clear();
                app.commit_feedback.clear();
            }
            KeyCode::Tab => app.move_destination_focused = !app.move_destination_focused,
            KeyCode::Backspace => {
                if app.move_destination_focused {
                    app.move_destination.pop();
                } else {
                    app.move_source.pop();
                }
            }
            KeyCode::Enter if !app.move_destination_focused => {
                if app.move_source.is_empty() {
                    app.commit_feedback = "Enter the tracked source path first.".into();
                } else {
                    app.move_destination_focused = true;
                    app.commit_feedback.clear();
                }
            }
            KeyCode::Enter => {
                app.pending_preview = Some(crate::preview::Context::Move);
                app.commit_feedback.clear();
                app.preview_worker.request(crate::preview::Request::Move(
                    app.move_source.clone().into(),
                    app.move_destination.clone().into(),
                ));
            }
            KeyCode::Char(ch) if !ch.is_control() => {
                if app.move_destination_focused {
                    app.move_destination.push(ch);
                } else {
                    app.move_source.push(ch);
                }
            }
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if let Some(path_text) = app.remove_path_input.as_mut() {
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Esc => {
                app.remove_path_input = None;
                app.commit_feedback.clear();
            }
            KeyCode::Backspace => {
                path_text.pop();
            }
            KeyCode::Enter => {
                app.pending_preview = Some(crate::preview::Context::Remove);
                app.commit_feedback.clear();
                app.preview_worker.request(crate::preview::Request::Remove(
                    std::path::PathBuf::from(&*path_text),
                ));
            }
            KeyCode::Char(ch) if !ch.is_control() => path_text.push(ch),
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if app.amend_confirm {
        if app.mutation_pending {
            return Ok(Flow::Handled);
        }
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('y') => {
                let subject = app.commit_message.clone().unwrap_or_default();
                let description = app.commit_description.clone();
                let message = combined_commit_message(&subject, &description);
                let expected = app.amend_expected.clone();
                let path = root.to_path_buf();
                let sender = app.commit_sender.clone();
                app.mutation_pending = true;
                std::thread::spawn(move || {
                    let result = expected
                        .as_ref()
                        .ok_or_else(|| "Amend preview is unavailable".to_owned())
                        .and_then(|preview| crate::commit::amend_commit(&path, &message, preview));
                    let _ = sender.send((subject, description, true, result));
                });
            }
            KeyCode::Char('n') | KeyCode::Esc => app.amend_confirm = false,
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    Ok(Flow::Next)
}
