use crossterm_keybind::KeyBindTrait;
use gitview::{
    app::{
        run::{Navigation, run_app_with_agent},
        terminal::{BracketedPasteMode, TerminalHost},
    },
    ui::HELP,
};
use std::{env, process::ExitCode};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    // Everything after --run is the agent's command line, so flags such as
    // --help belong to the agent, not to Gitview.
    let (args, agent) = match args.iter().position(|arg| arg == "--run") {
        Some(index) => {
            let command = args[index + 1..]
                .iter()
                .map(|part| part.to_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
                .ok_or("--run needs a command made of valid UTF-8")?;
            if command.is_empty() {
                return Err("--run needs a command, for example: gitview --run claude".into());
            }
            (args[..index].to_vec(), Some(command))
        }
        None => (args, None),
    };
    if args.iter().any(|arg| arg == "--shell-integration") {
        print_shell_integration();
        return Ok(());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--profile-config-for") {
        let folder = args
            .get(index + 1)
            .ok_or("--profile-config-for requires a folder path")?;
        let config_dir = gitview::config::config_dir()?;
        if let Some(profile) =
            gitview::config::profile_for_worktree(std::path::Path::new(folder), &config_dir)?
        {
            println!("{}", profile.config_path.display());
        }
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "Usage: gitview [--ascii] [--version]\n       gitview [--ascii] --run <command> [args...]\n       gitview init [directory]\n       gitview clone <repository> [directory]\n       gitview --shell-integration\n\nOpen the current Git repository. Use init or clone to set up a repository.\nUse --run to start a program, such as a coding agent, in a pane beside Gitview; Ctrl+\\ moves focus between them. Use --ascii for plain terminal borders. Folder-specific Git config profiles are loaded automatically. Run --shell-integration for the optional zsh hook.\n\n{HELP}"
        );
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("gitview {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "init") {
        return run_init(&args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "clone") {
        return run_clone(&args[1..]);
    }
    Navigation::init_and_load_file(None)
        .map_err(|e| format!("Could not initialize keybindings: {e}"))?;
    let ascii = args.iter().any(|arg| arg == "--ascii");
    let cwd =
        env::current_dir().map_err(|error| format!("Could not read current folder: {error}"))?;
    let config_dir = gitview::config::config_dir()?;
    let selected_profile = gitview::config::profile_for_worktree(&cwd, &config_dir)?;
    let (profile_label, config_context) = match selected_profile {
        Some(profile) => (
            profile.name.clone(),
            gitview::config::GitConfigContext::for_profile(profile.name, profile.config_path),
        ),
        None => (
            "Primary".to_owned(),
            gitview::config::GitConfigContext::new(None),
        ),
    };
    let mut profiles = gitview::config::load_profiles(&config_dir)?;
    gitview::process::install_git_config_context(config_context)?;
    let output = gitview::process::git_command()
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Could not run Git; install Git and ensure it is in PATH: {e}"))?;
    if !output.status.success() {
        return Err("Not in a Git repository. Run gitview from a repository.".into());
    }
    let root = gitview::repository::parse_root(&output.stdout);
    let mut terminal =
        ratatui::try_init().map_err(|e| format!("Could not initialize terminal: {e}"))?;
    let paste_mode = match BracketedPasteMode::enable() {
        Ok(mode) => mode,
        Err(error) => {
            ratatui::restore();
            return Err(format!("Could not enable bracketed paste: {error}"));
        }
    };
    let mut host = TerminalHost::new(paste_mode);
    let result = run_app_with_agent(
        &mut terminal,
        &mut host,
        &root,
        ascii,
        profile_label,
        &config_dir,
        &mut profiles,
        agent,
    );
    let paste_result = host.disable_paste();
    ratatui::restore();
    result.map_err(|e| format!("Terminal error: {e}"))?;
    paste_result.map_err(|error| format!("Could not disable bracketed paste: {error}"))?;
    Ok(())
}

fn run_init(args: &[std::ffi::OsString]) -> Result<(), String> {
    if args.len() > 1 {
        return Err("Usage: gitview init [directory]".into());
    }
    let directory = args.first().map(std::path::PathBuf::from).unwrap_or(
        env::current_dir().map_err(|error| format!("Could not read current folder: {error}"))?,
    );
    gitview::repository::initialize(&directory)?;
    println!("Initialized Git repository at {}", directory.display());
    Ok(())
}

fn run_clone(args: &[std::ffi::OsString]) -> Result<(), String> {
    if args.is_empty() || args.len() > 2 {
        return Err("Usage: gitview clone <repository> [directory]".into());
    }
    let source = std::path::Path::new(&args[0]);
    let destination = args.get(1).map(std::path::Path::new);
    println!("Cloning {}...", source.display());
    gitview::repository::clone_repository(source, destination)?;
    println!("Clone complete.");
    Ok(())
}

fn print_shell_integration() {
    print!(
        r#"# Optional Gitview zsh integration. Add this output to ~/.zshrc to enable; remove it to disable.
git() {{
  local profile_config
  profile_config="$(command gitview --profile-config-for "$PWD")" || return $?
  if [[ -n "$profile_config" ]]; then
    GIT_CONFIG_GLOBAL="$profile_config" command git "$@"
  else
    command git "$@"
  fi
}}
"#
    );
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
