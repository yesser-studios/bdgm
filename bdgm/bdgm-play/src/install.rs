use std::io;

use bdgm::game::ValidatedGame;
use fs_extra::dir::{self, CopyOptions};
use platform_dirs::AppDirs;

use crate::{args::Args, dirs::get_app_dir_path};

pub fn install(game: &ValidatedGame, app_dirs: &AppDirs, args: &Args) -> anyhow::Result<()> {
    let id = game.id();
    let game_dir = app_dirs.data_dir.join(&id);
    let install_dir = game_dir.join("app").join(game.version());
    let install_part_dir = game_dir
        .join("app")
        .join(game.version())
        .with_added_extension("part");
    if !install_dir.try_exists()? {
        if install_part_dir.try_exists()? {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Installation directory does not exist but installation part directory (path: {}) does. An installation may already be in progress for this game.", install_part_dir.display())
            ).into());
        }
        println!("Copying files...");
        dir::create_all(&install_part_dir, false)?;
        dir::copy(
            &get_app_dir_path(args),
            &install_part_dir,
            &CopyOptions::new().overwrite(true).content_only(true),
        )?;
        std::fs::rename(&install_part_dir, install_dir)?;
        println!("Copied!");
    }

    Ok(())
}
