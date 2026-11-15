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
                let raw = app.history_raw;
                std::thread::spawn(move || {
                    let _ = sender.send((
                        id,
                        crate::history::load_view(&path, filter.as_deref(), raw, 0),
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
        if !page.key(root, plain_code(key), &mut app.last_search) {
            app.tag_page = None;
            app.history_request += 1;
            let id = app.history_request;
            let sender = app.history_sender.clone();
            let path = root.to_path_buf();
            app.history = None;
            let requested_history_path = app.history_filter.clone();
            let raw = app.history_raw;
            std::thread::spawn(move || {
                let _ = sender.send((
                    id,
                    crate::history::load_view(&path, requested_history_path.as_deref(), raw, 0),
                ));
            });
        }
        return Ok(Flow::Handled);
    }
    // On the plain session list, workspace keys pass through: Tab and
    // Shift-Tab, the section and page letters, and Ctrl-w chords.
    let workspace_key = app.workspace_prefix
        || matches!(
            key.code,
            KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Char('W' | 'H' | 'b' | 'A' | 'p' | 'q' | '?')
        ) && !key.modifiers.contains(KeyModifiers::CONTROL)
        || (key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL));
    if let Some(page) = &mut app.sessions_page
        && app.app_state.view() == crate::app::View::Sessions
        && !app.help
        && !(page.in_list() && workspace_key)
    {
        if page.takes_motions() {
            use crate::motion::Parsed;
            match app.page_motions.key(key) {
                Parsed::Pending => return Ok(Flow::Handled),
                Parsed::Motion(motion) => {
                    // The review sits in the centre panel; its border and
                    // hint row take three rows.
                    let centre = app
                        .gitview_area
                        .map(|area| crate::ui::cockpit_centre_area(area, &app.workspace));
                    page.motion(motion, screen_page_rows(centre, 3));
                    return Ok(Flow::Handled);
                }
                Parsed::Other => {}
            }
        } else {
            app.page_motions.reset();
        }
        // Esc on the list hands focus back to History; the page stays.
        if !page.key(root, plain_code(key), &mut app.last_search) {
            app.app_state.show(crate::app::View::History);
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
