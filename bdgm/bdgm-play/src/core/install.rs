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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::core::{
        state::State,
        test_utils::{RecordingStateHandler, TestEnv, extracted, manifest, validated_game},
    };

    fn test_manifest() -> String {
        manifest(
            "com.example.test",
            "1.0.0",
            "windows",
            "game.exe",
            "[]",
            "[]",
        )
    }

    fn install_test_game(
        env: &TestEnv,
        manifest: &str,
        executable: &str,
        state: &mut RecordingStateHandler,
    ) -> GameDirs {
        let disc = env.disc(manifest, executable);
        let validated = extracted(disc).validate(state).expect("validate test disc");

        validated
            .install(&env.app_dirs, state)
            .expect("install test game")
            .game_dirs
    }

    #[test]
    fn installs_the_app_directory_and_creates_game_directories() {
        let env = TestEnv::new();
        let manifest = test_manifest();
        let mut state = RecordingStateHandler::new();

        let dirs = install_test_game(&env, &manifest, "game.exe", &mut state);

        assert!(dirs.install_dir().is_dir());
        assert!(dirs.executable().is_file());
        assert!(dirs.data().is_dir());
        assert!(dirs.cache().is_dir());
        assert!(state.states.iter().any(
            |state| matches!(state, State::Installing(game) if game.id() == "com.example.test")
        ));
    }

    #[test]
    fn check_installed_requires_the_install_parent() {
        let env = TestEnv::new();
        let game = validated_game(&test_manifest());
        let mut state = RecordingStateHandler::new();

        assert!(
            !check_installed(game.id(), game.version(), &env.app_dirs, &mut state)
                .expect("check missing installation")
        );

        let dirs = env.game_dirs(&game);
        fs::create_dir_all(dirs.install_dir()).expect("create install directory");

        assert!(
            check_installed(game.id(), game.version(), &env.app_dirs, &mut state)
                .expect("check installation")
        );
    }

    #[test]
    fn check_installed_does_not_require_an_executable() {
        let env = TestEnv::new();
        let game = validated_game(&test_manifest());
        let dirs = env.game_dirs(&game);
        fs::create_dir_all(dirs.install_dir()).expect("create empty install directory");
        let mut state = RecordingStateHandler::new();

        assert!(
            check_installed(game.id(), game.version(), &env.app_dirs, &mut state)
                .expect("check empty installation")
        );
    }

    #[test]
    fn replaces_an_incomplete_installation() {
        let env = TestEnv::new();
        let manifest = test_manifest();
        let mut state = RecordingStateHandler::new();
        let dirs = install_test_game(&env, &manifest, "game.exe", &mut state);
        let sentinel = dirs.install_dir().join("stale.txt");
        fs::write(&sentinel, b"stale").expect("write sentinel file");
        fs::remove_file(dirs.executable()).expect("remove installed executable");

        let reinstalled = install_test_game(&env, &manifest, "game.exe", &mut state);

        assert!(reinstalled.executable().is_file());
        assert!(!sentinel.exists());
    }

    #[test]
    fn removes_a_stale_part_directory() {
        let env = TestEnv::new();
        let manifest = test_manifest();
        let mut state = RecordingStateHandler::new();
        let dirs = install_test_game(&env, &manifest, "game.exe", &mut state);
        let part_dir = dirs.install_dir().with_added_extension("part");
        fs::create_dir_all(&part_dir).expect("create stale part directory");
        fs::write(part_dir.join("stale.txt"), b"stale").expect("write stale part file");
        fs::remove_file(dirs.executable()).expect("remove installed executable");

        let reinstalled = install_test_game(&env, &manifest, "game.exe", &mut state);

        assert!(reinstalled.executable().is_file());
        assert!(!part_dir.exists());
    }

    #[test]
    fn skips_installation_when_the_executable_already_exists() {
        let env = TestEnv::new();
        let manifest = test_manifest();
        let mut state = RecordingStateHandler::new();
        let dirs = install_test_game(&env, &manifest, "game.exe", &mut state);
        fs::write(dirs.executable(), b"changed").expect("change installed executable");

        let reinstalled = install_test_game(&env, &manifest, "game.exe", &mut state);

        assert_eq!(
            fs::read(reinstalled.executable()).expect("read executable"),
            b"changed"
        );
    }
}
