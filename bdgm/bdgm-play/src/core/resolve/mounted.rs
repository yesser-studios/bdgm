use std::{fs, path::Path, str::FromStr};

use bdgm::game::Game;

use crate::core::{dirs::get_disc_manifest_path, error::Result};

/// Probes `BDGM/DISC.BDGM` without validating its field values.
///
/// All read and parse failures become `Ok(None)` so callers can fall back to
/// full source validation.
pub(super) fn get_mounted_manifest(disc_root: &Path) -> Result<Option<Game>> {
    let manifest_path = get_disc_manifest_path(disc_root);
    let manifest = match fs::read_to_string(&manifest_path) {
        Ok(manifest) => manifest,
        Err(_) => return Ok(None),
    };

    match Game::from_str(&manifest) {
        Ok(game) => Ok(Some(game)),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_utils::{TestEnv, manifest};

    #[test]
    fn reads_a_manifest_from_a_mounted_disc() {
        let env = TestEnv::new();
        let disc = env.disc(
            &manifest(
                "com.example.test",
                "1.0.0",
                "windows",
                "game.exe",
                "[]",
                "[]",
            ),
            "game.exe",
        );

        let game = get_mounted_manifest(&disc)
            .expect("read mounted manifest")
            .expect("manifest should exist");

        assert_eq!(game.id.as_deref(), Some("com.example.test"));
        assert_eq!(game.version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn returns_none_when_the_manifest_is_missing_or_invalid() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");

        assert!(
            get_mounted_manifest(&root)
                .expect("read missing manifest")
                .is_none()
        );

        TestEnv::write_manifest(&root, "not a manifest");
        assert!(
            get_mounted_manifest(&root)
                .expect("read invalid manifest")
                .is_none()
        );
    }

    #[test]
    fn returns_a_parseable_manifest_even_when_validation_would_fail() {
        let env = TestEnv::new();
        let root = env.disc_root("disc");
        TestEnv::write_manifest(
            &root,
            "BDGM/1.1\nid=com.example.test\nversion=1.0.0\nexecutable=game.exe\nruntime=windows\n",
        );

        let game = get_mounted_manifest(&root)
            .expect("read mounted manifest")
            .expect("manifest should parse");

        assert_eq!(game.id.as_deref(), Some("com.example.test"));
        assert!(game.name.is_none());
    }
}
