use bdgm::disc::{Entry, UdfRevision};
use clap::Parser;

use crate::args::Args;

pub(crate) fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let entry = Entry::scan_dir(args.source)?;
    entry.write_udf(
        args.output,
        if args.dvd {
            UdfRevision::V2_01
        } else {
            UdfRevision::V2_50
        },
    )
}
