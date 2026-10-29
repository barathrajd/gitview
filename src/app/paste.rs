use std::io;

use super::{
    helpers::*,
    model::App,
    run::{Env, Flow, Host},
};
use crossterm::event::Event;

pub(super) fn handle_paste<H: Host>(
    app: &mut App,
    _env: &mut Env<'_, H>,
    event: &Event,
) -> io::Result<Flow> {
    if let Event::Paste(pasted) = &event {
        if app.pending_preview.is_some() || app.commit_preview_mode.is_some() {
            return Ok(Flow::Handled);
        }
        if let Some(page) = &mut app.object_page {
            page.paste(pasted);
            return Ok(Flow::Handled);
        }
        if let Some(page) = &mut app.comparison_page {
            page.paste(pasted);
            return Ok(Flow::Handled);
        }
        if let Some(path) = &mut app.history_path_input {
            if !app.help {
                path.push_str(pasted);
            }
            return Ok(Flow::Handled);
        }
        if let Some(page) = &mut app.tag_page {
            page.paste(pasted);
            return Ok(Flow::Handled);
        }
        if app.move_path_input && !app.help {
            if app.move_destination_focused {
                app.move_destination.push_str(pasted);
            } else {
                app.move_source.push_str(pasted);
            }
        }
        if let Some(path) = &mut app.remove_path_input
            && !app.help
        {
            path.push_str(pasted);
        }
        if let Some(path) = &mut app.ignore_path_input
            && !app.help
        {
            path.push_str(pasted);
        }
        if app.commit_message.is_some() && !app.amend_confirm && !app.mutation_pending && !app.help
        {
            let pasted = normalize_commit_paste(pasted, app.commit_description_focused);
            if app.commit_description_focused {
                app.commit_description.push_str(&pasted);
            } else if let Some(message) = &mut app.commit_message {
                message.push_str(&pasted);
            }
            app.commit_feedback.clear();
        }
        return Ok(Flow::Handled);
    }
    Ok(Flow::Next)
}
