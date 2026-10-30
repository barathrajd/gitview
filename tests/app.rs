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
fn uppercase_i_starts_an_arbitrary_ignore_path_prompt() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('I'), false),
        Some(AppAction::StartIgnorePathInput)
    );
    app.handle_key(KeyCode::Char('H'), false);
    assert_eq!(app.handle_key(KeyCode::Char('I'), false), None);
}

#[test]
fn uppercase_r_starts_a_tracked_path_removal_prompt() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('R'), false),
        Some(AppAction::StartRemovePathInput)
    );
}

#[test]
fn uppercase_m_starts_a_tracked_path_move_prompt() {
    let mut app = AppState::default();
    assert_eq!(
        app.handle_key(KeyCode::Char('M'), false),
        Some(AppAction::StartMovePathInput)
    );
}

#[test]
fn list_movement_preserves_selection_and_clamps_to_available_rows() {
    assert_eq!(move_down(None, 4), Some(1));
    assert_eq!(move_down(Some(2), 3), Some(2));
    assert_eq!(move_down(Some(0), 0), None);
    assert_eq!(move_up(Some(0)), Some(0));
    assert_eq!(move_up(None), None);
}

#[test]
fn gg_and_capital_g_jump_to_the_first_and_last_rows() {
    let mut app = AppState::default();

    assert_eq!(
        app.handle_key(KeyCode::Char('G'), false),
        Some(AppAction::MoveBottom)
    );
    assert_eq!(app.handle_key(KeyCode::Char('g'), false), None);
    assert_eq!(
        app.handle_key(KeyCode::Char('g'), false),
        Some(AppAction::MoveTop)
    );
}

#[test]
fn a_count_repeats_the_next_vertical_motion() {
    let mut app = AppState::default();

    assert_eq!(app.handle_key(KeyCode::Char('1'), false), None);
    assert_eq!(app.handle_key(KeyCode::Char('2'), false), None);
    assert_eq!(
        app.handle_key(KeyCode::Down, false),
        Some(AppAction::MoveDownBy(12))
    );
    assert_eq!(app.handle_key(KeyCode::Char('3'), false), None);
    assert_eq!(
        app.handle_key(KeyCode::Up, false),
        Some(AppAction::MoveUpBy(3))
    );
    // The count is spent: a plain motion moves one row again.
    assert_eq!(
        app.handle_key(KeyCode::Down, false),
        Some(AppAction::MoveDown)
    );
}

#[test]
fn a_pending_count_or_g_is_dropped_by_other_keys() {
    let mut app = AppState::default();

    assert_eq!(app.handle_key(KeyCode::Char('4'), false), None);
    // Other keys keep their action and discard the count.
    assert_eq!(
        app.handle_key(KeyCode::Char('s'), false),
        Some(AppAction::StageSelected)
    );
    assert_eq!(
        app.handle_key(KeyCode::Down, false),
        Some(AppAction::MoveDown)
    );

    assert_eq!(app.handle_key(KeyCode::Char('g'), false), None);
    assert_eq!(
        app.handle_key(KeyCode::Char('s'), false),
        Some(AppAction::StageSelected)
    );
    assert_eq!(app.handle_key(KeyCode::Char('g'), false), None);
    assert_eq!(app.handle_key(KeyCode::Esc, false), None);
    assert_eq!(app.handle_key(KeyCode::Char('g'), false), None);

    // A blocked screen also clears pending motion input.
    assert_eq!(app.handle_key(KeyCode::Char('5'), false), None);
    assert_eq!(app.handle_key(KeyCode::Down, true), None);
    assert_eq!(
        app.handle_key(KeyCode::Down, false),
        Some(AppAction::MoveDown)
    );
}

#[test]
fn counted_moves_clamp_to_the_list() {
    assert_eq!(gitview::app::move_down_by(Some(1), 3, 10), Some(2));
    assert_eq!(gitview::app::move_down_by(None, 0, 10), None);
    assert_eq!(gitview::app::move_up_by(Some(2), 10), Some(0));
    assert_eq!(gitview::app::move_up_by(None, 10), None);
}

#[test]
fn control_page_keys_map_to_half_and_full_page_motions() {
    let mut app = AppState::default();

    assert_eq!(
        app.handle_control('d', false),
        Some(AppAction::HalfPageDown)
    );
    assert_eq!(app.handle_control('u', false), Some(AppAction::HalfPageUp));
    assert_eq!(app.handle_control('f', false), Some(AppAction::PageDown));
    assert_eq!(app.handle_control('b', false), Some(AppAction::PageUp));
    assert_eq!(app.handle_control('s', false), None);
    assert_eq!(app.handle_control('d', true), None);
    assert_eq!(
        app.handle_key(KeyCode::PageDown, false),
        Some(AppAction::PageDown)
    );
    assert_eq!(
        app.handle_key(KeyCode::PageUp, false),
        Some(AppAction::PageUp)
    );
}

#[test]
fn a_page_key_drops_a_pending_count() {
    let mut app = AppState::default();

    assert_eq!(app.handle_key(KeyCode::Char('3'), false), None);
    assert_eq!(
        app.handle_control('d', false),
        Some(AppAction::HalfPageDown)
    );
    assert_eq!(
        app.handle_key(KeyCode::Down, false),
        Some(AppAction::MoveDown)
    );
}

#[test]
fn zz_centers_the_selection_and_a_lone_z_keeps_other_keys() {
    let mut app = AppState::default();

    assert_eq!(app.handle_key(KeyCode::Char('z'), false), None);
    assert_eq!(
        app.handle_key(KeyCode::Char('z'), false),
        Some(AppAction::CenterSelection)
    );
    assert_eq!(app.handle_key(KeyCode::Char('z'), false), None);
    assert_eq!(
        app.handle_key(KeyCode::Char('s'), false),
        Some(AppAction::StageSelected)
    );
}
