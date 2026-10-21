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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_source_and_output() {
        let args =
            Args::try_parse_from(["bdgm-build", "disc", "game.udf"]).expect("parse valid args");

        assert_eq!(args.source, PathBuf::from("disc"));
        assert_eq!(args.output, PathBuf::from("game.udf"));
        assert!(!args.dvd);
    }

    #[test]
    fn parses_dvd_flag() {
        let args = Args::try_parse_from(["bdgm-build", "--dvd", "disc", "game.udf"])
            .expect("parse args with --dvd");

        assert!(args.dvd);
    }

    #[test]
    fn requires_source_and_output() {
        assert!(Args::try_parse_from(["bdgm-build"]).is_err());
        assert!(Args::try_parse_from(["bdgm-build", "disc"]).is_err());
    }
}
