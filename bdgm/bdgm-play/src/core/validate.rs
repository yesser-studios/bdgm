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
        Err(e) => {
            return Err(AppError::IoError(e));
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

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use bdgm::error::ParserError;
    use tempfile::tempdir;

    use super::*;
    use crate::core::test_utils::{TestEnv, extracted, manifest, validated_game};

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

    #[test]
    fn validates_a_complete_disc() {
        let env = TestEnv::new();
        let disc = env.disc(&test_manifest(), "game.exe");

        let game = validate(&extracted(disc)).expect("validate disc");

        assert_eq!(game.id(), "com.example.test");
        assert_eq!(game.name(), "Test Game");
        assert_eq!(game.version(), "1.0.0");
        assert_eq!(game.executable().as_path(), Path::new("game.exe"));
    }

    #[test]
    fn rejects_a_disc_without_a_manifest() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");
        TestEnv::write_executable(&root, "game.exe");

        let error = validate(&extracted(root)).expect_err("manifest is missing");

        assert!(matches!(error, AppError::ValidationError(_)));
        assert!(error.to_string().contains("The DISC.BDGM file is missing!"));
    }

    #[test]
    fn rejects_an_invalid_manifest() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");
        TestEnv::write_manifest(&root, "not a manifest");
        TestEnv::write_executable(&root, "game.exe");

        let error = validate(&extracted(root)).expect_err("manifest is invalid");

        assert!(matches!(
            error,
            AppError::ValidationError(ref errors)
                if errors
                    .0
                    .contains(&BDGMError::UnableToParse(ParserError::InvalidHeader))
        ));
    }

    #[test]
    fn rejects_a_disc_without_an_app_directory() {
        let env = TestEnv::new();
        let root = env.root().join("disc");
        fs::create_dir_all(root.join("BDGM")).expect("create BDGM directory");
        TestEnv::write_manifest(&root, &test_manifest());

        let error = validate(&extracted(root)).expect_err("APP directory is missing");

        assert!(matches!(error, AppError::ValidationError(_)));
        assert!(error.to_string().contains("The APP directory is missing!"));
    }

    #[test]
    fn rejects_a_missing_executable() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");
        TestEnv::write_manifest(&root, &test_manifest());

        let error = validate(&extracted(root)).expect_err("executable is missing");

        assert!(matches!(error, AppError::ValidationError(_)));
        assert!(
            error
                .to_string()
                .contains("The executable BDGM/APP/game.exe is missing!")
        );
    }

    #[test]
    fn executable_exists_checks_for_a_regular_file() {
        let temp = tempdir().expect("create temporary directory");
        let app_path = temp.path().to_path_buf();
        let game = validated_game(&test_manifest());

        assert!(!executable_exists(&game, &app_path));

        fs::create_dir_all(app_path.join("game.exe")).expect("create directory instead of file");
        assert!(!executable_exists(&game, &app_path));

        fs::remove_dir_all(app_path.join("game.exe")).expect("remove directory");
        fs::write(app_path.join("game.exe"), b"executable").expect("write executable");
        assert!(executable_exists(&game, &app_path));
    }
}
