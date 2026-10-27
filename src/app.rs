use crossterm::event::KeyCode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    History,
    WorkingTree,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppAction {
    LoadWorkingTree,
    ReturnToHistory,
    StartSearch,
    NextMatch,
    OpenBranches,
    StageSelected,
    UnstageSelected,
    RefreshWorkingTree,
    MoveUp,
    MoveDown,
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

        match (self.view, key) {
            (_, KeyCode::Char('b')) => Some(AppAction::OpenBranches),
            (View::History, KeyCode::Char('/')) => Some(AppAction::StartSearch),
            (View::History, KeyCode::Char('n')) => Some(AppAction::NextMatch),
            (View::History, KeyCode::Up) => Some(AppAction::MoveUp),
            (View::History, KeyCode::Down) => Some(AppAction::MoveDown),
            (View::WorkingTree, KeyCode::Char('s')) => Some(AppAction::StageSelected),
            (View::WorkingTree, KeyCode::Char('u')) => Some(AppAction::UnstageSelected),
            (View::WorkingTree, KeyCode::Char('r')) => Some(AppAction::RefreshWorkingTree),
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
