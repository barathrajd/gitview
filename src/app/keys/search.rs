use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{
    helpers::*,
    model::App,
    run::{Env, Flow, Host},
};

pub(in crate::app) fn handle<H: Host>(
    app: &mut App,
    env: &mut Env<'_, H>,
    key: KeyEvent,
) -> io::Result<Flow> {
    let Env { root, .. } = env;
    if app.searching {
        // Typed characters edit the query, so only page keys and Ctrl
        // letters move the results; other Ctrl or Alt letters are ignored.
        if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
            || plain_code(key) == KeyCode::Null
        {
            let page = screen_page_rows(app.gitview_area, SEARCH_CHROME);
            list_motion(
                &mut app.page_motions,
                key,
                &mut app.search_selection,
                app.search_results.len(),
                page,
            );
            return Ok(Flow::Handled);
        }
        match key.code {
            KeyCode::Esc => {
                app.searching = false;
                app.search_debounce = None;
            }
            KeyCode::Char('?') => app.help = true,
            KeyCode::Backspace => {
                if app.query.pop().is_some() {
                    app.search_debounce = Some(Instant::now() + Duration::from_millis(180));
                }
            }
            KeyCode::Enter => {
                if app.search_debounce.is_some() {
                    update_search_results(
                        app.history.as_ref(),
                        &app.query,
                        &mut app.search_results,
                        &mut app.search_selection,
                    );
                }
                app.search_debounce = None;
                if let Some((index, id)) = selected_search_commit(
                    app.history.as_ref(),
                    &app.search_results,
                    app.search_selection.selected(),
                ) {
                    app.selection.select(Some(index));
                    app.request_id += 1;
                    let path = root.to_path_buf();
                    let sender = app.details_sender.clone();
                    let current = app.request_id;
                    app.file_selection.select(None);
                    app.details = Some((current, None));
                    std::thread::spawn(move || {
                        let _ = sender.send((current, crate::details::load_commit(&path, &id)));
                    });
                    app.searching = false;
                }
            }
            KeyCode::Up => {
                app.search_selection
                    .select(crate::app::move_up(app.search_selection.selected()));
            }
            KeyCode::Down => {
                app.search_selection.select(crate::app::move_down(
                    app.search_selection.selected(),
                    app.search_results.len(),
                ));
            }
            KeyCode::Char(c) => {
                app.query.push(c);
                app.search_debounce = Some(Instant::now() + Duration::from_millis(180));
            }
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    if key.code == KeyCode::Char('W')
        && !app.help
        && !app.mutation_pending
        && (!app.branch_view || (app.branch_input.is_none() && app.branch_confirm.is_none()))
    {
        let _ = app.app_state.handle_key(KeyCode::Char('W'), false);
        app.branch_view = false;
        app.diff = None;
        app.prepared_diff = None;
        app.ordinary_diff_worker.cancel();
        app.diff_entry = None;
        app.details = None;
        return Ok(Flow::Handled);
    }
    Ok(Flow::Next)
}
