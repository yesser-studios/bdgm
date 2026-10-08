use std::{collections::HashMap, path::PathBuf, process::ExitStatus, sync::Arc};

use bdgm::game::ValidatedGame;
use bdgm::runtime::Runtime;
use platform_dirs::AppDirs;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    cli::args::Args,
    core::{
        dirs::{GameDirs, get_app_dir_path},
        error::RuntimeNotFound,
        server::play_html_game,
    },
};

pub fn get_envvars(
    game: &ValidatedGame,
    args: &Args,
    app_dirs: &AppDirs,
) -> HashMap<&'static str, String> {
    let game_dirs = GameDirs::from(game, app_dirs);

    let mut envvars = HashMap::new();
    envvars.insert("BDGM_DATA", game_dirs.data.to_string_lossy().into_owned());
    envvars.insert(
        "XDG_DATA_DIRS",
        game_dirs.data.to_string_lossy().into_owned(),
    );
    envvars.insert("BDGM_CACHE", game_dirs.cache.to_string_lossy().into_owned());
    if let Some(path) = get_app_dir_path(args) {
        envvars.insert("BDGM_APP", path.to_string_lossy().into_owned());
    }
    if let Some(path) = &args.location {
        envvars.insert("BDGM_DISC", path.to_string_lossy().into_owned());
    }
    envvars.insert("BDGM_VERSION", game.bdgm_version().to_string());

    envvars
}

/// Build the async command for a native (non-HTML) game.
///
/// Single place for runtime dispatch shared by `run_game` (CLI, awaited)
/// and `spawn_game_process` (GUI, killable). Returns an error for HTML
/// games — use `server::start_html_server` instead.
pub fn build_command(
    game: &ValidatedGame,
    args: &Args,
    app_dirs: &AppDirs,
) -> anyhow::Result<tokio::process::Command> {
    let envvars = get_envvars(game, args, app_dirs);
    let game_dirs = GameDirs::from(game, app_dirs);
    let runtime_path = args
        .runtime
        .as_ref()
        .map(|x| x.to_string_lossy().to_string());

    let mut cmd = match game.runtime() {
        Runtime::Java => {
            let mut c = tokio::process::Command::new(runtime_path.as_deref().unwrap_or("java"));
            c.args(game.runtime_args())
                .arg("-jar")
                .arg(game_dirs.install.join(game.executable()))
                .args(game.args());
            c
        }
        Runtime::Dotnet => {
            let mut c = tokio::process::Command::new(runtime_path.as_deref().unwrap_or("dotnet"));
            c.args(game.runtime_args())
                .arg(game_dirs.install.join(game.executable()))
                .args(game.args());
            c
        }
        Runtime::Python => {
            let mut c = tokio::process::Command::new(runtime_path.as_deref().unwrap_or("python"));
            c.args(game.runtime_args())
                .arg(game_dirs.install.join(game.executable()))
                .args(game.args());
            c
        }
        Runtime::Windows => {
            if cfg!(target_os = "windows") {
                let mut c = tokio::process::Command::new(game_dirs.install.join(game.executable()));
                c.args(game.args());
                c
            } else {
                let mut c = tokio::process::Command::new(runtime_path.as_deref().unwrap_or("wine"));
                c.args(game.runtime_args())
                    .arg(game_dirs.install.join(game.executable()))
                    .args(game.args())
                    .env("WINEPREFIX", game_dirs.root.join("wineprefix"));
                c
            }
        }
        Runtime::HTML => {
            return Err(anyhow::anyhow!(
                "HTML games must be started via server::start_html_server"
            ));
        }
    };

    cmd.envs(envvars).current_dir(&game_dirs.install);
    Ok(cmd)
}

/// Map a `spawn` I/O error to [`RuntimeNotFound`] when the runtime binary
/// is missing (`ErrorKind::NotFound`). All other errors pass through.
///
/// The native Windows launch (spawning the game exe directly) is excluded:
/// a `NotFound` there means a missing game file, not a missing runtime.
fn map_spawn_error(
    e: std::io::Error,
    game: &ValidatedGame,
    runtime: Option<&PathBuf>,
) -> anyhow::Error {
    let is_native_windows_launch =
        cfg!(target_os = "windows") && matches!(game.runtime(), Runtime::Windows);
    if e.kind() == std::io::ErrorKind::NotFound && !is_native_windows_launch {
        match runtime {
            Some(p) => anyhow::anyhow!(RuntimeNotFound::from_path(p, None)),
            None => anyhow::anyhow!(RuntimeNotFound::from_game(game)),
        }
    } else {
        e.into()
    }
}
/// Run a game to completion, awaiting its exit.
///
/// CLI path: just await this. HTML games are served via
/// `server::play_html_game` (start + await); native games use the shared
/// `build_command` and await the child.
pub async fn run_game(
    game: &ValidatedGame,
    args: &Args,
    app_dirs: &AppDirs,
    verbose: bool,
) -> anyhow::Result<ExitStatus> {
    let game_dirs = GameDirs::from(game, app_dirs);

    let status = match game.runtime() {
        Runtime::HTML => {
            play_html_game(game, &game_dirs.install, app_dirs, verbose).await?;
            std::process::ExitStatus::default()
        }
        _ => {
            build_command(game, args, app_dirs)?
                .spawn()
                .map_err(|e| map_spawn_error(e, game, args.runtime.as_ref()))?
                .wait()
                .await?
        }
    };
    if !status.success() && verbose {
        eprintln!("Your game crashed: {status}");
        if (cfg!(windows) && !matches!(game.runtime(), Runtime::Windows)) || cfg!(not(windows)) {
            eprintln!("Specifying a runtime with `--runtime /path/to/runtime` may fix your issue.");
        }
    }

    Ok(status)
}

/// Shared handle to a running native game process.
///
/// Cloning shares ownership of the same child. The child is spawned with
/// `kill_on_drop(true)`, so dropping the last handle without an explicit
/// kill still terminates the game. Killing is done via a brief lock +
/// `start_kill`, while the exit-waiter polls `try_wait` so the lock is
/// never held across a long wait.
#[derive(Clone, Debug)]
pub struct ProcessHandle {
    pub child: Arc<AsyncMutex<tokio::process::Child>>,
}

/// Spawn a native (non-HTML) game without blocking, returning a killable handle.
///
/// Returns an error for HTML games — use `server::start_html_server` instead.
pub fn spawn_game_process(
    game: &ValidatedGame,
    args: &Args,
    app_dirs: &AppDirs,
) -> anyhow::Result<ProcessHandle> {
    let child = build_command(game, args, app_dirs)?
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| map_spawn_error(e, game, args.runtime.as_ref()))?;

    Ok(ProcessHandle {
        child: Arc::new(AsyncMutex::new(child)),
    })
}

/// Kill a running process without blocking the caller for long.
pub async fn kill_process(handle: &ProcessHandle) {
    let mut child = handle.child.lock().await;
    let _ = child.start_kill();
}

/// Poll a running process until it exits, returning its exit code (if any).
///
/// Uses `try_wait` in a loop so the child mutex is only held briefly,
/// leaving room for `kill_process` to acquire it.
pub async fn wait_for_process(handle: ProcessHandle) -> anyhow::Result<Option<i32>> {
    loop {
        {
            let mut child = handle.child.lock().await;
            if let Some(status) = child.try_wait()? {
                return Ok(status.code());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}
