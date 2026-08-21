use std::path::PathBuf;

use clap::Parser;

/// BDGM disc image builder
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Args {
    /// The source directory to copy to the image
    pub source: PathBuf,
    /// The target .udf image file
    pub output: PathBuf,
    /// Whether to create a UDF 2.01 image for CDs and DVDs instead of UDF 2.50
    #[arg(long)]
    pub dvd: bool,
}
