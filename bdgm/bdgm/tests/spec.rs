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
        .without("name")
        .with_raw("name =Game")
        .build();

    assert_eq!(
        validate_manifest(manifest).unwrap_err(),
        BDGMErrors(vec![BDGMError::])
    )
}
