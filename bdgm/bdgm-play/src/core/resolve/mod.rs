use std::{path::Path, sync::Arc};

use bdgm::game::ValidatedGame;
use platform_dirs::AppDirs;

use super::{
    dirs::GameDirs,
    error::{AppError, ExtractError, Result, UnsupportedOperation},
    install,
    request::{ExtractedSource, InstalledGame, ResolvedSource, Source, SourceKind},
    state::{State, StateHandler},
    validate,
};

mod mounted;
#[cfg(windows)]
mod raw_disc;
mod udf;

pub(super) fn resolve(
    source: Source,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<ResolvedSource> {
    on_state.change_state(State::Extracting(None));

    let Source { path, kind } = source;
    match kind {
        SourceKind::MountedDisc => resolve_mounted(&path, app_dirs, on_state),
        SourceKind::Image => resolve_image(&path, app_dirs, on_state),
        SourceKind::RawDisc => resolve_raw_disc(&path, app_dirs, on_state),
    }
}

fn resolve_image(
    path: &Path,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<ResolvedSource> {
    let file = std::fs::File::open(path).map_err(ExtractError::IoError)?;
    let udf = udf::open_udf_volume(file)?;

    if let Ok(Some(game)) = udf::get_udf_manifest(&udf) {
        let game = ValidatedGame::validate(game).map_err(AppError::ValidationError)?;
        if let Some(installed) = installed_if_available(game, app_dirs, on_state)? {
            return Ok(installed);
        }
    }

    Ok(ResolvedSource::Extracted(udf::extract(&udf, on_state)?))
}

fn resolve_raw_disc(
    path: &Path,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<ResolvedSource> {
    if !cfg!(windows) {
        return Err(AppError::UnsupportedOnThisPlatform(
            UnsupportedOperation::ReadingRawDiscs,
        ));
    }

    #[cfg(windows)]
    {
        let reader = raw_disc::RawDiscReader::open(path)?;
        let udf = udf::open_udf_volume(reader)?;

        if let Ok(Some(game)) = udf::get_udf_manifest(&udf) {
            let game = ValidatedGame::validate(game).map_err(AppError::ValidationError)?;
            if let Some(installed) = installed_if_available(game, app_dirs, on_state)? {
                return Ok(installed);
            }
        }

        Ok(ResolvedSource::Extracted(udf::extract(&udf, on_state)?))
    }

    #[cfg(not(windows))]
    {
        let _ = (path, app_dirs, on_state);
        unreachable!("the platform check above returned on non-Windows")
    }
}

fn resolve_mounted(
    path: &Path,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<ResolvedSource> {
    if let Some(game) = mounted::get_mounted_manifest(path)? {
        let game = ValidatedGame::validate(game).map_err(AppError::ValidationError)?;
        if let Some(installed) = installed_if_available(game, app_dirs, on_state)? {
            return Ok(installed);
        }
    }

    Ok(ResolvedSource::Extracted(ExtractedSource {
        path: path.to_path_buf(),
        _tempdir: None,
    }))
}

fn installed_if_available(
    game: ValidatedGame,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<Option<ResolvedSource>> {
    let game_dirs = GameDirs::from(&game, Arc::new(app_dirs.clone()));
    let installed = install::check_installed(game.id(), game.version(), app_dirs, on_state)?
        && validate::executable_exists(&game, &game_dirs.install_dir());

    Ok(installed.then(|| ResolvedSource::Installed(InstalledGame { game, game_dirs })))
}
