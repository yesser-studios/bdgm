use bdgm::error::ExecutableError;

use crate::common::{ManifestBuilder, validate_manifest};

#[test]
fn absolute_path_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("executable", "/bin/bash")
        .build();
    dbg!(&manifest);

    assert!(
        validate_manifest(manifest)
            .expect_err("Manifest should not be valid")
            .0
            .contains(&ExecutableError::IsAbsolute.into())
    )
}

#[test]
fn current_dir_symbol_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("executable", "./game.exe")
        .build();
    dbg!(&manifest);

    assert!(
        validate_manifest(manifest)
            .expect_err("Manifest should not be valid")
            .0
            .contains(&ExecutableError::ContainsCurrentDir.into())
    )
}

#[test]
fn parent_dir_symbol_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("executable", "../game.exe")
        .build();
    dbg!(&manifest);

    assert!(
        validate_manifest(manifest)
            .expect_err("Manifest should not be valid")
            .0
            .contains(&ExecutableError::ContainsParentDir.into())
    )
}

#[test]
fn backslash_separated_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("executable", "Game\\Game.exe")
        .build();
    dbg!(&manifest);

    assert!(
        validate_manifest(manifest)
            .expect_err("Manifest should not be valid")
            .0
            .contains(&ExecutableError::BackslashSeparated.into())
    )
}
