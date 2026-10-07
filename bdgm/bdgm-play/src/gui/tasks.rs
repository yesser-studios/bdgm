use std::str::FromStr;

use bdgm::game::{Game, ValidatedGame};
use platform_dirs::AppDirs;
use tempfile::TempDir;

use crate::{
    cli::args::Args,
    core::{
        dirs::GameDirs,
        image::resolve_image_args_with_progress,
        install::install,
        launch::{kill_process, spawn_game_process, wait_for_process},
        read_manifest,
        server::start_html_server,
    },
};

use super::{
    message::Message,
    state::{GameInfo, Slot, StartedHandle, StartedSession},
};

// Synchronous preparation: resolve image -> manifest -> validate ->
// install. Runs inside `spawn_blocking` so the UI stays responsive while
// a raw disc is being dumped. Reports `(sectors_done, sectors_total)` via
// `on_progress` (only the Windows raw-disc path emits progress).
struct Prepared {
    resolved: Args,
    tempdir: Option<TempDir>,
    game: ValidatedGame,
    app_dirs: AppDirs,
}

fn prepare_blocking(
    args: Args,
    on_progress: impl FnMut(u64, u64) + Send,
) -> anyhow::Result<Prepared> {
    let (resolved, tempdir) = resolve_image_args_with_progress(args, on_progress)?;
    let manifest = read_manifest(&resolved)?;
    let game = Game::from_str(&manifest)?;
    let game = ValidatedGame::validate(game)?;

    let app_dirs =
        AppDirs::new(Some("bdgm-play"), true).ok_or_else(|| anyhow::anyhow!("NoAppDirs"))?;
    // NOTE: runs on a blocking thread; large installs will not freeze the UI,
    // but install itself still has no progress reporting.
    install(&game, &app_dirs, &resolved)?;

    Ok(Prepared {
        resolved,
        tempdir,
        game,
        app_dirs,
    })
}

fn store_session(
    slot: &Slot,
    handle: StartedHandle,
    tempdir: Option<TempDir>,
    info: GameInfo,
) -> anyhow::Result<GameInfo> {
    slot.lock()
        .map_err(|_| anyhow::anyhow!("starting slot poisoned"))?
        .replace(StartedSession {
            handle,
            tempdir,
            info: info.clone(),
        });

    Ok(info)
}

// Background start as a stream: yields `DumpProgress` while a raw disc is
// being dumped, then a single terminal `GameStarted`. The blocking
// resolve/install phase runs in `spawn_blocking` with progress bridged over
// a tokio unbounded channel; the async HTML-server spawn runs afterwards.
pub(crate) fn start_game_stream(
    args: Args,
    slot: Slot,
) -> impl iced::futures::Stream<Item = Message> + Send + 'static {
    iced::stream::channel(
        100,
        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            use iced::futures::SinkExt;

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(u64, u64)>();

            let blocking = tokio::task::spawn_blocking(move || {
                prepare_blocking(args, |done, total| {
                    let _ = tx.send((done, total));
                })
            });

            // Forward dump progress concurrently so the bar moves while the
            // blocking thread is still copying sectors.
            let mut forward_output = output.clone();
            let forwarder = tokio::spawn(async move {
                while let Some((done, total)) = rx.recv().await {
                    let _ = forward_output
                        .send(Message::DumpProgress(done, total))
                        .await;
                }
            });

            let prepared: anyhow::Result<Prepared> = match blocking.await {
                Ok(result) => result,
                Err(join_err) => Err(anyhow::anyhow!("background task failed: {join_err}")),
            };
            // `tx` is dropped with the blocking closure, so the forwarder drains
            // and exits here before we emit the terminal message.
            let _ = forwarder.await;

            let result: Result<GameInfo, String> = match prepared {
                Err(e) => Err(format!("{e:#}")),
                Ok(prep) => finish_async(prep, &slot).await,
            };

            let _ = output.send(Message::GameStarted(result)).await;
        },
    )
}

async fn finish_async(prep: Prepared, slot: &Slot) -> Result<GameInfo, String> {
    let mut info = GameInfo::from(&prep.game);
    let handle_result: anyhow::Result<StartedHandle> = match prep.game.runtime() {
        bdgm::runtime::Runtime::HTML => {
            let game_dirs = GameDirs::from(&prep.game, &prep.app_dirs);
            match start_html_server(&prep.game, &game_dirs.install, &prep.app_dirs, false).await {
                Ok(server) => {
                    info.url = Some(server.url.clone());
                    Ok(StartedHandle::Server {
                        join: server.join,
                        abort: server.abort,
                    })
                }
                Err(e) => Err(e),
            }
        }
        _ => match spawn_game_process(&prep.game, &prep.resolved, &prep.app_dirs) {
            Ok(proc) => Ok(StartedHandle::Process(proc)),
            Err(e) => Err(e),
        },
    };

    match handle_result {
        Ok(handle) => store_session(slot, handle, prep.tempdir, info).map_err(|e| format!("{e:#}")),
        Err(e) => Err(format!("{e:#}")),
    }
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
