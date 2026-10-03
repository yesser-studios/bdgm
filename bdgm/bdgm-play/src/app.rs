use std::process::ExitStatus;

use bdgm::game::{Game, ValidatedGame};
use bdgm_play::{
    args::Args, error::AppError, image::resolve_image_args, install::install, launch::launch_game,
    read_manifest,
};
use clap::Parser;
use platform_dirs::AppDirs;

pub(crate) async fn run() -> anyhow::Result<ExitStatus> {
    let app_dirs = AppDirs::new(Some("bdgm-play"), true).ok_or(AppError::NoAppDirs)?;

    let args = Args::parse();
    let (args, tempdir) = resolve_image_args(args)?;

    let manifest = read_manifest(&args)?;

    let game = Game::from_str(&manifest)?;
    let game = ValidatedGame::validate(game)?;

    println!("Found: {} version {}", game.name(), game.version());
    install(&game, &app_dirs, &args)?;

    println!("Launching...");
    let status = launch_game(&game, &args, &app_dirs, true).await?;
    drop(tempdir);

    Ok(status)
}
