use bdgm::manifest_builder::ManifestBuilder;
use bdgm::{error::ExecutableError, game::ValidatedGame};

#[test]
fn absolute_path_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("executable", "/bin/bash")
        .build();

    assert!(
        ValidatedGame::validate_manifest(&manifest)
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

    assert!(
        ValidatedGame::validate_manifest(&manifest)
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

    assert!(
        ValidatedGame::validate_manifest(&manifest)
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

    assert!(
        ValidatedGame::validate_manifest(&manifest)
            .expect_err("Manifest should not be valid")
            .0
            .contains(&ExecutableError::BackslashSeparated.into())
    )
}

#[test]
fn nested_relative_path_allowed() {
    let manifest = ManifestBuilder::valid()
        .with("executable", "bin/game/ExampleGame.dll")
        .build();

    assert!(
        ValidatedGame::validate_manifest(&manifest).is_ok(),
        "a nested path relative to BDGM/APP should be accepted"
    )
}

#[test]
fn double_dot_inside_file_name_allowed() {
    let manifest = ManifestBuilder::valid()
        .with("executable", "game..exe")
        .build();

    assert!(
        ValidatedGame::validate_manifest(&manifest).is_ok(),
        "'.' only needs to be rejected as a path component"
    )
}
