//! Read-only preparation for existing confirmation flows. Writes stay in their adapters.
use crate::{branches, worktree};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
pub enum Request {
    Discard(worktree::Entry),
    Untrack(worktree::Entry),
    Remove(PathBuf),
    Move(PathBuf, PathBuf),
    Branch(OsString),
    Previous,
    IgnoreSelected(worktree::Entry),
    IgnorePath(PathBuf),
}
pub enum Value {
    Discard(worktree::DiscardPreview),
    Untrack(worktree::UntrackPreview),
    Remove(worktree::RemovePathPreview),
    Move(worktree::MovePathPreview),
    Branch(branches::BranchPreview),
    Previous(branches::PreviousCheckoutPreview),
    Ignore(Option<worktree::IgnoreRule>),
}
#[derive(Clone, Copy)]
pub enum Context {
    Discard,
    Untrack,
    Remove,
    Move,
    BranchRename,
    BranchDelete,
    Previous,
    IgnoreSelected,
    IgnorePath,
}
impl Context {
    pub fn label(self) -> &'static str {
        match self {
            Self::Discard => "Reviewing discard...",
            Self::Untrack => "Reviewing stop tracking...",
            Self::Remove => "Reviewing tracked-path removal...",
            Self::Move => "Reviewing tracked-path move...",
            Self::BranchRename => "Reviewing branch rename...",
            Self::BranchDelete => "Reviewing branch deletion...",
            Self::Previous => "Resolving previous checkout...",
            Self::IgnoreSelected | Self::IgnorePath => "Inspecting ignore rules...",
        }
    }
    pub fn is_branch(self) -> bool {
        matches!(
            self,
            Self::BranchRename | Self::BranchDelete | Self::Previous
        )
    }
}
pub fn prepare(root: &Path, request: Request) -> Result<Value, String> {
    match request {
        Request::Discard(entry) => worktree::preview_discard(root, &entry).map(Value::Discard),
        Request::Untrack(entry) => worktree::preview_untrack(root, &entry).map(Value::Untrack),
        Request::Remove(path) => worktree::preview_remove_path(root, &path).map(Value::Remove),
        Request::Move(source, target) => {
            worktree::preview_move_path(root, &source, &target).map(Value::Move)
        }
        Request::Branch(name) => branches::preview_branch(root, &name).map(Value::Branch),
        Request::Previous => branches::preview_previous_checkout(root).map(Value::Previous),
        Request::IgnoreSelected(entry) => worktree::ignore_rule(root, &entry).map(Value::Ignore),
        Request::IgnorePath(path) => worktree::ignore_rule_for_path(root, &path).map(Value::Ignore),
    }
}
