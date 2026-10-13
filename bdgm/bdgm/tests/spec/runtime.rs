use crate::common::ManifestBuilder;
use bdgm::{error::BDGMError, game::ValidatedGame};

#[test]
fn runtime_not_in_options_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("runtime", "bogus-should-fail")
        .build();
    dbg!(&manifest);

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&BDGMError::RuntimeInvalid("bogus-should-fail".to_string()))
    )
}

#[test]
fn runtime_version_missing_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("runtime", "dotnet")
        .without("runtime_version")
        .build();
    dbg!(&manifest);

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .unwrap_err()
            .0
            .contains(&BDGMError::MandatoryFieldMissing(
                "runtime_version".to_string()
            ))
    )
}

#[test]
fn runtime_version_html_not_required() {
    let manifest = ManifestBuilder::valid()
        .with("runtime", "html")
        .with("runtime_version", "5")
        .build();
    dbg!(&manifest);

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}

#[test]
fn runtime_version_html_windows_non_mandatory() {
    let manifest = ManifestBuilder::valid()
        .with("runtime", "html")
        .without("runtime_version")
        .build();
    dbg!(&manifest);

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok());

    let manifest = ManifestBuilder::valid()
        .with("runtime", "windows")
        .without("runtime_version")
        .build();
    dbg!(&manifest);

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok());
}
