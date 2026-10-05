use std::{path::PathBuf, process::ExitStatus};

use bdgm_play::{args::Args, image::resolve_image_args, run_sanitized};

pub async fn run_gui(mut args: Args) -> anyhow::Result<ExitStatus> {
    resolve_image(&mut args, PathBuf::from("/hello.bin"));
    play(args).await
}

async fn play(args: Args) -> anyhow::Result<ExitStatus> {
    let (args, tempdir) = resolve_image_args(args)?;
    let result = run_sanitized(args).await;
    drop(tempdir);

    result
}

pub(crate) fn resolve_mounted_disc(args: &mut Args, path: PathBuf) {
    args.location = Some(path);
    args.image = false;
    args.set_raw_disc(false);
}

pub(crate) fn resolve_raw_disc(args: &mut Args, drive_letter: char) {
    let path = PathBuf::from(format!("\\\\.\\{drive_letter}:"));
    dbg!(&path);

    args.location = Some(path);
    args.set_raw_disc(true);
    args.image = false;
}

pub(crate) fn resolve_image(args: &mut Args, path: PathBuf) {
    args.location = Some(path);
    args.image = true;
    args.set_raw_disc(false);
}
