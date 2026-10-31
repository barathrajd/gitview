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
    if let Some(search) = &mut app.inline_search {
        let rows = app.history.as_ref().and_then(|rows| rows.as_ref().ok());
        match key.code {
            KeyCode::Esc => {
                app.selection.select(search.origin);
                app.inline_search = None;
            }
            KeyCode::Enter => {
                app.last_search.clone_from(&app.query);
                app.inline_search = None;
            }
            // The full page ranks the same query.
            KeyCode::Tab => {
                app.selection.select(search.origin);
                app.inline_search = None;
                open_search_page(app);
            }
            KeyCode::Backspace | KeyCode::Char(_) if plain_code(key) != KeyCode::Null => {
                if key.code == KeyCode::Backspace {
                    app.query.pop();
                } else if let KeyCode::Char(c) = key.code {
                    app.query.push(c);
                }
                search.matches = rows
                    .filter(|_| !app.query.trim().is_empty())
                    .map(|rows| crate::history::matches_in_order(rows, &app.query))
                    .unwrap_or_default();
                // The first match at or below the start, wrapping around.
                let start = search.origin.unwrap_or(0);
                let target = search
                    .matches
                    .iter()
                    .find(|&&row| row >= start)
                    .or_else(|| search.matches.first())
                    .copied()
                    .or(search.origin);
                app.selection.select(target);
            }
            _ => {}
        }
        return Ok(Flow::Handled);
    }
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
                app.last_search.clone_from(&app.query);
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

/// Open the full results page, ranked for the current query.
fn open_search_page(app: &mut App) {
    app.searching = true;
    update_search_results(
        app.history.as_ref(),
        &app.query,
        &mut app.search_results,
        &mut app.search_selection,
    );
    app.search_debounce = app.history.is_none().then_some(Instant::now());
}
