mod agent_pane;
mod branches;
mod cockpit;
mod commit;
mod diff;
mod diff_layout;
mod diff_rows;
mod help;
mod hints;
mod panels;
mod prepared;
mod render;
mod screen;
mod theme;
mod viewport;

pub use agent_pane::{
    AgentLayout, agent_pane_inner, plan_agent_layout, plan_workspace_agent_layout,
    render_agent_bar, render_agent_pane,
};
pub use diff_layout::diff_hunk_line_positions;
pub use help::{HELP, render_help_overlay, render_help_overlay_in};
pub(crate) use hints::{hint_row_count, render_key_hints};
pub use prepared::PreparedDiff;
pub(crate) use prepared::{
    PreparedComparisonFile, comparison_file_lines, prepare_comparison_file, retained_line_bytes,
};
pub use render::{render, render_in};
pub use screen::{Cockpit, HelpPage, Screen};
pub use theme::ASCII_BORDER;
pub(crate) use theme::{ACCENT, MUTED, page_block, selection_style};
pub use viewport::{Viewport, split_side_pane};

pub use cockpit::render_workspace_cockpit;
