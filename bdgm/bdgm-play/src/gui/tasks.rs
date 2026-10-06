use std::str::FromStr;

use bdgm::game::{Game, ValidatedGame};
use platform_dirs::AppDirs;

use crate::{
    cli::args::Args,
    core::{
        dirs::GameDirs,
        image::resolve_image_args,
        install::install,
        launch::{kill_process, spawn_game_process, wait_for_process},
        read_manifest,
        server::start_html_server,
    },
};

use super::state::{GameInfo, Slot, StartedHandle, StartedSession};

// Background start: resolve image -> manifest -> validate -> install ->
// spawn killable process or HTML server. Stores the session in `slot` and
// returns only the display info so `Message` stays `Clone`.
pub(crate) async fn start_game_task(args: Args, slot: Slot) -> Result<GameInfo, String> {
    let result: anyhow::Result<GameInfo> = (|| async {
        let (resolved, tempdir) = resolve_image_args(args)?;
        let manifest = read_manifest(&resolved)?;
        let game = Game::from_str(&manifest)?;
        let game = ValidatedGame::validate(game)?;

        let app_dirs =
            AppDirs::new(Some("bdgm-play"), true).ok_or_else(|| anyhow::anyhow!("NoAppDirs"))?;
        // NOTE: runs on the async runtime; large installs will briefly block
        // the UI. Future work: move into `spawn_blocking` with progress.
        install(&game, &app_dirs, &resolved)?;

        let mut info = GameInfo::from(&game);
        let handle = match game.runtime() {
            bdgm::runtime::Runtime::HTML => {
                let game_dirs = GameDirs::from(&game, &app_dirs);
                let server = start_html_server(&game, &game_dirs.install, &app_dirs, false).await?;
                info.url = Some(server.url.clone());
                StartedHandle::Server {
                    join: server.join,
                    abort: server.abort,
                }
            }
            _ => {
                let proc = spawn_game_process(&game, &resolved, &app_dirs)?;
                StartedHandle::Process(proc)
            }
        };

        slot.lock()
            .map_err(|_| anyhow::anyhow!("starting slot poisoned"))?
            .replace(StartedSession {
                handle,
                tempdir,
                info: info.clone(),
            });

        Ok(info)
    })()
    .await;

    result.map_err(|e| format!("{e:#}"))
}

pub(crate) async fn wait_process_task(
    handle: crate::core::launch::ProcessHandle,
) -> Result<Option<i32>, String> {
    wait_for_process(handle).await.map_err(|e| format!("{e:#}"))
}

pub(crate) async fn wait_server_task(
    join: tokio::task::JoinHandle<anyhow::Result<()>>,
) -> Result<Option<i32>, String> {
    match join.await {
        Ok(Ok(())) => Ok(None),
        Ok(Err(e)) => Err(format!("Server crashed: {e:#}")),
        Err(join_err) if join_err.is_cancelled() => Ok(None),
        Err(join_err) => Err(format!("Server task failed: {join_err}")),
    }
}

pub(crate) async fn kill_process_task(handle: crate::core::launch::ProcessHandle) {
    kill_process(&handle).await;
}
