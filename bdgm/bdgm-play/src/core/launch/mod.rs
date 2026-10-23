use std::{path::Path, sync::Arc};

use bdgm::{game::ValidatedGame, runtime::Runtime};

use crate::core::AppError;

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

pub async fn launch_request<Handler>(
    request: LaunchRequest,
    mut on_state: Handler,
) -> Result<(ValidatedGame, LaunchHandle)>
where
    Handler: StateHandler + Send + 'static,
{
    let LaunchRequest { source, runtime } = request;
    let app_dirs = Arc::new(get_app_dirs()?);

    let (result, mut on_state) = tokio::task::spawn_blocking({
        let app_dirs = app_dirs.clone();
        move || {
            let result = (|| -> Result<InstalledGame> {
                let resolved = resolve(source, &app_dirs, &mut on_state)?;

                match resolved {
                    ResolvedSource::Extracted(source) => {
                        let validated = source.validate(&mut on_state)?;
                        validated.install(&app_dirs, &mut on_state)
                    }
                    ResolvedSource::Installed(installed) => Ok(installed),
                }
            })();
            (result, on_state)
        }
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))?;

    let installed = result?;
    let game = installed.game.clone();
    let handle = run_game(&installed, runtime.as_deref(), &mut on_state).await?;

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

            process::run_command(command, game)
        }
    }
}
