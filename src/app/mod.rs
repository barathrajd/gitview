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

use crate::motion::{Motion, Parsed};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    History,
    #[default]
    WorkingTree,
    /// The sidebar's Branches section.
    Branches,
    /// The sidebar's Sessions section.
    Sessions,
}

impl View {
    /// The workspace panel that holds this view.
    pub fn panel(self) -> crate::workspace::Panel {
        match self {
            View::History => crate::workspace::Panel::Center,
            View::WorkingTree | View::Branches | View::Sessions => crate::workspace::Panel::Sidebar,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppAction {
    LoadWorkingTree,
    ReturnToHistory,
    StartSearch,
    /// `n`: the next commit matching the last search, below the selection.
    SearchNext,
    /// `N`: the previous matching commit, above the selection.
    SearchPrevious,
    StartPathHistory,
    /// `v`: switch History between the structured graph and raw rows.
    ToggleRawHistory,
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
    /// Ctrl-d.
    HalfPageDown,
    /// Ctrl-u.
    HalfPageUp,
    /// Ctrl-f or Page Down.
    PageDown,
    /// Ctrl-b or Page Up.
    PageUp,
    /// `zz`.
    CenterSelection,
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
    motions: crate::motion::MotionParser,
}

impl AppState {
    pub fn view(&self) -> View {
        self.view
    }

    /// Show `view`, as the agent can ask through the control channel.
    pub fn show(&mut self, view: View) {
        self.view = view;
    }

    /// Apply Tab when the active screen permits workspace navigation:
    /// Files, Branches, Sessions, History, and back to Files.
    pub fn handle_tab(&mut self, blocked: bool) -> Option<AppAction> {
        self.cycle(blocked, true)
    }

    /// Apply Shift-Tab: the Tab cycle in reverse.
    pub fn handle_back_tab(&mut self, blocked: bool) -> Option<AppAction> {
        self.cycle(blocked, false)
    }

    fn cycle(&mut self, blocked: bool, forward: bool) -> Option<AppAction> {
        if blocked {
            return None;
        }
        // Files, Branches, Sessions, History, and back to Files.
        const ORDER: [View; 4] = [
            View::WorkingTree,
            View::Branches,
            View::Sessions,
            View::History,
        ];
        let index = ORDER
            .iter()
            .position(|&view| view == self.view)
            .unwrap_or(0);
        let next = if forward {
            index + 1
        } else {
            index + ORDER.len() - 1
        };
        self.view = ORDER[next % ORDER.len()];
        Some(match self.view {
            View::WorkingTree => AppAction::LoadWorkingTree,
            View::History => AppAction::ReturnToHistory,
            View::Branches => AppAction::OpenBranches,
            View::Sessions => AppAction::OpenSessions,
        })
    }

    /// Translate a Ctrl-modified letter into a page motion. Other Ctrl
    /// letters do nothing, so they never run a plain-key shortcut.
    pub fn handle_control(&mut self, letter: char, blocked: bool) -> Option<AppAction> {
        let key = KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL);
        let parsed = self.motions.key(key);
        if blocked {
            return None;
        }
        match parsed {
            Parsed::Motion(motion) => Some(motion_action(motion)),
            Parsed::Pending | Parsed::Other => None,
        }
    }

    /// Translate a key in the active workspace into an app action.
    pub fn handle_key(&mut self, key: KeyCode, blocked: bool) -> Option<AppAction> {
        if key == KeyCode::Tab {
            self.motions.reset();
            return self.handle_tab(blocked);
        }
        if key == KeyCode::BackTab {
            self.motions.reset();
            return self.handle_back_tab(blocked);
        }
        if blocked {
            self.motions.reset();
            return None;
        }
        match self.motions.key(KeyEvent::new(key, KeyModifiers::NONE)) {
            Parsed::Pending => return None,
            // Plain arrows keep their own actions, matched below.
            Parsed::Motion(Motion::Up(1) | Motion::Down(1)) | Parsed::Other => {}
            Parsed::Motion(motion) => return Some(motion_action(motion)),
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
            (_, KeyCode::Char('b')) => {
                self.view = View::Branches;
                Some(AppAction::OpenBranches)
            }
            (_, KeyCode::Char('A')) => {
                self.view = View::Sessions;
                Some(AppAction::OpenSessions)
            }
            (View::History, KeyCode::Char('o')) => Some(AppAction::InspectObject),
            (View::History, KeyCode::Char('D')) => Some(AppAction::OpenComparison),
            (View::History, KeyCode::Char('f')) => Some(AppAction::StartPathHistory),
            (View::History, KeyCode::Char('v')) => Some(AppAction::ToggleRawHistory),
            (_, KeyCode::Char('/')) => Some(AppAction::StartSearch),
            (_, KeyCode::Char('n')) => Some(AppAction::SearchNext),
            (_, KeyCode::Char('N')) => Some(AppAction::SearchPrevious),
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
fn motion_action(motion: Motion) -> AppAction {
    match motion {
        Motion::Up(1) => AppAction::MoveUp,
        Motion::Down(1) => AppAction::MoveDown,
        Motion::Up(count) => AppAction::MoveUpBy(count),
        Motion::Down(count) => AppAction::MoveDownBy(count),
        Motion::Top => AppAction::MoveTop,
        Motion::Bottom => AppAction::MoveBottom,
        Motion::HalfPageUp => AppAction::HalfPageUp,
        Motion::HalfPageDown => AppAction::HalfPageDown,
        Motion::PageUp => AppAction::PageUp,
        Motion::PageDown => AppAction::PageDown,
        Motion::Center => AppAction::CenterSelection,
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
