use std::{fs, path::Path, str::FromStr};

use bdgm::game::Game;

use crate::core::{dirs::get_disc_manifest_path, error::Result};

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
