use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use bdgm::game::ValidatedGame;
use platform_dirs::AppDirs;
use tempfile::{TempDir, tempdir};

use super::{
    dirs::GameDirs,
    request::{ExtractedSource, InstalledGame},
    state::{State, StateHandler},
};

pub(super) struct RecordingStateHandler {
    pub(super) states: Vec<State>,
    pub(super) reverts: usize,
}

impl RecordingStateHandler {
    pub(super) fn new() -> Self {
        Self {
            states: Vec::new(),
            reverts: 0,
        }
    }
}

impl StateHandler for RecordingStateHandler {
    fn change_state(&mut self, state: State) {
        self.states.push(state);
    }

    fn revert_state(&mut self) {
        self.reverts += 1;
    }
}

pub(super) struct TestEnv {
    temp: TempDir,
    pub(super) app_dirs: AppDirs,
}

impl TestEnv {
    pub(super) fn new() -> Self {
        let temp = tempdir().expect("create temporary directory");
        let root = temp.path();
        let app_dirs = AppDirs {
            cache_dir: root.join("cache"),
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            state_dir: root.join("state"),
        };

        Self { temp, app_dirs }
    }

    pub(super) fn root(&self) -> &Path {
        self.temp.path()
    }

    pub(super) fn disc_root(&self, name: &str) -> PathBuf {
        let root = self.temp.path().join(name);
        fs::create_dir_all(root.join("BDGM").join("APP")).expect("create disc directories");
        root
    }

    pub(super) fn write_manifest(root: &Path, manifest: &str) {
        fs::write(root.join("BDGM").join("DISC.BDGM"), manifest).expect("write disc manifest");
    }

    pub(super) fn write_executable(root: &Path, executable: &str) {
        let executable = root.join("BDGM").join("APP").join(executable);
        if let Some(parent) = executable.parent() {
            fs::create_dir_all(parent).expect("create executable parent directory");
        }
        fs::write(executable, b"test executable").expect("write disc executable");
    }

    pub(super) fn disc(&self, manifest: &str, executable: &str) -> PathBuf {
        let root = self.disc_root("disc");
        Self::write_manifest(&root, manifest);
        Self::write_executable(&root, executable);
        root
    }

    pub(super) fn game_dirs(&self, game: &ValidatedGame) -> GameDirs {
        GameDirs::from(game, Arc::new(self.app_dirs.clone()))
    }

    pub(super) fn installed(&self, game: ValidatedGame) -> InstalledGame {
        let game_dirs = self.game_dirs(&game);
        InstalledGame { game, game_dirs }
    }
}

pub(super) fn extracted(path: PathBuf) -> ExtractedSource {
    ExtractedSource {
        path,
        _tempdir: None,
    }
}

pub(super) fn manifest(
    id: &str,
    version: &str,
    runtime: &str,
    executable: &str,
    args: &str,
    runtime_args: &str,
) -> String {
    format!(
        "BDGM/1.1\nname=Test Game\nid={id}\nversion={version}\nexecutable={executable}\nruntime={runtime}\nruntime_version=1\nargs={args}\nruntime_args={runtime_args}\n"
    )
}

pub(super) fn validated_game(manifest: &str) -> ValidatedGame {
    ValidatedGame::validate_manifest(manifest).expect("test manifest should be valid")
}
