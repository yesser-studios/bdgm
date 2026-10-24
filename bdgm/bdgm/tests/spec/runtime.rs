use bdgm::manifest_builder::ManifestBuilder;
use bdgm::{error::BDGMError, game::ValidatedGame};

#[test]
fn runtime_not_in_options_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("runtime", "bogus-should-fail")
        .build();

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
        .without("runtime_version")
        .build();

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}

#[test]
fn runtime_version_html_ignored() {
    let manifest = ManifestBuilder::valid()
        .with("runtime", "html")
        .with("runtime_version", "not-a-real-version")
        .build();

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}

#[test]
fn runtime_version_windows_not_required() {
    let manifest = ManifestBuilder::valid()
        .with("runtime", "windows")
        .without("runtime_version")
        .build();

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}
