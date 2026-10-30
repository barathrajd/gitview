use std::{env, io, process::Command};

use crate::ui::{self};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm_keybind::{KeyBind, KeyBindTrait};
use ratatui::{Terminal, widgets::ListState};

use super::run::Host;

pub(super) fn backend_error(error: impl std::error::Error) -> io::Error {
    io::Error::other(error.to_string())
}

#[derive(KeyBind)]
pub enum Navigation {
    #[keybindings["j"]]
    Down,
    #[keybindings["k"]]
    Up,
    #[keybindings["h"]]
    Back,
    #[keybindings["l"]]
    Open,
}

pub(super) fn reload_profile_context(
    root: &std::path::Path,
    config_dir: &std::path::Path,
    profiles: &mut Vec<crate::config::FolderProfile>,
    profile_label: &mut String,
) -> Result<(), String> {
    let updated_profiles = crate::config::load_profiles(config_dir)?;
    let selected = crate::config::profile_for_worktree(root, config_dir)?;
    let (label, context) = match selected {
        Some(profile) => (
            profile.name.clone(),
            crate::config::GitConfigContext::for_profile(profile.name, profile.config_path),
        ),
        None => (
            "Primary".to_owned(),
            crate::config::GitConfigContext::new(None),
        ),
    };
    crate::process::update_git_config_context(context)?;
    *profiles = updated_profiles;
    *profile_label = label;
    Ok(())
}

pub(super) fn edit_profile_registry<H: Host>(
    terminal: &mut Terminal<H::Backend>,
    host: &mut H,
    root: &std::path::Path,
    config_dir: &std::path::Path,
    profiles: &mut Vec<crate::config::FolderProfile>,
    profile_label: &mut String,
    feedback: &mut String,
) -> io::Result<()> {
    let registry_path = config_dir.join("profiles.json");
    if !registry_path.exists()
        && let Err(error) = crate::config::save_profiles(config_dir, profiles)
    {
        *feedback = error;
        return Ok(());
    }
    let editor = env::var_os("VISUAL").or_else(|| env::var_os("EDITOR"));
    let Some(editor) = editor else {
        *feedback = "Set VISUAL or EDITOR to edit profiles.json.".into();
        return Ok(());
    };

    let editor_result = host.run_external(terminal, || {
        Command::new(editor).arg(&registry_path).status()
    })?;

    match editor_result {
        Ok(status) if status.success() => {
            match reload_profile_context(root, config_dir, profiles, profile_label) {
                Ok(()) => *feedback = "Git config profiles reloaded.".into(),
                Err(error) => *feedback = error,
            }
        }
        Ok(status) => *feedback = format!("Profile editor exited with {status}."),
        Err(error) => *feedback = format!("Could not open profile editor: {error}"),
    }
    Ok(())
}

pub(super) enum CommitReadResult {
    Create(Result<crate::commit::CommitPreview, String>),
    Amend(Result<crate::commit::AmendPreview, String>),
}

pub(super) fn hunk_line_positions(
    prepared: Option<&ui::PreparedDiff>,
    diff: &str,
    raw: bool,
    side_by_side: bool,
    terminal_width: u16,
) -> Vec<usize> {
    prepared.map_or_else(
        || ui::diff_hunk_line_positions(diff, terminal_width, raw, side_by_side),
        |prepared| prepared.hunk_positions(terminal_width, raw, side_by_side),
    )
}

pub(super) fn navigation_key(key: KeyEvent) -> KeyCode {
    match Navigation::dispatch(&key).first() {
        Some(Navigation::Down) => KeyCode::Down,
        Some(Navigation::Up) => KeyCode::Up,
        Some(Navigation::Back) => KeyCode::Esc,
        Some(Navigation::Open) => KeyCode::Enter,
        None => key.code,
    }
}

pub(super) fn normalize_commit_paste(pasted: &str, multiline: bool) -> String {
    pasted
        .chars()
        .filter_map(|character| match character {
            '\r' => None,
            '\n' if multiline => Some('\n'),
            '\n' | '\t' => Some(' '),
            character if character.is_control() => None,
            character => Some(character),
        })
        .collect()
}

pub(super) fn combined_commit_message(subject: &str, description: &str) -> String {
    if description.trim().is_empty() {
        subject.to_owned()
    } else {
        format!("{}\n\n{}", subject.trim_end(), description.trim_end())
    }
}

/// Whether a message saved from the external editor should be committed
/// directly: its subject isn't empty and it differs from the reviewed draft.
pub(super) fn commit_after_editor(subject: &str, description: &str, edited: &str) -> bool {
    let (edited_subject, edited_description) = split_commit_message(edited);
    !edited_subject.trim().is_empty()
        && combined_commit_message(&edited_subject, &edited_description)
            != combined_commit_message(subject, description)
}

pub(super) fn split_commit_message(message: &str) -> (String, String) {
    match message.split_once('\n') {
        Some((subject, description)) => (
            subject.to_owned(),
            description.trim_start_matches('\n').trim_end().to_owned(),
        ),
        None => (message.to_owned(), String::new()),
    }
}

pub(super) fn update_search_results(
    history: Option<&Result<Vec<String>, String>>,
    query: &str,
    results: &mut Vec<usize>,
    selection: &mut ListState,
) {
    let previously_selected = results.get(selection.selected().unwrap_or(0)).copied();
    *results = history
        .and_then(|result| result.as_ref().ok())
        .map(|rows| crate::history::search_matches(rows, query))
        .unwrap_or_default();
    let selected = previously_selected
        .and_then(|previous| results.iter().position(|index| *index == previous))
        .or_else(|| (!results.is_empty()).then_some(0));
    selection.select(selected);
}

pub(super) fn selected_search_commit(
    history: Option<&Result<Vec<String>, String>>,
    results: &[usize],
    selected: Option<usize>,
) -> Option<(usize, String)> {
    let index = *results.get(selected.unwrap_or(0))?;
    let id = history
        .and_then(|result| result.as_ref().ok())?
        .get(index)
        .and_then(|row| crate::history::commit_id(row))?
        .to_owned();
    Some((index, id))
}

/// Rows a full-screen page shows: the Gitview area less `chrome`, the rows its
/// borders, titles, and key hints take.
pub(super) fn screen_page_rows(area: Option<ratatui::layout::Rect>, chrome: u16) -> usize {
    area.map_or(1, |area| {
        usize::from(area.height.saturating_sub(chrome)).max(1)
    })
}

/// Chrome for the branch list: its border and the two hint rows.
pub(super) const BRANCH_CHROME: u16 = 4;
/// Chrome estimate for the diff, commit details, and profiles; their headers
/// differ slightly.
pub(super) const PAGE_CHROME: u16 = 6;

/// Apply a motion key to a list of `len` rows. Returns false for keys the
/// list should handle itself. Ctrl and Alt letters that are not motions are
/// consumed, so they never run the list's plain-letter actions.
pub(super) fn list_motion(
    parser: &mut crate::motion::MotionParser,
    key: KeyEvent,
    selection: &mut ratatui::widgets::ListState,
    len: usize,
    page: usize,
) -> bool {
    use crate::motion::{Motion, Parsed};
    match parser.key(key) {
        Parsed::Pending => true,
        Parsed::Motion(Motion::Center) => {
            if let Some(selected) = selection.selected() {
                *selection.offset_mut() = selected.saturating_sub(page / 2);
            }
            true
        }
        Parsed::Motion(motion) => {
            if len > 0 {
                selection.select(motion.select(selection.selected(), len, page));
            }
            true
        }
        Parsed::Other => {
            matches!(key.code, KeyCode::Char(_))
                && key.modifiers.intersects(
                    crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT,
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn keybinding_library_maps_vim_navigation_without_capturing_modifiers() {
        Navigation::init_and_load_file(None).unwrap();
        for (letter, expected) in [
            ('j', KeyCode::Down),
            ('k', KeyCode::Up),
            ('h', KeyCode::Esc),
            ('l', KeyCode::Enter),
        ] {
            assert_eq!(
                navigation_key(KeyEvent::new(KeyCode::Char(letter), KeyModifiers::NONE)),
                expected
            );
            assert_eq!(
                navigation_key(KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL)),
                KeyCode::Char(letter)
            );
        }
        assert_eq!(
            navigation_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            KeyCode::Down
        );
    }

    #[test]
    fn commit_paste_keeps_description_newlines_and_flattens_subject_lines() {
        assert_eq!(
            normalize_commit_paste("feat: add editor\r\nwith paste", false),
            "feat: add editor with paste"
        );
        assert_eq!(
            normalize_commit_paste("First line\r\nSecond\tline", true),
            "First line\nSecond line"
        );
    }

    #[test]
    fn splits_and_recombines_existing_commit_subject_and_description() {
        let (subject, description) =
            split_commit_message("fix: preserve files\n\nExplain the safety rule.\n");
        assert_eq!(subject, "fix: preserve files");
        assert_eq!(description, "Explain the safety rule.");
        assert_eq!(
            combined_commit_message(&subject, &description),
            "fix: preserve files\n\nExplain the safety rule."
        );
    }

    #[test]
    fn saved_editor_changes_with_a_subject_commit_without_a_second_enter() {
        assert!(commit_after_editor("", "", "feat: add editor\n"));
        assert!(commit_after_editor("old", "body", "new subject\n\nbody\n"));
        // Quitting without changes, or saving an empty subject, returns to the form.
        assert!(!commit_after_editor("old", "", "old\n"));
        assert!(!commit_after_editor("old", "body", "old\n\nbody\n"));
        assert!(!commit_after_editor("old", "", "\n"));
        assert!(!commit_after_editor("", "", ""));
    }

    #[test]
    fn search_selection_resolves_to_the_matching_commit_for_direct_open() {
        let rows = Ok(vec![
            "* abcdef0 First result\u{1f}abcdef0".to_owned(),
            "* 1234567 Second result\u{1f}1234567".to_owned(),
        ]);
        assert_eq!(
            selected_search_commit(Some(&rows), &[1], Some(0)),
            Some((1, "1234567".to_owned()))
        );
        assert_eq!(selected_search_commit(Some(&rows), &[], None), None);
    }
}
