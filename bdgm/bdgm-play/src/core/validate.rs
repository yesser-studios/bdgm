use std::{fs, path::PathBuf};

use bdgm::{error::BDGMError, game::ValidatedGame};

use super::{
    dirs::{get_disc_app_dir_path, get_disc_manifest_path},
    error::{AppError, Result},
    request::ExtractedSource,
};

pub(super) fn validate(source: &ExtractedSource) -> Result<ValidatedGame> {
    let manifest_path = get_disc_manifest_path(&source.path);
    let manifest = match fs::read_to_string(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::ValidationError(BDGMError::DiscFileMissing.into()));
        }
        Err(_) => {
            return Err(AppError::ValidationError(BDGMError::DiscFileInvalid.into()));
        }
    };

    let game = ValidatedGame::validate_manifest(&manifest).map_err(AppError::ValidationError)?;

    let app_path = get_disc_app_dir_path(&source.path);
    if !app_path.is_dir() {
        return Err(AppError::ValidationError(
            BDGMError::AppDirectoryMissing.into(),
        ));
    }

    if !executable_exists(&game, &app_path) {
        return Err(AppError::ValidationError(
            BDGMError::ExecutableMissing(game.executable().to_string_lossy().into_owned()).into(),
        ));
    }

    Ok(game)
}

#[allow(clippy::ptr_arg)]
pub(super) fn executable_exists(game: &ValidatedGame, app_path: &PathBuf) -> bool {
    app_path.join(game.executable()).is_file()
}
