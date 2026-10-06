use std::path::PathBuf;

use bdgm::{error::BDGMError, game::ValidatedGame};
use platform_dirs::AppDirs;

use crate::{
    cli::args::Args,
    core::error::{
        AppError,
        IoError::{self},
    },
};

pub struct GameDirs {
    pub root: PathBuf,
    pub install: PathBuf,
    pub cache: PathBuf,
    pub data: PathBuf,
}

impl GameDirs {
    pub fn from(game: &ValidatedGame, app_dirs: &AppDirs) -> Self {
        let root = app_dirs.data_dir.join(game.id());
        let install = root.join("app").join(game.version());
        let cache = app_dirs.cache_dir.join(game.id());
        let data = root.join("data");

        GameDirs {
            root,
            install,
            cache,
            data,
        }
    }
}

pub fn get_app_dir_path(args: &Args) -> Option<PathBuf> {
    args.location.as_ref().map(|p| p.join("BDGM").join("APP"))
}

pub fn try_get_executable_path(args: &Args, game: &ValidatedGame) -> anyhow::Result<PathBuf> {
    let app_dir_path = match get_app_dir_path(args) {
        Some(p) => p,
        None => return Err(IoError::PathNone.into()),
    };

    let executable_path = app_dir_path.join(game.executable());

    if !executable_path.try_exists()? {
        return Err(AppError::InvalidGameFile(BDGMError::ExecutableMissing(
            executable_path.to_string_lossy().into_owned(),
        ))
        .into());
    }

    Ok(executable_path)
}

pub fn try_get_manifest_path(args: &Args) -> anyhow::Result<PathBuf> {
    let manifest_path = match args.location.as_ref() {
        Some(p) => p.join("BDGM").join("DISC.BDGM"),
        None => return Err(IoError::PathNone.into()),
    };

    if !manifest_path.try_exists()? {
        return Err(BDGMError::DiscFileMissing.into());
    }

    Ok(manifest_path)
}
