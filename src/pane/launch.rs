//! Choosing the program for the agent pane from the command line, and checking
//! it is installed before Gitview takes over the terminal.

use std::{
    env,
    ffi::{OsStr, OsString},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

/// The program to run in the agent pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequest {
    /// The program and its arguments.
    pub argv: Vec<String>,
    /// The product name when the program came from a shortcut such as
    /// `--claude`, so a missing program can be explained in plain words.
    pub product: Option<&'static str>,
}

/// Splits the agent flag off the command line.
///
/// `--run <command> [args...]`, `--claude [args...]`, and `--codex [args...]`
/// start a program in the agent pane. Everything after the flag belongs to that
/// program, so `gitview --run ls --help` shows the help of `ls`, not Gitview's.
/// Returns Gitview's own arguments and the request, if any.
pub fn split_agent_args(
    args: &[OsString],
) -> Result<(Vec<OsString>, Option<AgentRequest>), String> {
    let found = args.iter().enumerate().find_map(|(index, arg)| {
        let shortcut = match arg.to_str()? {
            "--run" => None,
            "--claude" => Some(("claude", "Claude Code")),
            "--codex" => Some(("codex", "OpenAI Codex")),
            _ => return None,
        };
        Some((index, shortcut))
    });
    let Some((index, shortcut)) = found else {
        return Ok((args.to_vec(), None));
    };

    let flag = args[index].to_string_lossy().into_owned();
    let rest = args[index + 1..]
        .iter()
        .map(|part| part.to_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| format!("{flag} needs a command made of valid UTF-8"))?;
    let request = match shortcut {
        Some((program, product)) => AgentRequest {
            argv: std::iter::once(program.to_owned()).chain(rest).collect(),
            product: Some(product),
        },
        None if rest.is_empty() => {
            return Err("--run needs a command, for example: gitview --run claude".into());
        }
        None => AgentRequest {
            argv: rest,
            product: None,
        },
    };
    Ok((args[..index].to_vec(), Some(request)))
}

fn is_executable_file(path: &std::path::Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

/// Finds `name` the way a shell would: a name with a slash is taken as the path
/// it spells, and any other name is looked up in each folder of `search_path`.
pub fn find_executable(name: &str, search_path: Option<&OsStr>) -> Option<PathBuf> {
    if name.contains('/') {
        let path = PathBuf::from(name);
        return is_executable_file(&path).then_some(path);
    }
    env::split_paths(search_path?)
        .filter(|folder| !folder.as_os_str().is_empty())
        .map(|folder| folder.join(name))
        .find(|candidate| is_executable_file(candidate))
}

/// Confirms the requested program exists, or explains what to do about it.
pub fn check_installed(request: &AgentRequest, search_path: Option<&OsStr>) -> Result<(), String> {
    let program = &request.argv[0];
    if find_executable(program, search_path).is_some() {
        return Ok(());
    }
    Err(match request.product {
        Some(product) => format!(
            "{product} (`{program}`) was not found on your PATH. Install it and run gitview \
             again, or start another program with `gitview --run <command>`."
        ),
        None if program.contains('/') => format!(
            "`{program}` is not an executable file. Check the path, or give a command that is \
             on your PATH."
        ),
        None => format!(
            "`{program}` was not found on your PATH. Check the name, or give its path, such as \
             ./{program}."
        ),
    })
}
