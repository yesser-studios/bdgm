use crate::common::{ManifestBuilder, validate_manifest};

mod common;

#[test]
#[should_panic(expected = "Manifest could not")]
fn header_enforced() {
    let manifest = ManifestBuilder::valid().build_raw();

    validate_manifest(manifest).expect("Manifest could not be validated");
}
