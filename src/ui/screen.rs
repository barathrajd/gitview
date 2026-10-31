use crate::{
    app::{BranchAction, View},
    branches::Branches,
    config::FolderProfile,
    details::CommitDetails,
    worktree::Entry,
};
use ratatui::widgets::ListState;
use std::path::Path;

use super::prepared::PreparedDiff;

pub struct Cockpit<'a> {
    pub repository: &'a Path,
    pub profile: &'a str,
    pub branches: Option<&'a Result<Branches, String>>,
    pub history: Option<&'a Result<Vec<String>, String>>,
    pub status: Option<&'a Result<Vec<Entry>, String>>,
    pub feedback: &'a str,
    /// The inline `/` prompt, drawn on the status line in place of the counts.
    pub search_prompt: Option<String>,
    pub focused: View,
    pub history_selection: &'a mut ListState,
    pub working_tree_selection: &'a mut ListState,
}

pub enum Screen<'a> {
    PreparedDiff {
        diff: &'a (u64, Option<Result<String, String>>),
        prepared: &'a PreparedDiff,
        scroll: u16,
        selected_hunk: usize,
        raw: bool,
        side_by_side: bool,
        worktree_entry: Option<&'a Entry>,
    },
    PreviewLoading {
        message: &'static str,
    },
    CommitPreviewLoading {
        amend: bool,
    },
    Object(&'a crate::object_page::ObjectPage),
    Comparison(&'a crate::comparison_page::ComparisonPage),
    PathHistoryInput {
        path: &'a str,
    },
    Tags(&'a crate::tag_page::TagPage),
    Sessions(&'a crate::sessions_page::SessionsPage),
    Cockpit(Cockpit<'a>),
    Help,
    BranchInput(&'a str),
    RenameBranchInput {
        input: &'a str,
        name: &'a std::ffi::OsStr,
    },
    IgnorePathInput {
        path: &'a str,
        feedback: &'a str,
    },
    RemovePathInput {
        path: &'a str,
        feedback: &'a str,
    },
    RemovePathConfirmation {
        path: &'a std::ffi::OsStr,
        pending: bool,
    },
    MovePathInput {
        source: &'a str,
        destination: &'a str,
        destination_focused: bool,
        feedback: &'a str,
    },
    MovePathConfirmation {
        source: &'a std::ffi::OsStr,
        destination: &'a std::ffi::OsStr,
        pending: bool,
    },
    BranchConfirmation {
        pending: bool,
        action: Option<&'a BranchAction>,
    },
    CommitInput {
        repository: &'a Path,
        profile: &'a str,
        branches: Option<&'a Result<Branches, String>>,
        message: &'a str,
        description: &'a str,
        description_focused: bool,
        amend: bool,
        feedback: &'a str,
        entries: Option<&'a Result<Vec<Entry>, String>>,
        pending: bool,
    },
    AmendConfirmation {
        repository: &'a Path,
        profile: &'a str,
        branches: Option<&'a Result<Branches, String>>,
        entries: Option<&'a Result<Vec<Entry>, String>>,
        head: &'a str,
        original_message: &'a str,
        new_message: &'a str,
        pending: bool,
    },
    DiscardConfirmation {
        entry: &'a Entry,
        pending: bool,
    },
    UntrackConfirmation {
        entry: &'a Entry,
        pending: bool,
    },
    Branches {
        branches: Option<&'a Result<Branches, String>>,
        feedback: &'a str,
        selection: &'a mut ListState,
    },
    Diff {
        diff: &'a (u64, Option<Result<String, String>>),
        scroll: u16,
        selected_hunk: usize,
        raw: bool,
        side_by_side: bool,
        worktree_entry: Option<&'a Entry>,
    },
    CommitDetails {
        details: &'a (u64, Option<Result<CommitDetails, String>>),
        selection: &'a mut ListState,
    },
    WorkingTree {
        status: Option<&'a Result<Vec<Entry>, String>>,
        selection: &'a mut ListState,
    },
    History {
        history: Option<&'a Result<Vec<String>, String>>,
        searching: bool,
        query: &'a str,
        search_message: &'a str,
        root: &'a Path,
        selection: &'a mut ListState,
    },
    Search {
        history: Option<&'a Result<Vec<String>, String>>,
        matches: &'a [usize],
        query: &'a str,
        pending: bool,
        selection: &'a mut ListState,
    },
    Profiles {
        profiles: &'a [FolderProfile],
        selected: Option<usize>,
        active_profile: &'a str,
        registry_path: &'a Path,
        feedback: &'a str,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpPage {
    History,
    WorkingTree,
    Branches,
    BranchInput,
    BranchConfirmation,
    Commit,
    CommitMessage,
    AmendConfirmation,
    WorktreeDiff,
    CommitDiff,
    Search,
    Profiles,
    IgnorePathInput,
}

pub(super) struct HistoryScreen<'a> {
    pub(super) history: Option<&'a Result<Vec<String>, String>>,
    pub(super) searching: bool,
    pub(super) query: &'a str,
    pub(super) search_message: &'a str,
    pub(super) root: &'a Path,
    pub(super) selection: &'a mut ListState,
}
