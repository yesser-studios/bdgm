use crate::common::{ManifestBuilder, validate_manifest};
use bdgm::error::BDGMError;

#[test]
fn args_not_required() {
    let manifest = ManifestBuilder::valid()
        .without("args")
        .without("runtime_args")
        .build();
    dbg!(&manifest);

    assert!(validate_manifest(manifest).is_ok())
}

#[test]
fn args_json_array_allowed() {
    let manifest = ManifestBuilder::valid()
        .with("args", "[\"--help\", \"--verbose\"]")
        .with("runtime_args", "[\"--help\", \"--verbose\"]")
        .build();
    dbg!(&manifest);

    let result = validate_manifest(manifest);
    dbg!(&result);
    assert!(result.is_ok())
}

#[test]
fn args_json_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("args", "--help, --verbose")
        .with("runtime_args", "--help, --verbose")
        .build();
    dbg!(&manifest);

    let errs = validate_manifest(manifest).unwrap_err().0;
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
    dbg!(&manifest);

    let errs = validate_manifest(manifest).unwrap_err().0;
    assert!(errs.iter().any(|x| matches!(x, BDGMError::ArgsInvalid(_))));
    assert!(
        errs.iter()
            .any(|x| matches!(x, BDGMError::RuntimeArgsInvalid(_)))
    );
}
