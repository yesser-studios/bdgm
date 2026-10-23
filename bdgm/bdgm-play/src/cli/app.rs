use std::{path::PathBuf, process::ExitStatus, time::Duration};

use crate::{
    args::Args,
    cli::state::CliStateHandler,
    core::{AppError, LaunchRequest, Source, SourceKind, launch_request},
};

pub async fn run(args: Args, extracted_location: PathBuf) -> Result<Option<ExitStatus>, AppError> {
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

    match handle {
        crate::core::LaunchHandle::GameProcess(game_process) => {
            let status = game_process.wait().await?;
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
        crate::core::LaunchHandle::HtmlServer(server) => {
            println!("Server running at {}", server.get_url());
            println!("Press Ctrl-C to stop.");
            loop {
                tokio::time::sleep(Duration::from_hours(24)).await
            }
        }
    }
}
