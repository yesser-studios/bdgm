use std::path::PathBuf;

use bdgm::game::ValidatedGame;
use platform_dirs::AppDirs;
use tempfile::TempDir;

use super::{
    dirs::GameDirs,
    error::Result,
    install,
    state::{State, StateHandler},
    validate,
};

pub struct LaunchRequest {
    pub(super) source: Source,
    pub(super) runtime: Option<PathBuf>,
}

impl LaunchRequest {
    pub fn new(source: Source, runtime: Option<PathBuf>) -> Self {
        Self { source, runtime }
    }
}

pub struct Source {
    pub(super) path: PathBuf,
    pub(super) kind: SourceKind,
}

impl Source {
    pub fn new(path: PathBuf, kind: SourceKind) -> Self {
        Self { path, kind }
    }
}

pub enum SourceKind {
    MountedDisc,
    RawDisc,
    Image,
}

#[allow(clippy::large_enum_variant)]
pub(super) enum ResolvedSource {
    Extracted(ExtractedSource),
    Installed(InstalledGame),
}

pub(super) struct ExtractedSource {
    pub(super) path: PathBuf,
    pub(super) _tempdir: Option<TempDir>,
}

impl ExtractedSource {
    pub(super) fn validate(self, on_state: &mut dyn StateHandler) -> Result<ValidatedSource> {
        on_state.change_state(State::Validating);
        let game = validate::validate(&self)?;

        Ok(ValidatedSource {
            game,
            path: self.path,
            _tempdir: self._tempdir,
        })
    }
}

pub(super) struct ValidatedSource {
    pub(super) game: ValidatedGame,
    pub(super) path: PathBuf,
    _tempdir: Option<TempDir>,
}

impl ValidatedSource {
    pub(super) fn install(
        self,
        app_dirs: &AppDirs,
        on_state: &mut dyn StateHandler,
    ) -> Result<InstalledGame> {
        on_state.change_state(State::Installing(self.game.clone()));

        let game = self.game.clone();
        let game_dirs = install::install(self, app_dirs, on_state)?;

        Ok(InstalledGame { game, game_dirs })
    }
}

pub(super) struct InstalledGame {
    pub(super) game: ValidatedGame,
    pub(super) game_dirs: GameDirs,
}
