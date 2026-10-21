use bdgm::{
    error::{BDGMErrors, VersionFormatError},
    game::{Game, ValidatedGame},
};

fn game_with_version(version: &str) -> Game {
    Game {
        bdgm_version: Some("1.1".to_string()),
        name: Some("Test Game".to_string()),
        id: Some("com.example.test".to_string()),
        version: Some(version.to_string()),
        executable: Some("game.exe".to_string()),
        args: Vec::new(),
        runtime: Some("dotnet".to_string()),
        runtime_version: Some("10".to_string()),
        runtime_args: Vec::new(),
    }
}

fn errors_for_version(version: &str) -> BDGMErrors {
    ValidatedGame::validate(game_with_version(version)).unwrap_err()
}

#[test]
fn valid_version_accepted() {
    assert!(ValidatedGame::validate(game_with_version("1.0.0")).is_ok())
}

#[test]
fn forbidden_characters_rejected() {
    for character in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
        let version = format!("1.0.0{character}build");

        assert!(
            errors_for_version(&version)
                .0
                .contains(&VersionFormatError::ContainsForbiddenCharacter(character).into()),
            "the version should reject '{character}'"
        );
    }
}

#[test]
fn dot_and_dotdot_rejected() {
    for version in [".", ".."] {
        assert!(
            errors_for_version(version)
                .0
                .contains(&VersionFormatError::ForbiddenVersion(version.to_string()).into()),
            "the version {version} should be rejected"
        );
    }
}

#[test]
fn forbidden_sequences_rejected() {
    for sequence in [
        "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM9", "LPT0", "LPT9",
    ] {
        assert!(
            errors_for_version(sequence).0.contains(
                &VersionFormatError::ContainsForbiddenSequence(sequence.to_string()).into()
            ),
            "the version should reject '{sequence}'"
        );
    }
}

#[test]
fn forbidden_endings_rejected() {
    for ending in [".part", ".lock", ".", " "] {
        let version = format!("1.0.0{ending}");

        assert!(
            errors_for_version(&version).0.contains(
                &VersionFormatError::EndsWithForbiddenSequence(ending.to_string()).into()
            ),
            "the version should not end with '{ending}'"
        );
    }
}
