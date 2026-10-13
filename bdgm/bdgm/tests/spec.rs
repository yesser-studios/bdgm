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

#[ignore = "Known bug: Parser skips keys with whitespace instead of failing"]
#[test]
fn spaces_before_equals_forbidden() {
    let manifest = ManifestBuilder::valid()
        .with_raw("test =should_fail")
        .build();

    dbg!(&manifest);

    assert_eq!(
        validate_manifest(manifest).unwrap_err(),
        BDGMErrors(vec![BDGMError::UnableToParse(
            ParserError::WhitespaceInKey("test ".to_string())
        )])
    )
}
