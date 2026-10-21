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
        if let Some(installed) = installed_and_available(game, app_dirs, on_state)? {
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
            if let Some(installed) = installed_and_available(game, app_dirs, on_state)? {
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
        if let Some(installed) = installed_and_available(game, app_dirs, on_state)? {
            return Ok(installed);
        }
    }

    Ok(ResolvedSource::Extracted(ExtractedSource {
        path: path.to_path_buf(),
        _tempdir: None,
    }))
}

fn installed_and_available(
    game: ValidatedGame,
    app_dirs: &AppDirs,
    on_state: &mut dyn StateHandler,
) -> Result<Option<ResolvedSource>> {
    let game_dirs = GameDirs::from(&game, Arc::new(app_dirs.clone()));
    let installed = install::check_installed(game.id(), game.version(), app_dirs, on_state)?
        && validate::executable_exists(&game, &game_dirs.install_dir());

    Ok(installed.then(|| ResolvedSource::Installed(InstalledGame { game, game_dirs })))
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(not(windows))]
    use std::path::PathBuf;

    use bdgm::disc::{Entry, UdfRevision};

    use super::*;
    use crate::core::test_utils::{RecordingStateHandler, TestEnv, extracted, manifest};

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

    fn install_game(
        env: &TestEnv,
        disc: &Path,
        state: &mut RecordingStateHandler,
    ) -> ValidatedGame {
        let validated = extracted(disc.to_path_buf())
            .validate(state)
            .expect("validate test disc");

        validated
            .install(&env.app_dirs, state)
            .expect("install test game")
            .game
    }

    #[test]
    fn mounted_disc_returns_an_extracted_source_when_not_installed() {
        let env = TestEnv::new();
        let disc = env.disc(&test_manifest(), "game.exe");
        let mut state = RecordingStateHandler::new();

        let resolved = resolve(
            Source::new(disc.clone(), SourceKind::MountedDisc),
            &env.app_dirs,
            &mut state,
        )
        .expect("resolve mounted disc");

        match resolved {
            ResolvedSource::Extracted(source) => assert_eq!(source.path, disc),
            ResolvedSource::Installed(_) => panic!("game should not be installed"),
        }
        assert!(matches!(
            state.states.first(),
            Some(State::Extracting(None))
        ));
        assert!(
            state
                .states
                .iter()
                .all(|state| matches!(state, State::Extracting(None))),
            "resolving a mounted disc should only report extraction states"
        );
    }

    #[test]
    fn mounted_disc_returns_the_installation_when_available() {
        let env = TestEnv::new();
        let disc = env.disc(&test_manifest(), "game.exe");
        let mut install_state = RecordingStateHandler::new();
        let game = install_game(&env, &disc, &mut install_state);
        let mut state = RecordingStateHandler::new();

        let resolved = resolve(
            Source::new(disc, SourceKind::MountedDisc),
            &env.app_dirs,
            &mut state,
        )
        .expect("resolve mounted disc");

        match resolved {
            ResolvedSource::Installed(installed) => {
                assert_eq!(installed.game.id(), game.id());
                assert_eq!(installed.game.version(), game.version());
                assert!(installed.game_dirs.executable().is_file());
            }
            ResolvedSource::Extracted(_) => panic!("game should be installed"),
        }
        assert!(matches!(
            state.states.first(),
            Some(State::Extracting(None))
        ));
    }

    #[test]
    fn mounted_disc_falls_back_when_the_manifest_cannot_be_parsed() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");
        TestEnv::write_manifest(&root, "not a manifest");
        TestEnv::write_executable(&root, "game.exe");
        let mut state = RecordingStateHandler::new();

        let resolved = resolve(
            Source::new(root.clone(), SourceKind::MountedDisc),
            &env.app_dirs,
            &mut state,
        )
        .expect("resolve mounted disc");

        match resolved {
            ResolvedSource::Extracted(source) => assert_eq!(source.path, root),
            ResolvedSource::Installed(_) => panic!("invalid manifest cannot be installed"),
        }
    }

    #[test]
    fn mounted_disc_falls_back_when_the_manifest_is_missing() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");
        TestEnv::write_executable(&root, "game.exe");
        let mut state = RecordingStateHandler::new();

        let resolved = resolve(
            Source::new(root.clone(), SourceKind::MountedDisc),
            &env.app_dirs,
            &mut state,
        )
        .expect("resolve mounted disc");

        match resolved {
            ResolvedSource::Extracted(source) => assert_eq!(source.path, root),
            ResolvedSource::Installed(_) => panic!("missing manifest cannot be installed"),
        }
    }

    #[test]
    fn mounted_disc_rejects_a_parseable_but_invalid_manifest() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");
        TestEnv::write_manifest(
            &root,
            "BDGM/1.1\nid=com.example.test\nversion=1.0.0\nexecutable=game.exe\nruntime=windows\n",
        );
        TestEnv::write_executable(&root, "game.exe");
        let mut state = RecordingStateHandler::new();

        let result = resolve(
            Source::new(root, SourceKind::MountedDisc),
            &env.app_dirs,
            &mut state,
        );

        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }

    #[test]
    fn image_source_extracts_a_valid_disc() {
        let env = TestEnv::new();
        let disc = env.disc(&test_manifest(), "game.exe");
        let image = env.root().join("game.udf");
        let entry = Entry::scan_dir(&disc).expect("scan test disc");
        entry
            .write_udf(image.clone(), UdfRevision::V2_50)
            .expect("write UDF image");

        let mut state = RecordingStateHandler::new();
        let resolved = resolve(
            Source::new(image, SourceKind::Image),
            &env.app_dirs,
            &mut state,
        )
        .expect("resolve image");

        match resolved {
            ResolvedSource::Extracted(source) => {
                assert!(source.path.join("BDGM").join("DISC.BDGM").is_file());
                assert!(
                    source
                        .path
                        .join("BDGM")
                        .join("APP")
                        .join("game.exe")
                        .is_file()
                );

                let game =
                    crate::core::validate::validate(&source).expect("validate the extracted image");
                assert_eq!(game.id(), "com.example.test");
            }
            ResolvedSource::Installed(_) => panic!("game should not be installed"),
        }
    }

    #[test]
    fn image_source_rejects_non_udf_data() {
        let env = TestEnv::new();
        let path = env.root().join("not-udf.img");
        fs::write(&path, b"not a udf image").expect("write invalid image");
        let mut state = RecordingStateHandler::new();

        let result = resolve(
            Source::new(path, SourceKind::Image),
            &env.app_dirs,
            &mut state,
        );

        assert!(matches!(result, Err(AppError::ExtractError(_))));
    }

    #[cfg(not(windows))]
    #[test]
    fn raw_disc_source_is_unsupported_outside_windows() {
        let env = TestEnv::new();
        let mut state = RecordingStateHandler::new();

        let result = resolve(
            Source::new(PathBuf::from("ignored"), SourceKind::RawDisc),
            &env.app_dirs,
            &mut state,
        );

        assert!(matches!(
            result,
            Err(AppError::UnsupportedOnThisPlatform(
                UnsupportedOperation::ReadingRawDiscs
            ))
        ));
    }
}
