pub mod run;
pub mod terminal;

use crossterm::event::KeyCode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    History,
    #[default]
    WorkingTree,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppAction {
    LoadWorkingTree,
    ReturnToHistory,
    StartSearch,
    StartPathHistory,
    OpenComparison,
    InspectObject,
    OpenBranches,
    StageSelected,
    StageAll,
    UnstageSelected,
    UnstageAll,
    StartCommit,
    StartAmend,
    PreviewDiscardSelected,
    PreviewUntrackSelected,
    InspectIgnoreRule,
    StartIgnorePathInput,
    StartRemovePathInput,
    StartMovePathInput,
    RefreshWorkingTree,
    MoveUp,
    MoveDown,
}

#[derive(Clone)]
pub enum BranchAction {
    Previous(crate::branches::PreviousCheckoutPreview),
    Switch(std::ffi::OsString),
    Create {
        name: String,
        base: String,
    },
    Rename {
        preview: crate::branches::BranchPreview,
        name: String,
    },
    Delete(crate::branches::BranchPreview),
}

#[derive(Debug, Default)]
pub struct AppState {
    view: View,
}

impl AppState {
    pub fn view(&self) -> View {
        self.view
    }

    /// Apply Tab when the active screen permits workspace navigation.
    pub fn handle_tab(&mut self, blocked: bool) -> Option<AppAction> {
        if blocked {
            return None;
        }

        match self.view {
            View::History => {
                self.view = View::WorkingTree;
                Some(AppAction::LoadWorkingTree)
            }
            View::WorkingTree => {
                self.view = View::History;
                Some(AppAction::ReturnToHistory)
            }
        }
    }

    /// Translate a key in the active workspace into an app action.
    pub fn handle_key(&mut self, key: KeyCode, blocked: bool) -> Option<AppAction> {
        if key == KeyCode::Tab {
            return self.handle_tab(blocked);
        }
        if blocked {
            return None;
        }
        if key == KeyCode::Char('W') {
            self.view = View::WorkingTree;
            return Some(AppAction::LoadWorkingTree);
        }
        if key == KeyCode::Char('H') {
            self.view = View::History;
            return Some(AppAction::ReturnToHistory);
        }

        match (self.view, key) {
            (_, KeyCode::Char('b')) => Some(AppAction::OpenBranches),
            (View::History, KeyCode::Char('o')) => Some(AppAction::InspectObject),
            (View::History, KeyCode::Char('D')) => Some(AppAction::OpenComparison),
            (View::History, KeyCode::Char('f')) => Some(AppAction::StartPathHistory),
            (View::History, KeyCode::Char('/')) => Some(AppAction::StartSearch),
            (View::History, KeyCode::Up) => Some(AppAction::MoveUp),
            (View::History, KeyCode::Down) => Some(AppAction::MoveDown),
            (View::WorkingTree, KeyCode::Char('s')) => Some(AppAction::StageSelected),
            (View::WorkingTree, KeyCode::Char('S')) => Some(AppAction::StageAll),
            (View::WorkingTree, KeyCode::Char('u')) => Some(AppAction::UnstageSelected),
            (View::WorkingTree, KeyCode::Char('U')) => Some(AppAction::UnstageAll),
            (View::WorkingTree, KeyCode::Char('r')) => Some(AppAction::RefreshWorkingTree),
            (View::WorkingTree, KeyCode::Char('c')) => Some(AppAction::StartCommit),
            (View::WorkingTree, KeyCode::Char('a')) => Some(AppAction::StartAmend),
            (View::WorkingTree, KeyCode::Char('d')) => Some(AppAction::PreviewDiscardSelected),
            (View::WorkingTree, KeyCode::Char('x')) => Some(AppAction::PreviewUntrackSelected),
            (View::WorkingTree, KeyCode::Char('i')) => Some(AppAction::InspectIgnoreRule),
            (View::WorkingTree, KeyCode::Char('I')) => Some(AppAction::StartIgnorePathInput),
            (View::WorkingTree, KeyCode::Char('R')) => Some(AppAction::StartRemovePathInput),
            (View::WorkingTree, KeyCode::Char('M')) => Some(AppAction::StartMovePathInput),
            (View::WorkingTree, KeyCode::Up) => Some(AppAction::MoveUp),
            (View::WorkingTree, KeyCode::Down) => Some(AppAction::MoveDown),
            _ => None,
        }
    }
}
pub fn move_down(selected: Option<usize>, len: usize) -> Option<usize> {
    (len > 0).then(|| (selected.unwrap_or(0) + 1).min(len - 1))
}

pub fn move_up(selected: Option<usize>) -> Option<usize> {
    selected.map(|index| index.saturating_sub(1))
}
