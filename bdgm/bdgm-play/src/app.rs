use std::process::ExitStatus;

use bdgm_play::{args::Args, image::resolve_image_args, run_sanitized};

pub(crate) async fn run(args: Args) -> anyhow::Result<ExitStatus> {
    let (args, tempdir) = resolve_image_args(args)?;

    let result = run_sanitized(args).await;
    drop(tempdir);
    result
}
