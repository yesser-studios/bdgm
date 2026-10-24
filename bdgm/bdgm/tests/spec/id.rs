use bdgm::manifest_builder::ManifestBuilder;
use bdgm::{
    error::{BDGMErrors, IdFormatError},
    game::ValidatedGame,
};

fn errors_for_id(id: &str) -> BDGMErrors {
    let manifest = ManifestBuilder::valid().with("id", id).build();

    ValidatedGame::validate_manifest(&manifest).unwrap_err()
}

#[test]
fn valid_id_accepted() {
    let manifest = ManifestBuilder::valid()
        .with("id", "com.example.test-game")
        .build();

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}

#[test]
fn uppercase_forbidden() {
    assert!(
        errors_for_id("com.example.TestGame")
            .0
            .contains(&IdFormatError::ContainsUppercase.into())
    )
}

#[test]
fn underscores_forbidden() {
    assert!(
        errors_for_id("com.example.test_game")
            .0
            .contains(&IdFormatError::ContainsUnderscores.into())
    )
}

#[test]
fn whitespace_forbidden() {
    assert!(
        errors_for_id("com.example.test game")
            .0
            .contains(&IdFormatError::ContainsWhitespace.into())
    )
}

#[test]
fn forbidden_character_rejected() {
    assert!(
        errors_for_id("com.example.test*game")
            .0
            .contains(&IdFormatError::ContainsForbiddenCharacter('*').into())
    )
}

#[test]
fn too_few_components_rejected() {
    assert!(
        errors_for_id("com.example")
            .0
            .contains(&IdFormatError::IncorrectStructure.into())
    )
}

#[test]
fn empty_component_rejected() {
    assert!(
        errors_for_id("com..example")
            .0
            .contains(&IdFormatError::IncorrectStructure.into())
    )
}

#[test]
fn leading_dash_rejected() {
    assert!(
        errors_for_id("com.-example.test")
            .0
            .contains(&IdFormatError::IncorrectStructure.into())
    )
}

#[test]
fn trailing_dash_rejected() {
    assert!(
        errors_for_id("com.example-.test")
            .0
            .contains(&IdFormatError::IncorrectStructure.into())
    )
}

#[test]
fn double_dash_rejected() {
    assert!(
        errors_for_id("com.example.te--st")
            .0
            .contains(&IdFormatError::IncorrectStructure.into())
    )
}
