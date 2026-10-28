use crossterm::event::KeyCode;
use gitview::app::{AppAction, AppState, View, move_down, move_up};

#[test]
fn working_tree_is_the_default_and_tab_switches_to_history() {
    let mut app = AppState::default();

    assert_eq!(app.view(), View::WorkingTree);
    assert_eq!(app.handle_tab(false), Some(AppAction::ReturnToHistory));
    assert_eq!(app.view(), View::History);
    assert_eq!(app.handle_tab(false), Some(AppAction::LoadWorkingTree));
    assert_eq!(app.view(), View::WorkingTree);
}

#[test]
fn capital_h_focuses_history_from_the_working_tree() {
    let mut state = AppState::default();
    assert_eq!(
        state.handle_key(KeyCode::Char('H'), false),
        Some(AppAction::ReturnToHistory)
    );

    assert_eq!(state.view(), View::History);
}

#[test]
fn capital_w_opens_working_tree_and_capital_s_stages_everything() {
    let mut state = AppState::default();
    assert_eq!(
        state.handle_key(KeyCode::Char('W'), false),
        Some(AppAction::LoadWorkingTree)
    );
    assert_eq!(state.view(), View::WorkingTree);
    assert_eq!(
        state.handle_key(KeyCode::Char('S'), false),
        Some(AppAction::StageAll)
    );
    assert_eq!(
        state.handle_key(KeyCode::Char('x'), false),
        Some(AppAction::PreviewUntrackSelected)
    );
}

#[test]
fn tab_is_ignored_while_an_overlay_or_mutation_blocks_navigation() {
    let mut app = AppState::default();

    assert_eq!(app.handle_tab(true), None);
    assert_eq!(app.view(), View::WorkingTree);
}

#[test]
fn history_and_working_tree_keys_map_to_testable_actions() {
    let mut app = AppState::default();
    assert_eq!(app.handle_tab(false), Some(AppAction::ReturnToHistory));

    assert_eq!(
        app.handle_key(KeyCode::Char('/'), false),
        Some(AppAction::StartSearch)
    );
    assert_eq!(app.handle_key(KeyCode::Char('n'), false), None);
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
fn lowercase_c_starts_a_commit_only_from_the_working_tree() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('c'), false),
        Some(AppAction::StartCommit)
    );
    assert_eq!(app.handle_tab(false), Some(AppAction::ReturnToHistory));
    assert_eq!(app.handle_key(KeyCode::Char('c'), false), None);
}

#[test]
fn lowercase_a_starts_an_amend_only_from_the_working_tree() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('a'), false),
        Some(AppAction::StartAmend)
    );
    assert_eq!(app.handle_tab(false), Some(AppAction::ReturnToHistory));
    assert_eq!(app.handle_key(KeyCode::Char('a'), false), None);
}

#[test]
fn lowercase_d_previews_discard_only_from_the_working_tree() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('d'), false),
        Some(AppAction::PreviewDiscardSelected)
    );
    assert_eq!(app.handle_tab(false), Some(AppAction::ReturnToHistory));
    assert_eq!(app.handle_key(KeyCode::Char('d'), false), None);
}

#[test]
fn lowercase_x_previews_untracking_only_from_the_working_tree() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('x'), false),
        Some(AppAction::PreviewUntrackSelected)
    );
    app.handle_key(KeyCode::Char('H'), false);
    assert_eq!(app.handle_key(KeyCode::Char('x'), false), None);
}

#[test]
fn lowercase_i_inspects_ignore_rules_only_from_the_working_tree() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('i'), false),
        Some(AppAction::InspectIgnoreRule)
    );
    app.handle_key(KeyCode::Char('H'), false);
    assert_eq!(app.handle_key(KeyCode::Char('i'), false), None);
}

#[test]
fn list_movement_preserves_selection_and_clamps_to_available_rows() {
    assert_eq!(move_down(None, 4), Some(1));
    assert_eq!(move_down(Some(2), 3), Some(2));
    assert_eq!(move_down(Some(0), 0), None);
    assert_eq!(move_up(Some(0)), Some(0));
    assert_eq!(move_up(None), None);
}
