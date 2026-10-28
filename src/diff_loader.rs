//! Requests for the single replaceable ordinary-diff preparation worker.
use crate::{ui::PreparedDiff, worktree::Entry};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
pub struct Request {
    pub id: u64,
    pub source: Source,
}
pub enum Source {
    Worktree(Entry),
    Commit { revision: String, file: OsString },
    Loaded { text: String, path: Option<PathBuf> },
}
pub fn load(root: &Path, request: Request) -> (u64, Result<PreparedDiff, String>) {
    let result = match request.source {
        Source::Worktree(entry) => crate::worktree::load_diff(root, &entry)
            .map(|text| PreparedDiff::new(text, Some(Path::new(&entry.path)))),
        Source::Commit { revision, file } => crate::details::load_diff(root, &revision, &file)
            .map(|text| PreparedDiff::new(text, Some(Path::new(&file)))),
        Source::Loaded { text, path } => Ok(PreparedDiff::new(text, path.as_deref())),
    };
    (request.id, result)
}
