use std::{fs, io, sync::Arc};

use fs_extra::dir::{CopyOptions, copy as copy_dir};
use platform_dirs::AppDirs;

use super::{
    dirs::{GameDirs, create_game_dirs, get_disc_app_dir_path, get_install_dir_path},
    error::{InstallError, Result},
    fs::acquire_lock,
    request::ValidatedSource,
    state::StateHandler,
    validate::executable_exists,
};

pub(super) fn install(
    source: ValidatedSource,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<GameDirs> {
    let game_dirs = GameDirs::from(&source.game, Arc::new(app_dirs.clone()));
    create_game_dirs(&game_dirs).map_err(InstallError::IoError)?;

    let install_dir = game_dirs.install_dir();
    let part_dir = install_dir.with_added_extension("part");

    let _lock = acquire_lock(&install_dir, true, Some(on_state)).map_err(InstallError::IoError)?;

    if install_dir.exists() && executable_exists(&source.game, &install_dir) {
        return Ok(game_dirs);
    }

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(InstallError::IoError)?;
    }
    if part_dir.exists() {
        fs::remove_dir_all(&part_dir).map_err(InstallError::IoError)?;
    }

    let source_app_dir = get_disc_app_dir_path(&source.path);
    fs::create_dir_all(&part_dir).map_err(InstallError::IoError)?;
    copy_dir(
        &source_app_dir,
        &part_dir,
        &CopyOptions::new().overwrite(true).content_only(true),
    )
    .map_err(|error| InstallError::IoError(io::Error::other(error)))?;

    fs::rename(&part_dir, &install_dir).map_err(InstallError::IoError)?;

    Ok(game_dirs)
}

pub(super) fn check_installed(
    id: &str,
    version: &str,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<bool> {
    let install_dir = get_install_dir_path(id, version, app_dirs);

    let Some(parent) = install_dir.parent() else {
        return Ok(false);
    };
    if !parent.exists() {
        return Ok(false);
    }

    let _lock = acquire_lock(&install_dir, true, Some(on_state)).map_err(InstallError::IoError)?;

    Ok(install_dir.exists())
}
