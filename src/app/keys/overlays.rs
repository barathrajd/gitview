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
    let Env {
        terminal,
        host,
        root,
        config_dir,
        profiles,
        ..
    } = env;
    if app.help {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
            app.help = false;
        }
        return Ok(Flow::Handled);
    }
    if app.pending_preview.is_some() {
        if key.code == KeyCode::Esc {
            app.preview_worker.cancel();
            app.pending_preview = None;
        }
        return Ok(Flow::Handled);
    }
    if app.commit_preview_mode.is_some() {
        if key.code == KeyCode::Esc {
            app.commit_preview_worker.cancel();
            app.commit_preview_mode = None;
            if std::mem::take(&mut app.commit_preview_retry) {
                app.commit_feedback.push_str(
                    "\nPreview refresh cancelled; reopen the composer to review staged changes.",
                );
            } else {
                app.commit_feedback.clear();
            }
        }
        return Ok(Flow::Handled);
    }
    if let Some(page) = &mut app.object_page {
        if page.takes_motions() {
            use crate::motion::Parsed;
            match app.page_motions.key(key) {
                Parsed::Pending => return Ok(Flow::Handled),
                Parsed::Motion(motion) => {
                    page.motion(motion);
                    return Ok(Flow::Handled);
                }
                Parsed::Other => {}
            }
        } else {
            app.page_motions.reset();
        }
        if !page.key(root, plain_code(key)) {
            app.object_page = None;
        }
        return Ok(Flow::Handled);
    }
    if let Some(page) = &mut app.comparison_page {
        if page.takes_motions() {
            use crate::motion::Parsed;
            match app.page_motions.key(key) {
                Parsed::Pending => return Ok(Flow::Handled),
                Parsed::Motion(motion) => {
                    page.motion(motion);
                    return Ok(Flow::Handled);
                }
                Parsed::Other => {}
            }
        } else {
            app.page_motions.reset();
        }
        if !page.key(root, plain_code(key)) {
            app.comparison_page = None;
        }
        return Ok(Flow::Handled);
    }
    if let Some(input) = &mut app.history_path_input {
        match key.code {
            KeyCode::Esc => app.history_path_input = None,
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
            KeyCode::Enter => {
                app.history_filter = (!input.is_empty()).then(|| std::path::PathBuf::from(&*input));
                app.history_path_input = None;
                app.history_request += 1;
                let id = app.history_request;
                let sender = app.history_sender.clone();
                let path = root.to_path_buf();
                let filter = app.history_filter.clone();
                app.history = None;
                app.selection.select(None);
                std::thread::spawn(move || {
                    let _ = sender.send((
                        id,
                        crate::history::load_history_for_path(&path, filter.as_deref()),
                    ));
                });
            }
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if let Some(page) = &mut app.tag_page {
        if page.takes_motions() {
            use crate::motion::Parsed;
            match app.page_motions.key(key) {
                Parsed::Pending => return Ok(Flow::Handled),
                Parsed::Motion(motion) => {
                    page.motion(motion);
                    return Ok(Flow::Handled);
                }
                Parsed::Other => {}
            }
        } else {
            app.page_motions.reset();
        }
        if !page.key(root, plain_code(key)) {
            app.tag_page = None;
            app.history_request += 1;
            let id = app.history_request;
            let sender = app.history_sender.clone();
            let path = root.to_path_buf();
            app.history = None;
            let requested_history_path = app.history_filter.clone();
            std::thread::spawn(move || {
                let _ = sender.send((
                    id,
                    crate::history::load_history_for_path(&path, requested_history_path.as_deref()),
                ));
            });
        }
        return Ok(Flow::Handled);
    }
    if let Some(page) = &mut app.sessions_page {
        if page.takes_motions() {
            use crate::motion::Parsed;
            match app.page_motions.key(key) {
                Parsed::Pending => return Ok(Flow::Handled),
                Parsed::Motion(motion) => {
                    page.motion(motion, screen_page_rows(app.gitview_area, PAGE_CHROME));
                    return Ok(Flow::Handled);
                }
                Parsed::Other => {}
            }
        } else {
            app.page_motions.reset();
        }
        if !page.key(root, plain_code(key)) {
            app.sessions_page = None;
        }
        return Ok(Flow::Handled);
    }
    if app.profiles_view {
        let page = screen_page_rows(app.gitview_area, PAGE_CHROME);
        if list_motion(
            &mut app.page_motions,
            key,
            &mut app.profile_selection,
            profiles.len(),
            page,
        ) {
            return Ok(Flow::Handled);
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => app.profiles_view = false,
            KeyCode::Char('?') => app.help = true,
            KeyCode::Char('q') => return Ok(Flow::Quit),
            KeyCode::Char('e') => edit_profile_registry(
                &mut **terminal,
                &mut **host,
                root,
                config_dir,
                profiles,
                &mut app.profile_label,
                &mut app.profile_feedback,
            )?,
            KeyCode::Char('r') => {
                match reload_profile_context(root, config_dir, profiles, &mut app.profile_label) {
                    Ok(()) => app.profile_feedback = "Git config profiles reloaded.".into(),
                    Err(error) => app.profile_feedback = error,
                }
            }
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    Ok(Flow::Next)
}
