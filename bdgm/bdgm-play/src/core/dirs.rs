use std::{fs, io, path::Path, path::PathBuf, sync::Arc};

use bdgm::game::ValidatedGame;
use platform_dirs::AppDirs;

use super::error::{AppError, Result};

pub(super) struct GameDirs {
    app_dirs: Arc<AppDirs>,
    id: String,
    version: String,
    executable: PathBuf,
}

impl GameDirs {
    pub(super) fn from(game: &ValidatedGame, app_dirs: Arc<AppDirs>) -> Self {
        Self {
            app_dirs,
            id: game.id().to_string(),
            version: game.version().to_string(),
            executable: game.executable().clone(),
        }
    }

    pub(super) fn root(&self) -> PathBuf {
        self.app_dirs.data_dir.join("games").join(&self.id)
    }

    pub(super) fn install_dir(&self) -> PathBuf {
        self.root().join("app").join(&self.version)
    }

    pub(super) fn cache(&self) -> PathBuf {
        self.app_dirs.cache_dir.join("games").join(&self.id)
    }

    pub(super) fn data(&self) -> PathBuf {
        self.root().join("data")
    }

    pub(super) fn executable(&self) -> PathBuf {
        self.install_dir().join(&self.executable)
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }
}

pub(super) fn get_install_dir_path(id: &str, version: &str, app_dirs: &AppDirs) -> PathBuf {
    app_dirs
        .data_dir
        .join("games")
        .join(id)
        .join("app")
        .join(version)
}

pub(super) fn get_disc_manifest_path(disc_root: &Path) -> PathBuf {
    disc_root.join("BDGM").join("DISC.BDGM")
}

pub(super) fn get_disc_app_dir_path(disc_root: &Path) -> PathBuf {
    disc_root.join("BDGM").join("APP")
}

pub(super) fn get_app_dirs() -> Result<AppDirs> {
    AppDirs::new(Some("bdgm-play"), true).ok_or(AppError::NoAppDirs)
}

pub(super) fn create_game_dirs(game_dirs: &GameDirs) -> io::Result<()> {
    fs::create_dir_all(game_dirs.data())?;
    fs::create_dir_all(game_dirs.cache())?;

    if let Some(parent) = game_dirs.install_dir().parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(())
}
