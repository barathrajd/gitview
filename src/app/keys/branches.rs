use std::io;

use crate::app::{
    BranchAction,
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
    if app.branch_view && !app.help {
        if app.mutation_pending {
            return Ok(Flow::Handled);
        }
        if let Some(input) = &mut app.branch_input {
            match key.code {
                KeyCode::Char('?') => app.help = true,
                KeyCode::Esc => {
                    app.branch_input = None;
                    app.branch_rename = None;
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(ch) if !ch.is_control() => input.push(ch),
                KeyCode::Enter => {
                    let name = input.clone();
                    app.branch_input = None;
                    if let Some(preview) = app.branch_rename.take() {
                        app.branch_confirm = Some(BranchAction::Rename { preview, name });
                        return Ok(Flow::Handled);
                    }
                    if let Some(Ok(data)) = &app.branches {
                        if let Some(base) = &data.head_commit {
                            if name.is_empty() {
                                app.branch_feedback = "Enter a branch name.".into();
                            } else {
                                app.branch_confirm = Some(BranchAction::Create {
                                    name,
                                    base: base.clone(),
                                });
                            }
                        } else {
                            app.branch_feedback =
                                "Create the first commit before creating a branch.".into();
                        }
                    }
                }
                _ => {}
            }
            return Ok(Flow::Handled);
        }
        if let Some(action) = &app.branch_confirm {
            match key.code {
                KeyCode::Char('?') => app.help = true,
                KeyCode::Char('y') => {
                    app.mutation_pending = true;
                    let action = action.clone();
                    let path = root.to_path_buf();
                    let sender = app.operation_sender.clone();
                    std::thread::spawn(move || {
                        let (success_message, result) = match action {
                            BranchAction::Previous(preview) => (
                                "Returned to previous checkout.".to_owned(),
                                crate::writes::serialized(|| {
                                    crate::branches::switch_previous_checkout(&path, &preview)
                                }),
                            ),
                            BranchAction::Switch(name) => (
                                "Branch switched successfully.".to_owned(),
                                crate::writes::serialized(|| {
                                    crate::branches::switch_branch(&path, &name)
                                }),
                            ),
                            BranchAction::Rename { preview, name } => (
                                "Branch renamed successfully.".to_owned(),
                                crate::writes::serialized(|| {
                                    crate::branches::rename_branch(&path, &preview, &name)
                                }),
                            ),
                            BranchAction::Delete(preview) => (
                                "Branch deleted successfully.".to_owned(),
                                crate::writes::serialized(|| {
                                    crate::branches::delete_branch(&path, &preview)
                                }),
                            ),
                            BranchAction::Create { name, base } => (
                                "Branch created and switched successfully.".to_owned(),
                                crate::writes::serialized(|| {
                                    crate::branches::create_branch(&path, &name, &base)
                                }),
                            ),
                        };
                        let _ = sender.send((success_message, result));
                    });
                }
                KeyCode::Char('n') | KeyCode::Esc => app.branch_confirm = None,
                _ => {}
            }
            return Ok(Flow::Handled);
        }
        match navigation_key(key) {
            KeyCode::Char('t') => {
                app.tag_page = Some(crate::tag_page::TagPage::open(root));
            }
            KeyCode::Char('-') => {
                app.pending_preview = Some(crate::preview::Context::Previous);
                app.branch_feedback.clear();
                app.preview_worker
                    .request(crate::preview::Request::Previous);
            }
            KeyCode::Char('r' | 'd') => {
                if let (Some(Ok(data)), Some(index)) =
                    (&app.branches, app.branch_selection.selected())
                    && let Some(name) = data.branches.get(index)
                {
                    app.pending_preview = Some(if key.code == KeyCode::Char('r') {
                        crate::preview::Context::BranchRename
                    } else {
                        crate::preview::Context::BranchDelete
                    });
                    app.branch_feedback.clear();
                    app.preview_worker
                        .request(crate::preview::Request::Branch(name.clone()));
                }
            }
            KeyCode::Char('c') => {
                if let Some(Ok(data)) = &app.branches {
                    if data.head_commit.is_some() {
                        app.branch_input = Some(String::new());
                        app.branch_feedback.clear();
                    } else {
                        app.branch_feedback =
                            "Create the first commit before creating a branch.".into();
                    }
                }
            }
            KeyCode::Enter => {
                if let (Some(Ok(data)), Some(index)) =
                    (&app.branches, app.branch_selection.selected())
                    && let Some(name) = data.branches.get(index)
                {
                    if matches!(&data.head, crate::branches::HeadState::Attached(current) if current == name)
                    {
                        app.branch_feedback = "Already on this branch.".into();
                    } else {
                        app.branch_confirm = Some(BranchAction::Switch(name.clone()));
                        app.branch_feedback.clear();
                    }
                }
            }
            KeyCode::Char('q') => return Ok(Flow::Quit),
            KeyCode::Char('?') => app.help = true,
            KeyCode::Esc | KeyCode::Char('b') => app.branch_view = false,
            KeyCode::Char('H') => {
                app.branch_view = false;
                let _ = app.app_state.handle_key(KeyCode::Char('H'), false);
            }
            KeyCode::Down => {
                if let Some(Ok(data)) = &app.branches
                    && !data.branches.is_empty()
                {
                    app.branch_selection.select(Some(
                        (app.branch_selection.selected().unwrap_or(0) + 1)
                            .min(data.branches.len() - 1),
                    ));
                }
            }
            KeyCode::Up => {
                if let Some(index) = app.branch_selection.selected() {
                    app.branch_selection.select(Some(index.saturating_sub(1)));
                }
            }
            _ => {}
        }
        return Ok(Flow::Handled);
    }
    Ok(Flow::Next)
}
