use std::io;

use bdgm::game::ValidatedGame;
use fs_extra::dir::{self, CopyOptions};
use platform_dirs::AppDirs;

use crate::{
    cli::args::Args, core::dirs::get_app_dir_path, core::error::IoError, core::fs::acquire_lock,
};

pub fn install(game: &ValidatedGame, app_dirs: &AppDirs, args: &Args) -> anyhow::Result<()> {
    let id = game.id();
    let game_dir = app_dirs.data_dir.join(id);
    let install_dir = game_dir.join("app").join(game.version());
    let install_part_dir = install_dir.with_added_extension("part");
    if !install_dir.try_exists()? {
        let lock = acquire_lock(&install_dir, false, false).map_err(|e| match e {
            crate::core::error::IoError::TryLockError(try_lock_error) => io::Error::new(
                io::ErrorKind::ResourceBusy,
                format!("An installation is in progress: {}", try_lock_error),
            )
            .into(),
            _ => e,
        })?;
        if install_part_dir.try_exists()? {
            println!("Installation part directory exists but lock is not held. Removing...");
            std::fs::remove_dir_all(&install_part_dir)?;
        }
        dir::create_all(&install_part_dir, false)?;

        println!("Copying files...");
        dir::copy(
            &get_app_dir_path(args).ok_or(IoError::PathNone)?,
            &install_part_dir,
            &CopyOptions::new().overwrite(true).content_only(true),
        )?;
        std::fs::rename(&install_part_dir, install_dir)?;
        drop(lock);
        println!("Copied!");
    }

    Ok(())
}
