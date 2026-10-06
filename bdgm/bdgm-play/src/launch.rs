use std::{collections::HashMap, process::ExitStatus, sync::Arc};

use std::process::Command;

use bdgm::game::ValidatedGame;
use platform_dirs::AppDirs;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    args::Args,
    dirs::{GameDirs, get_app_dir_path},
    server::play_html_game,
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

pub async fn launch_game(
    game: &ValidatedGame,
    args: &Args,
    app_dirs: &AppDirs,
    verbose: bool,
) -> anyhow::Result<ExitStatus> {
    let envvars = get_envvars(&game, &args, &app_dirs);
    let runtime = game.runtime();
    let game_dirs = GameDirs::from(&game, &app_dirs);
    let runtime_path = args
        .runtime
        .as_ref()
        .map(|x| x.to_string_lossy().to_string());

    let status = match runtime {
        bdgm::runtime::Runtime::Java => Command::new(runtime_path.as_deref().unwrap_or("java"))
            .args(game.runtime_args())
            .arg("-jar")
            .arg(game_dirs.install.join(game.executable()))
            .args(game.args())
            .envs(envvars)
            .current_dir(&game_dirs.install)
            .status()?,
        bdgm::runtime::Runtime::Dotnet => Command::new(runtime_path.as_deref().unwrap_or("dotnet"))
            .args(game.runtime_args())
            .arg(game_dirs.install.join(game.executable()))
            .args(game.args())
            .envs(envvars)
            .current_dir(&game_dirs.install)
            .status()?,
        bdgm::runtime::Runtime::Python => Command::new(runtime_path.as_deref().unwrap_or("python"))
            .args(game.runtime_args())
            .arg(game_dirs.install.join(game.executable()))
            .args(game.args())
            .envs(envvars)
            .current_dir(&game_dirs.install)
            .status()?,
        bdgm::runtime::Runtime::HTML => {
            play_html_game(&game, &game_dirs.install, app_dirs, verbose).await?;
            std::process::ExitStatus::default()
        }
        bdgm::runtime::Runtime::Windows => {
            if cfg!(target_os = "windows") {
                Command::new(game_dirs.install.join(game.executable()))
                    .args(game.args())
                    .envs(envvars)
                    .current_dir(&game_dirs.install)
                    .status()?
            } else {
                Command::new(runtime_path.as_deref().unwrap_or("wine"))
                    .args(game.runtime_args())
                    .arg(game_dirs.install.join(game.executable()))
                    .args(game.args())
                    .envs(envvars)
                    .env("WINEPREFIX", game_dirs.root.join("wineprefix"))
                    .current_dir(&game_dirs.install)
                    .status()?
            }
        }
    };
    if !status.success() {
        if verbose {
            eprintln!("Your game crashed: {status}");
            eprintln!("Setting a runtime with `--runtime /path/to/runtime` may fix your issue.");
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
    use bdgm::runtime::Runtime;

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

    let child = cmd
        .envs(envvars)
        .current_dir(&game_dirs.install)
        .kill_on_drop(true)
        .spawn()?;

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
