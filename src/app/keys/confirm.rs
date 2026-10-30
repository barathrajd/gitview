use std::io;

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    model::App,
    run::{Env, Flow, Host},
};

pub(in crate::app) fn handle<H: Host>(
    app: &mut App,
    env: &mut Env<'_, H>,
    key: KeyEvent,
) -> io::Result<Flow> {
    let Env { root, .. } = env;
    if let Some(preview) = app.discard_confirm.as_ref() {
        if app.mutation_pending {
            return Ok(Flow::Handled);
        }
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('y') => {
                app.mutation_pending = true;
                let preview = preview.clone();
                let path = root.to_path_buf();
                let sender = app.discard_sender.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(crate::writes::serialized(|| {
                        crate::worktree::discard(&path, &preview)
                    }));
                });
            }
            KeyCode::Char('n') | KeyCode::Esc => app.discard_confirm = None,
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if let Some(preview) = app.untrack_confirm.as_ref() {
        if app.mutation_pending {
            return Ok(Flow::Handled);
        }
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('y') => {
                app.mutation_pending = true;
                let preview = preview.clone();
                let path = root.to_path_buf();
                let sender = app.untrack_sender.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(crate::writes::serialized(|| {
                        crate::worktree::untrack(&path, &preview)
                    }));
                });
            }
            KeyCode::Char('n') | KeyCode::Esc => app.untrack_confirm = None,
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if let Some(preview) = app.remove_confirm.as_ref() {
        if app.mutation_pending {
            return Ok(Flow::Handled);
        }
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('y') => {
                app.mutation_pending = true;
                let preview = preview.clone();
                let path = root.to_path_buf();
                let sender = app.remove_sender.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(crate::writes::serialized(|| {
                        crate::worktree::remove_path(&path, &preview)
                    }));
                });
            }
            KeyCode::Char('n') | KeyCode::Esc => app.remove_confirm = None,
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if let Some(preview) = app.move_confirm.as_ref() {
        if app.mutation_pending {
            return Ok(Flow::Handled);
        }
        match key.code {
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('y') => {
                app.mutation_pending = true;
                let preview = preview.clone();
                let path = root.to_path_buf();
                let sender = app.move_sender.clone();
                std::thread::spawn(move || {
                    let _ = sender.send(crate::writes::serialized(|| {
                        crate::worktree::move_path(&path, &preview)
                    }));
                });
            }
            KeyCode::Char('n') | KeyCode::Esc => app.move_confirm = None,
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    Ok(Flow::Next)
}
