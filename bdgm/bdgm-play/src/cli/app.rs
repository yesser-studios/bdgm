use std::process::ExitStatus;

use crate::{
    cli::args::Args,
    core::{image::resolve_image_args, run_sanitized},
};

pub async fn run(args: Args) -> anyhow::Result<ExitStatus> {
    let (args, tempdir) = resolve_image_args(args)?;

    let result = run_sanitized(args, true).await;
    drop(tempdir);
    result
}
