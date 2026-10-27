use crossterm::event::KeyCode;
use gitview::app::{AppAction, AppState, View, move_down, move_up};

#[test]
fn tab_switches_workspaces_and_requests_status_only_when_entering_working_tree() {
    let mut app = AppState::default();

    assert_eq!(app.view(), View::History);
    assert_eq!(app.handle_tab(false), Some(AppAction::LoadWorkingTree));
    assert_eq!(app.view(), View::WorkingTree);
    assert_eq!(app.handle_tab(false), Some(AppAction::ReturnToHistory));
    assert_eq!(app.view(), View::History);
}

#[test]
fn tab_is_ignored_while_an_overlay_or_mutation_blocks_navigation() {
    let mut app = AppState::default();

    assert_eq!(app.handle_tab(true), None);
    assert_eq!(app.view(), View::History);
}

#[test]
fn history_and_working_tree_keys_map_to_testable_actions() {
    let mut app = AppState::default();

    assert_eq!(
        app.handle_key(KeyCode::Char('/'), false),
        Some(AppAction::StartSearch)
    );
    assert_eq!(
        app.handle_key(KeyCode::Char('n'), false),
        Some(AppAction::NextMatch)
    );
    assert_eq!(
        app.handle_key(KeyCode::Char('b'), false),
        Some(AppAction::OpenBranches)
    );
    assert_eq!(app.handle_key(KeyCode::Char('s'), false), None);

    assert_eq!(app.handle_tab(false), Some(AppAction::LoadWorkingTree));
    assert_eq!(
        app.handle_key(KeyCode::Char('s'), false),
        Some(AppAction::StageSelected)
    );
    assert_eq!(
        app.handle_key(KeyCode::Char('u'), false),
        Some(AppAction::UnstageSelected)
    );
    assert_eq!(
        app.handle_key(KeyCode::Char('r'), false),
        Some(AppAction::RefreshWorkingTree)
    );
    assert_eq!(app.handle_key(KeyCode::Char('/'), false), None);
}

#[test]
fn list_movement_preserves_selection_and_clamps_to_available_rows() {
    assert_eq!(move_down(None, 4), Some(1));
    assert_eq!(move_down(Some(2), 3), Some(2));
    assert_eq!(move_down(Some(0), 0), None);
    assert_eq!(move_up(Some(0)), Some(0));
    assert_eq!(move_up(None), None);
}
