use std::path::PathBuf;

use clap::Parser;

/// BDGM disc image builder
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Args {
    /// The mounted root of the game disc. This is the parent of the BDGM directory.
    pub location: PathBuf,

    /// Whether the path points to an image instead of a mount point.
    #[arg(long)]
    pub image: bool,

    /// Whether the path points to a raw disc instead of a mount point.
    #[arg(long)]
    pub raw_disc: bool,
}
