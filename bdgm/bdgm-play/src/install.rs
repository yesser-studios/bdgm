use bdgm::game::ValidatedGame;
use fs_extra::dir::{self, CopyOptions};
use platform_dirs::AppDirs;

use crate::{args::Args, dirs::get_app_dir_path};

pub fn install(game: &ValidatedGame, app_dirs: &AppDirs, args: &Args) -> anyhow::Result<()> {
    let id = game.id();
    let game_dir = app_dirs.data_dir.join(&id);
    let install_dir = game_dir.join("app").join(game.version());
    if !install_dir.try_exists()? {
        println!("Copying files...");
        dir::create_all(&install_dir, false)?;
        dir::copy(
            &get_app_dir_path(args),
            &install_dir,
            &CopyOptions::new().overwrite(true).content_only(true),
        )?;
        println!("Copied!");
    }

    Ok(())
}
