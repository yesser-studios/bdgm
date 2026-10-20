use std::{path::PathBuf, process::ExitStatus, time::Duration};

use crate::{
    cli::{args::Args, state::CliStateHandler},
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
    let mut state_handler = CliStateHandler::default();
    let (game, handle) = launch_request(request, &mut state_handler).await?;
    println!(
        "Playing: {} {} (ID: {})",
        game.name(),
        game.version(),
        game.id()
    );

    match handle {
        crate::core::LaunchHandle::GameProcess(game_process) => {
            let status = game_process.wait().await?;
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
