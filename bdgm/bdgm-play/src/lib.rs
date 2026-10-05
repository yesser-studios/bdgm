use std::{fs::File, io::Read, process::ExitStatus};

use bdgm::game::{Game, ValidatedGame};
use platform_dirs::AppDirs;

use crate::{
    args::Args, dirs::try_get_manifest_path, error::AppError, install::install, launch::launch_game,
};
pub mod args;
pub mod dirs;
pub mod dump;
pub mod error;
pub mod fs;
pub mod image;
pub mod install;
pub mod launch;
pub mod server;

pub fn read_manifest(args: &Args) -> anyhow::Result<String> {
    let path = try_get_manifest_path(args)?;
    let mut manifest = File::open(path)?;
    let mut contents = String::new();
    manifest.read_to_string(&mut contents)?;
    Ok(contents)
}

pub async fn run_sanitized(args: Args, verbose: bool) -> anyhow::Result<ExitStatus> {
    let app_dirs = AppDirs::new(Some("bdgm-play"), true).ok_or(AppError::NoAppDirs)?;
    let manifest = read_manifest(&args)?;

    let game = Game::from_str(&manifest)?;
    let game = ValidatedGame::validate(game)?;

    println!("Found: {} version {}", game.name(), game.version());
    install(&game, &app_dirs, &args)?;

    println!("Launching...");
    let status = launch_game(&game, &args, &app_dirs, verbose).await?;

    Ok(status)
}
