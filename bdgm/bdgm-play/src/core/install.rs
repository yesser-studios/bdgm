use std::io;

use bdgm::game::ValidatedGame;
use fs_extra::dir::{self, CopyOptions};
use platform_dirs::AppDirs;

use crate::{
    cli::args::Args,
    core::{
        dirs::{GameDirs, get_app_dir_path},
        error::IoError,
        fs::acquire_lock,
    },
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

/// Installs a game straight off a raw disc (`\\.\X:`) without dumping or
/// fully extracting the image: only the `/BDGM/APP` subtree is read.
///
/// Mirrors [`install`]'s locking, `.part` handling and early return when
/// already installed. Leaves a `.part` dir behind on failure, cleaned up
/// by the next run — same as [`install`].
#[cfg(windows)]
pub fn install_from_raw_disc(
    game: &ValidatedGame,
    app_dirs: &AppDirs,
    drive: &str,
) -> anyhow::Result<()> {
    let install_dir = GameDirs::from(game, app_dirs).install;
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

        println!("Copying files from disc...");
        crate::core::dump::copy_app_from_raw_disc(drive, game, &install_part_dir)?;
        std::fs::rename(&install_part_dir, install_dir)?;
        drop(lock);
        println!("Copied!");
    }

    Ok(())
}
