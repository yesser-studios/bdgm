use std::{path::Path, sync::Arc};

use bdgm::{game::ValidatedGame, runtime::Runtime};

use super::{
    dirs::{create_game_dirs, get_app_dirs},
    error::{InstallError, Result},
    request::{InstalledGame, LaunchRequest, ResolvedSource},
    resolve::resolve,
    state::{State, StateHandler},
};

mod handle;
mod process;
mod server;

pub use handle::LaunchHandle;
pub use process::GameProcess;
pub use server::HtmlServer;

pub async fn launch_request(
    request: LaunchRequest,
    on_state: &mut dyn StateHandler,
) -> Result<(ValidatedGame, LaunchHandle)> {
    let LaunchRequest { source, runtime } = request;
    let app_dirs = Arc::new(get_app_dirs()?);
    let resolved = resolve(source, &app_dirs, on_state)?;

    let installed = match resolved {
        ResolvedSource::Extracted(source) => {
            let validated = source.validate(on_state)?;
            validated.install(&app_dirs, on_state)?
        }
        ResolvedSource::Installed(installed) => installed,
    };

    let game = installed.game.clone();
    let handle = run_game(&installed, runtime.as_deref(), on_state).await?;

    Ok((game, handle))
}

async fn run_game(
    installed: &InstalledGame,
    runtime: Option<&Path>,
    on_state: &mut dyn StateHandler,
) -> Result<LaunchHandle> {
    let game = &installed.game;
    on_state.change_state(State::Launching(game.clone()));

    match game.runtime() {
        Runtime::HTML => {
            let entry_point = game.executable().to_string_lossy().replace('\\', "/");
            let server = server::start_server(&installed.game_dirs, entry_point, on_state).await?;

            Ok(LaunchHandle::HtmlServer(Arc::new(server)))
        }
        _ => {
            create_game_dirs(&installed.game_dirs).map_err(InstallError::IoError)?;
            let command = process::build_command(installed, runtime)?;

            process::run_command(command, game.runtime())
        }
    }
}
