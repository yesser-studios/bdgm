use std::{fs::File, io::Read};

use crate::{args::Args, dirs::try_get_manifest_path};
pub mod args;
pub mod dirs;
pub mod dump;
pub mod error;
pub mod fs;
pub mod image;
pub mod install;
pub mod launch;
pub mod server;

pub fn read_manifest(args: &Args) -> anyhow::Result<String> {
    let path = try_get_manifest_path(args)?;
    let mut manifest = File::open(path)?;
    let mut contents = String::new();
    manifest.read_to_string(&mut contents)?;
    Ok(contents)
}
