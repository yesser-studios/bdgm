use bdgm::error::{BDGMError, BDGMErrors, ParserError};

use crate::common::{ManifestBuilder, validate_manifest};

mod common;

#[test]
fn header_enforced() {
    let manifest = ManifestBuilder::valid().build_raw();

    assert_eq!(
        validate_manifest(manifest).unwrap_err(),
        BDGMErrors(vec![BDGMError::UnableToParse(ParserError::InvalidHeader)])
    );
}

#[test]
fn spaces_before_equals_forbidden() {
    let manifest = ManifestBuilder::valid()
        .with_raw("test =should_fail")
        .build();

    dbg!(&manifest);

    assert!(
        validate_manifest(manifest)
            .unwrap_err()
            .0
            .contains(&ParserError::WhitespaceInKey("test ".to_string()).into())
    )
}

#[ignore = "Bug: Leading and trailing whitespaces are not checked"]
#[test]
fn leading_trailing_whitespaces_forbidden() {
    let manifest = ManifestBuilder::valid()
        .with_raw(" test=should_fail ")
        .build();

    dbg!(&manifest);

    let errors = validate_manifest(manifest).unwrap_err();
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
    dbg!(&manifest);

    assert!(
        validate_manifest(manifest)
            .unwrap_err()
            .0
            .contains(&ParserError::EmptyValue("test".to_string()).into())
    )
}
