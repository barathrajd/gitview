mod agent;
mod control;
mod drain;
mod draw;
mod helpers;
mod input;
mod keys;
mod model;
mod paste;
mod refresh;
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
    OpenSessions,
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
    /// A counted `k`/Up, such as `5k`.
    MoveUpBy(usize),
    /// A counted `j`/Down, such as `5j`.
    MoveDownBy(usize),
    /// `gg`.
    MoveTop,
    /// `G`.
    MoveBottom,
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
    /// Digits typed before a motion, such as the `12` of `12j`.
    count: Option<usize>,
    /// The first `g` of `gg`.
    pending_g: bool,
}

impl AppState {
    pub fn view(&self) -> View {
        self.view
    }

    /// Show `view`, as the agent can ask through the control channel.
    pub fn show(&mut self, view: View) {
        self.view = view;
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
        // Pending motion input applies only to the key right after it.
        let count = self.count.take();
        let pending_g = std::mem::take(&mut self.pending_g);
        if key == KeyCode::Tab {
            return self.handle_tab(blocked);
        }
        if blocked {
            return None;
        }
        match key {
            KeyCode::Char('g') if pending_g => return Some(AppAction::MoveTop),
            KeyCode::Char('g') => {
                self.pending_g = true;
                return None;
            }
            KeyCode::Char(digit @ '0'..='9') if digit != '0' || count.is_some() => {
                let digit = digit as usize - '0' as usize;
                self.count = Some(count.unwrap_or(0).saturating_mul(10).saturating_add(digit));
                return None;
            }
            KeyCode::Char('G') => return Some(AppAction::MoveBottom),
            KeyCode::Up if count.is_some() => return count.map(AppAction::MoveUpBy),
            KeyCode::Down if count.is_some() => return count.map(AppAction::MoveDownBy),
            _ => {}
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
            (_, KeyCode::Char('A')) => Some(AppAction::OpenSessions),
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
    move_up_by(selected, 1)
}

/// Move `count` rows down, stopping at the last of `len` rows.
pub fn move_down_by(selected: Option<usize>, len: usize, count: usize) -> Option<usize> {
    (len > 0).then(|| selected.unwrap_or(0).saturating_add(count).min(len - 1))
}

/// Move `count` rows up, stopping at the first row.
pub fn move_up_by(selected: Option<usize>, count: usize) -> Option<usize> {
    selected.map(|index| index.saturating_sub(count))
}
