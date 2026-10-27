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
}
