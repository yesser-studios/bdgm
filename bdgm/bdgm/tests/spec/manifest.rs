use crate::common::ManifestBuilder;
use bdgm::{
    error::{BDGMError, BDGMErrors, ParserError},
    game::ValidatedGame,
};

#[test]
fn invalid_header_rejected() {
    let manifest = "NOTBDGM/1.1\n".to_string() + &ManifestBuilder::valid().build_raw();

    assert_eq!(
        ValidatedGame::validate_manifest(&manifest).unwrap_err(),
        BDGMErrors(vec![BDGMError::UnableToParse(ParserError::InvalidHeader)])
    );
}

#[test]
fn missing_header_rejected() {
    let manifest = ManifestBuilder::valid().build_raw();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&ParserError::InvalidHeader.into())
    );
}

#[test]
fn empty_manifest_rejected() {
    assert!(
        ValidatedGame::validate_manifest("")
            .unwrap_err()
            .0
            .contains(&BDGMError::MissingHeaderVersion)
    );
}

#[test]
fn missing_header_version_rejected() {
    let manifest = "BDGM/\n".to_string() + &ManifestBuilder::valid().build_raw();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&BDGMError::MissingHeaderVersion)
    );
}

#[test]
fn unsupported_header_version_rejected() {
    let manifest = "BDGM/9.9\n".to_string() + &ManifestBuilder::valid().build_raw();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&BDGMError::UnsupportedHeaderVersion("9.9".to_string()))
    );
}

#[test]
fn spaces_before_equals_forbidden() {
    let manifest = ManifestBuilder::valid()
        .with_raw("test =should_fail")
        .build();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&ParserError::WhitespaceInKey("test ".to_string()).into())
    )
}

#[test]
fn leading_trailing_whitespaces_forbidden() {
    let manifest = ManifestBuilder::valid()
        .with_raw(" test=should_fail ")
        .build();

    let errors = ValidatedGame::validate_manifest(&manifest).unwrap_err();
    assert!(
        errors
            .0
            .contains(&ParserError::LeadingWhitespace(" test=should_fail ".to_string()).into())
    );
    assert!(
        errors
            .0
            .contains(&ParserError::TrailingWhitespace(" test=should_fail ".to_string()).into())
    );
}

#[test]
fn empty_value_forbidden() {
    let manifest = ManifestBuilder::valid().with_raw("test=").build();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&ParserError::EmptyValue("test".to_string()).into())
    )
}

#[test]
fn duplicate_keys_forbidden() {
    let manifest = ManifestBuilder::valid().with_raw("name=Game").build();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&ParserError::DuplicateKey("name".to_string()).into())
    )
}

#[test]
fn equals_in_value_allowed() {
    let manifest = ManifestBuilder::valid().with("name", "Game=1=2").build();

    assert_eq!(
        "Game=1=2",
        ValidatedGame::validate_manifest(&manifest).unwrap().name()
    );
}

#[test]
fn test_with_empty_line_valid() {
    let manifest = ManifestBuilder::valid()
        .with_raw("")
        .with_raw("unknown=should_succeed")
        .build();

    assert!(
        manifest.contains("\n\n"),
        "the manifest should contain an actual empty line"
    );
    ValidatedGame::validate_manifest(&manifest)
        .expect("Manifest with an empty line could not be validated.");
}

#[test]
fn test_line_without_equals_valid() {
    let manifest = ManifestBuilder::valid().with_raw("Should succeed.").build();

    ValidatedGame::validate_manifest(&manifest)
        .expect("Manifest with a line without equals could not be validated.");
}

#[test]
fn test_line_starting_hash_valid() {
    let manifest = ManifestBuilder::valid()
        .with_raw("# Should succeed.")
        .build();

    ValidatedGame::validate_manifest(&manifest)
        .expect("Manifest with a comment could not be validated.");
}

#[test]
fn test_unknown_key_valid() {
    let manifest = ManifestBuilder::valid()
        .with("unknown", "should_succeed")
        .build();

    ValidatedGame::validate_manifest(&manifest)
        .expect("Manifest with unknown key could not be validated.");
}

#[test]
fn test_missing_key_enforced() {
    let manifest = ManifestBuilder::valid().without("name").build();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&BDGMError::MandatoryFieldMissing("name".to_string()))
    )
}

#[test]
fn all_mandatory_keys_enforced() {
    for key in ["name", "id", "version", "executable", "runtime"] {
        let manifest = ManifestBuilder::valid().without(key).build();
        let errors = ValidatedGame::validate_manifest(&manifest).unwrap_err();

        assert!(
            errors
                .0
                .contains(&BDGMError::MandatoryFieldMissing(key.to_string())),
            "removing {key} should produce a missing-field error"
        );
    }
}
