use std::path::PathBuf;

use clap::Parser;

/// BDGM disc image builder
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The mounted root of the game disc. This is the parent of the BDGM directory.
    pub location: Option<PathBuf>,

    /// Whether the path points to an image instead of a mount point.
    #[arg(long)]
    pub image: bool,

    /// Whether the path points to a raw disc instead of a mount point.
    #[arg(long)]
    #[cfg(windows)]
    pub raw_disc: bool,

    /// The path to the runtime to use to run the game. Does nothing on Windows for Windows games.
    #[arg(long)]
    pub runtime: Option<PathBuf>,
}

impl Args {
    pub fn new_imageless(location: Option<PathBuf>, runtime: Option<PathBuf>) -> Self {
        #[cfg(windows)]
        {
            Args {
                location,
                image: false,
                raw_disc: false,
                runtime,
            }
        }
        #[cfg(not(windows))]
        {
            Args {
                location,
                image: false,
                runtime,
            }
        }
    }

    pub fn is_raw_disc(&self) -> bool {
        #[cfg(windows)]
        {
            return self.raw_disc;
        }

        #[allow(unreachable_code)]
        false
    }

    #[allow(unused)]
    pub fn set_raw_disc(&mut self, value: bool) -> Option<()> {
        #[cfg(windows)]
        {
            self.raw_disc = value;
            return Some(());
        }
        None
    }
}
