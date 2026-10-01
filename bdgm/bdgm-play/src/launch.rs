use std::{
    collections::HashMap,
    process::{Command, ExitStatus},
};

use bdgm::game::ValidatedGame;
use platform_dirs::AppDirs;

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
    envvars.insert(
        "BDGM_APP",
        get_app_dir_path(args).to_string_lossy().into_owned(),
    );
    envvars.insert("BDGM_DISC", args.location.to_string_lossy().into_owned());
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
