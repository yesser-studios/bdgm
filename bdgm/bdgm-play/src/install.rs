use std::io;

use bdgm::game::ValidatedGame;
use fs_extra::dir::{self, CopyOptions};
use platform_dirs::AppDirs;

use crate::{args::Args, dirs::get_app_dir_path, fs::acquire_lock};

pub fn install(game: &ValidatedGame, app_dirs: &AppDirs, args: &Args) -> anyhow::Result<()> {
    let id = game.id();
    let game_dir = app_dirs.data_dir.join(&id);
    let install_dir = game_dir.join("app").join(game.version());
    let install_part_dir = game_dir.with_added_extension("part");
    if !install_dir.try_exists()? {
        let lock = acquire_lock(&install_dir).map_err(|_| {
            io::Error::new(
                io::ErrorKind::ResourceBusy,
                "An installation is in progress.",
            )
        });
        if install_part_dir.try_exists()? {
            println!("Installation part directory exists but lock is not held. Removing...");
            std::fs::remove_dir_all(&install_part_dir)?;
        }
        println!("Copying files...");
        dir::create_all(&install_part_dir, false)?;
        dir::copy(
            &get_app_dir_path(args),
            &install_part_dir,
            &CopyOptions::new().overwrite(true).content_only(true),
        )?;
        std::fs::rename(&install_part_dir, install_dir)?;
        drop(lock);
        println!("Copied!");
    }

    Ok(())
}
