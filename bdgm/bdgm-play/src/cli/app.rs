use std::{path::PathBuf, process::ExitStatus};

use bdgm::runtime::Runtime;

use crate::{
    args::Args,
    cli::state::CliStateHandler,
    core::{
        AppError, LaunchError, LaunchHandle, LaunchRequest, Source, SourceKind, launch_request,
    },
};

pub async fn run(args: Args, extracted_location: PathBuf) -> Result<Option<ExitStatus>, AppError> {
    match try_run(args, extracted_location).await {
        Ok(status) => Ok(status),
        Err(e) => {
            eprintln!("{e}");
            if let AppError::LaunchError(LaunchError::RuntimeNotFound(game, _)) = &e
                && (*game.runtime() != Runtime::Windows || cfg!(not(windows)))
            {
                eprintln!(
                    "Hint: specifying a runtime with `--runtime /path/to/runtime` may fix your issue."
                );
            }
            Err(e)
        }
    }
}

async fn try_run(args: Args, extracted_location: PathBuf) -> Result<Option<ExitStatus>, AppError> {
    let source = Source::new(extracted_location, {
        if !args.is_raw_disc() && !args.image {
            SourceKind::MountedDisc
        } else if args.is_raw_disc() {
            SourceKind::RawDisc
        } else {
            SourceKind::Image
        }
    });
    let request = LaunchRequest::new(source, args.runtime);
    let state_handler = CliStateHandler::default();
    let (game, handle) = launch_request(request, state_handler).await?;
    println!(
        "Playing: {} {} (ID: {})",
        game.name(),
        game.version(),
        game.id()
    );

    if let LaunchHandle::HtmlServer(server) = &handle {
        println!("Server running at {}", server.get_url());
        println!("Press Ctrl-C to stop.");
    }

    let status = handle.wait().await?;
    if let Some(status) = status {
        if let Some(code) = status.code()
            && code != 0
        {
            eprintln!("{} exited with code {code}!", game.name());
        } else if status.code().is_none() {
            eprintln!("{} exited with no code.", game.name());
        }
    }

    Ok(status)
}
