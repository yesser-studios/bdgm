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
    /// Derives installation and shared data paths from game metadata without creating directories.
    pub(super) fn from(game: &ValidatedGame, app_dirs: Arc<AppDirs>) -> Self {
        Self {
            app_dirs,
            id: game.id().to_string(),
            version: game.version().to_string(),
            executable: game.executable().clone(),
        }
    }

    /// Returns the game root at `<data_dir>/games/<id>`.
    pub(super) fn root(&self) -> PathBuf {
        self.app_dirs.data_dir.join("games").join(&self.id)
    }

    /// Returns the version-specific installation path beneath the game root.
    pub(super) fn install_dir(&self) -> PathBuf {
        self.root().join("app").join(&self.version)
    }

    /// Returns the cache path shared by all versions of this game.
    pub(super) fn cache(&self) -> PathBuf {
        self.app_dirs.cache_dir.join("games").join(&self.id)
    }

    /// Returns the persistent data path shared by all versions of this game.
    pub(super) fn data(&self) -> PathBuf {
        self.root().join("data")
    }

    /// Returns the manifest executable path within this version's installation.
    pub(super) fn executable(&self) -> PathBuf {
        self.install_dir().join(&self.executable)
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }
}

/// Builds an installation path from an already validated game ID and version.
pub(super) fn get_install_dir_path(id: &str, version: &str, app_dirs: &AppDirs) -> PathBuf {
    app_dirs
        .data_dir
        .join("games")
        .join(id)
        .join("app")
        .join(version)
}

/// Returns `BDGM/DISC.BDGM` beneath the disc root without checking existence.
pub(super) fn get_disc_manifest_path(disc_root: &Path) -> PathBuf {
    disc_root.join("BDGM").join("DISC.BDGM")
}

/// Returns `BDGM/APP` beneath the disc root without checking existence.
pub(super) fn get_disc_app_dir_path(disc_root: &Path) -> PathBuf {
    disc_root.join("BDGM").join("APP")
}

/// Resolves platform directories for bdgm-play, returning `NoAppDirs` if unavailable.
pub(super) fn get_app_dirs() -> Result<AppDirs> {
    AppDirs::new(Some("bdgm-play"), true).ok_or(AppError::NoAppDirs)
}

/// Creates shared data, cache, and installation parent directories.
///
/// The version-specific installation directory is not created. Returns the first
/// filesystem error; directories created before that error remain.
pub(super) fn create_game_dirs(game_dirs: &GameDirs) -> io::Result<()> {
    fs::create_dir_all(game_dirs.data())?;
    fs::create_dir_all(game_dirs.cache())?;

    if let Some(parent) = game_dirs.install_dir().parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::core::test_utils::{TestEnv, manifest, validated_game};

    fn test_game() -> ValidatedGame {
        validated_game(&manifest(
            "com.example.test",
            "1.2.3",
            "python",
            "sub/game.py",
            "[]",
            "[]",
        ))
    }

    #[test]
    fn game_dirs_follow_the_documented_tree() {
        let env = TestEnv::new();
        let game = test_game();
        let dirs = env.game_dirs(&game);

        let root = env.app_dirs.data_dir.join("games").join("com.example.test");
        assert_eq!(dirs.root(), root);
        assert_eq!(dirs.install_dir(), root.join("app").join("1.2.3"));
        assert_eq!(dirs.data(), root.join("data"));
        assert_eq!(
            dirs.cache(),
            env.app_dirs
                .cache_dir
                .join("games")
                .join("com.example.test")
        );
        assert_eq!(
            dirs.executable(),
            root.join("app").join("1.2.3").join("sub/game.py")
        );
        assert_eq!(dirs.id(), "com.example.test");
        assert_eq!(
            get_install_dir_path("com.example.test", "1.2.3", &env.app_dirs),
            dirs.install_dir()
        );
    }

    #[test]
    fn disc_paths_point_inside_the_bdgm_directory() {
        let root = Path::new("disc");

        assert_eq!(
            get_disc_manifest_path(root),
            root.join("BDGM").join("DISC.BDGM")
        );
        assert_eq!(get_disc_app_dir_path(root), root.join("BDGM").join("APP"));
    }

    #[test]
    fn create_game_dirs_creates_data_cache_and_install_parent() {
        let env = TestEnv::new();
        let dirs = env.game_dirs(&test_game());

        create_game_dirs(&dirs).expect("create game directories");

        assert!(dirs.data().is_dir());
        assert!(dirs.cache().is_dir());
        assert!(
            dirs.install_dir()
                .parent()
                .expect("install parent")
                .is_dir()
        );
        assert!(!dirs.install_dir().exists());
    }
}
