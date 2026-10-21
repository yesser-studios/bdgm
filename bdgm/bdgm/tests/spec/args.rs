use crate::common::ManifestBuilder;
use bdgm::{error::BDGMError, game::ValidatedGame};

#[test]
fn args_not_required() {
    let manifest = ManifestBuilder::valid()
        .without("args")
        .without("runtime_args")
        .build();

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}

#[test]
fn args_json_array_allowed() {
    let manifest = ManifestBuilder::valid()
        .with("args", "[\"--help\", \"--verbose\"]")
        .with("runtime_args", "[\"--help\", \"--verbose\"]")
        .build();

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}

#[test]
fn args_json_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("args", "--help, --verbose")
        .with("runtime_args", "--help, --verbose")
        .build();

    let errs = ValidatedGame::validate_manifest(&manifest).unwrap_err().0;
    assert!(errs.iter().any(|x| matches!(x, BDGMError::ArgsInvalid(_))));
    assert!(
        errs.iter()
            .any(|x| matches!(x, BDGMError::RuntimeArgsInvalid(_)))
    );
}

#[test]
fn args_array_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("args", "{\"arg1\": \"--help\", \"arg2\": \"--verbose\"}")
        .with(
            "runtime_args",
            "{\"arg1\": \"--help\", \"arg2\": \"--verbose\"}",
        )
        .build();

    let errs = ValidatedGame::validate_manifest(&manifest).unwrap_err().0;
    assert!(errs.iter().any(|x| matches!(x, BDGMError::ArgsInvalid(_))));
    assert!(
        errs.iter()
            .any(|x| matches!(x, BDGMError::RuntimeArgsInvalid(_)))
    );
}

#[test]
fn args_string_elements_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("args", "[1]")
        .with("runtime_args", "[true]")
        .build();

    let errs = ValidatedGame::validate_manifest(&manifest).unwrap_err().0;
    assert!(errs.iter().any(|x| matches!(x, BDGMError::ArgsInvalid(_))));
    assert!(
        errs.iter()
            .any(|x| matches!(x, BDGMError::RuntimeArgsInvalid(_)))
    );
}

#[test]
fn empty_args_arrays_allowed() {
    let manifest = ManifestBuilder::valid()
        .with("args", "[]")
        .with("runtime_args", "[]")
        .build();

    assert!(ValidatedGame::validate_manifest(&manifest).is_ok())
}
