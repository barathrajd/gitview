//! Workspace panel visibility, focus, and terminal geometry.
use ratatui::layout::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Sidebar,
    Center,
    Agent,
}

#[derive(Clone, Debug)]
pub struct Workspace {
    focused: Panel,
    fullscreen: bool,
    sidebar: bool,
    center: bool,
    agent: bool,
    has_agent: bool,
    sidebar_width: u16,
    agent_width: u16,
}

#[derive(Debug)]
pub struct WorkspaceLayout {
    pub sidebar: Option<Rect>,
    pub center: Option<Rect>,
    pub agent: Option<Rect>,
    pub bar: Rect,
}

impl Workspace {
    pub fn new(has_agent: bool) -> Self {
        Self {
            focused: Panel::Center,
            fullscreen: false,
            sidebar: true,
            center: true,
            agent: has_agent,
            has_agent,
            sidebar_width: 40,
            agent_width: 70,
        }
    }

    /// The visible agent's requested width, before terminal bounds are applied.
    pub fn agent_width(&self) -> Option<u16> {
        self.agent.then_some(self.agent_width)
    }

    /// Lay out Gitview within the area left after the outer agent split.
    pub fn gitview_layout(&self, area: Rect) -> WorkspaceLayout {
        let mut gitview = self.clone();
        gitview.agent = false;
        if gitview.focused == Panel::Agent {
            gitview.focused = if gitview.center {
                Panel::Center
            } else {
                Panel::Sidebar
            };
        }
        gitview.layout(area)
    }

    pub fn restore_panels(&mut self) {
        self.sidebar = true;
        self.center = true;
        self.agent = self.has_agent;
        self.fullscreen = false;
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    pub fn focused(&self) -> Panel {
        self.focused
    }

    fn visible(&self, panel: Panel) -> bool {
        match panel {
            Panel::Sidebar => self.sidebar,
            Panel::Center => self.center,
            Panel::Agent => self.agent,
        }
    }

    pub fn focus(&mut self, panel: Panel) -> bool {
        if !self.visible(panel) {
            return false;
        }
        self.focused = panel;
        true
    }

    /// Keep at least one panel visible and move focus when its panel hides.
    pub fn hide(&mut self, panel: Panel) -> bool {
        if [Panel::Sidebar, Panel::Center, Panel::Agent]
            .into_iter()
            .filter(|p| self.visible(*p))
            .count()
            == 1
            && self.visible(panel)
        {
            return false;
        }
        self.fullscreen = false;
        match panel {
            Panel::Center => self.center = false,
            Panel::Sidebar => self.sidebar = false,
            Panel::Agent => self.agent = false,
        }
        if self.focused == panel {
            self.focused = [Panel::Center, Panel::Sidebar, Panel::Agent]
                .into_iter()
                .find(|p| self.visible(*p))
                .unwrap();
        }
        true
    }

    pub fn show(&mut self, panel: Panel) {
        match panel {
            Panel::Sidebar => self.sidebar = true,
            Panel::Agent => self.agent = self.has_agent,
            Panel::Center => self.center = true,
        }
    }

    /// Adjust the requested width; layout clamps it to the current terminal.
    pub fn resize(&mut self, panel: Panel, columns: i16) {
        let (width, delta) = match panel {
            Panel::Sidebar => (&mut self.sidebar_width, i32::from(columns)),
            Panel::Agent => (&mut self.agent_width, i32::from(columns)),
            Panel::Center if self.sidebar => (&mut self.sidebar_width, -i32::from(columns)),
            Panel::Center if self.agent => (&mut self.agent_width, -i32::from(columns)),
            Panel::Center => return,
        };
        *width = (i32::from(*width) + delta).clamp(12, 200) as u16;
    }

    pub fn layout(&self, area: Rect) -> WorkspaceLayout {
        let bar_height = area.height.min(1);
        let content = Rect {
            height: area.height - bar_height,
            ..area
        };
        let mut layout = WorkspaceLayout {
            sidebar: None,
            center: None,
            agent: None,
            bar: Rect::new(area.x, content.bottom(), area.width, bar_height),
        };
        if area.width < 119 || self.fullscreen {
            match self.focused {
                Panel::Sidebar => layout.sidebar = Some(content),
                Panel::Center => layout.center = Some(content),
                Panel::Agent => layout.agent = Some(content),
            }
            return layout;
        }
        let budget = area.width.saturating_sub(if self.center { 40 } else { 0 });
        let sidebar = if self.sidebar {
            if !self.center && !self.agent {
                area.width
            } else {
                self.sidebar_width.min(budget / 2)
            }
        } else {
            0
        };
        let agent = if self.agent {
            if !self.center {
                area.width - sidebar
            } else {
                self.agent_width.min(budget - sidebar)
            }
        } else {
            0
        };
        let center = area.width - sidebar - agent;
        if self.sidebar {
            layout.sidebar = Some(Rect::new(area.x, area.y, sidebar, content.height));
        }
        if self.center {
            layout.center = Some(Rect::new(area.x + sidebar, area.y, center, content.height));
        }
        if self.agent {
            layout.agent = Some(Rect::new(
                area.x + sidebar + center,
                area.y,
                agent,
                content.height,
            ));
        }
        layout
    }
}
