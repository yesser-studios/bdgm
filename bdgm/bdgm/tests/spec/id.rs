use crate::common::ManifestBuilder;
use bdgm::game::ValidatedGame;

#[test]
fn lowercase_ascii_enforced() {
    let manifest = ManifestBuilder::valid()
        .with("id", "com.*example domain.Test_Game")
        .build();

    let errs = ValidatedGame::validate_manifest(&manifest).unwrap_err().0;
    assert!(
        errs.contains(&bdgm::error::IdFormatError::ContainsUppercase.into())
            && errs.contains(&bdgm::error::IdFormatError::ContainsForbiddenCharacter('*').into())
            && errs.contains(&bdgm::error::IdFormatError::ContainsUnderscores.into())
            && errs.contains(&bdgm::error::IdFormatError::ContainsWhitespace.into())
    );
}
