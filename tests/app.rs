use gitview::app::{AppAction, AppState, View};

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
