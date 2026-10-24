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
    /// Creates a launch request with an optional runtime executable override.
    ///
    /// `None` selects the default runtime command. The override is ignored for HTML
    /// and for Windows games running natively on Windows; no paths are checked here.
    pub fn new(source: Source, runtime: Option<PathBuf>) -> Self {
        Self { source, runtime }
    }
}

pub struct Source {
    pub(super) path: PathBuf,
    pub(super) kind: SourceKind,
}

impl Source {
    /// Describes a disc root, image file, or raw device path without opening or validating it.
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
    /// Reports validation and checks the manifest, APP directory, and executable file.
    ///
    /// Preserves temporary extraction storage on success. Manifest I/O and validation
    /// errors propagate; a failed validation drops any owned temporary directory.
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
    /// Reports installation and installs or reuses the game, returning its metadata and paths.
    ///
    /// Propagates installation errors and releases any temporary extraction storage.
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::test_utils::{RecordingStateHandler, TestEnv, extracted, manifest};

    #[test]
    fn constructors_store_the_source_and_runtime() {
        let request = LaunchRequest::new(
            Source::new(PathBuf::from("disc"), SourceKind::MountedDisc),
            Some(PathBuf::from("runtime")),
        );

        assert_eq!(request.source.path, PathBuf::from("disc"));
        assert!(matches!(request.source.kind, SourceKind::MountedDisc));
        assert_eq!(request.runtime, Some(PathBuf::from("runtime")));
    }

    #[test]
    fn validate_and_install_emit_states_and_install_the_game() {
        let env = TestEnv::new();
        let disc = env.disc(
            &manifest(
                "com.example.test",
                "1.0.0",
                "windows",
                "game.exe",
                "[]",
                "[]",
            ),
            "game.exe",
        );
        let mut state = RecordingStateHandler::new();

        let validated = extracted(disc.clone())
            .validate(&mut state)
            .expect("validate extracted source");

        assert_eq!(validated.path, disc);
        assert_eq!(validated.game.id(), "com.example.test");
        assert!(matches!(state.states.first(), Some(State::Validating)));

        let installed = validated
            .install(&env.app_dirs, &mut state)
            .expect("install validated source");

        assert!(installed.game_dirs.executable().is_file());
        assert!(state.states.iter().any(
            |state| matches!(state, State::Installing(game) if game.id() == "com.example.test")
        ));
    }
}
