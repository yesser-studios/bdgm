use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use axum::Router;
use bdgm::game::ValidatedGame;
use bimap::BiMap;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use platform_dirs::AppDirs;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

use crate::core::{
    error::AppError,
    fs::{acquire_lock, get_file, get_part_file, truncate},
};

pub async fn create_listener(port: Option<u16>) -> Result<TcpListener> {
    match port {
        Some(port) => Ok(TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("failed to bind persisted port {}", port))?),
        None => Ok(TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind free port")?),
    }
}

pub async fn serve(listener: TcpListener, directory: &PathBuf) -> Result<()> {
    let app = Router::new().fallback_service(ServeDir::new(directory));

    axum::serve(listener, app).await.context("Server crashed")?;

    Ok(())
}

const PORTLIST_FILE_NAME: &str = "ports.json";

pub fn get_portlist_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join(PORTLIST_FILE_NAME)
}

pub fn load_ports(file: &mut File) -> Result<BiMap<String, u16>> {
    let mut json = String::new();
    match file.read_to_string(&mut json) {
        Ok(_) => {
            if json.is_empty() {
                Ok(BiMap::new())
            } else {
                match serde_json::from_str(&json) {
                    Ok(ports) => Ok(ports),
                    Err(e) => Err(e.into()),
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BiMap::new()),
        Err(e) => Err(e.into()),
    }
}

pub fn save_ports(ports: BiMap<String, u16>, lock_file: File, data_dir: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(&ports)?;

    let path = get_portlist_file_path(data_dir);
    let (mut temp_file, part_file_path) = get_part_file(&path)?;
    truncate(&mut temp_file)?;
    temp_file.write_all(json.as_bytes())?;
    temp_file.sync_all()?;
    drop(temp_file);

    fs::rename(&part_file_path, &path)?;

    #[cfg(unix)]
    File::open(data_dir)?.sync_all()?;

    drop(lock_file);

    Ok(())
}

pub struct HtmlServerHandle {
    pub addr: std::net::SocketAddr,
    pub url: String,
    pub join: tokio::task::JoinHandle<anyhow::Result<()>>,
    pub abort: tokio::task::AbortHandle,
}

/// Start an HTML game server without blocking, returning a handle that can
/// be aborted to free the port.
///
/// Killable primitive for the GUI modal (abort the handle on Stop).
/// The CLI path just awaits via `play_html_game` (used by `launch::run_game`).
pub async fn start_html_server(
    game: &ValidatedGame,
    install_dir: &Path,
    app_dirs: &AppDirs,
    verbose: bool,
) -> anyhow::Result<HtmlServerHandle> {
    let lock = acquire_lock(&get_portlist_file_path(&app_dirs.data_dir), true, verbose)?;
    let mut file = get_file(&get_portlist_file_path(&app_dirs.data_dir))?;
    let mut ports = load_ports(&mut file)?;
    let port = ports.get_by_left(game.id());

    let listener = match port {
        Some(port) => {
            drop(file);
            drop(lock);
            create_listener(Some(*port)).await?
        }
        None => {
            let mut listener = create_listener(None).await?;
            let mut port = listener.local_addr()?.port();

            let mut tries = 0;
            while ports.get_by_right(&port).is_some() {
                listener = create_listener(None).await?;
                port = listener.local_addr()?.port();

                tries += 1;
                if tries >= 5000 {
                    return Err(AppError::CouldNotFindUnclaimedPort.into());
                }
            }

            ports.insert(game.id().to_string(), listener.local_addr()?.port());
            if verbose {
                println!("Persisting port {}...", listener.local_addr()?.port());
            }
            drop(file);
            save_ports(ports, lock, &app_dirs.data_dir)?;
            listener
        }
    };

    let executable = game.executable().to_string_lossy();
    let encoded = utf8_percent_encode(&executable, ENCODE_SET);

    let address = format!("http://{}/{}", listener.local_addr()?, encoded);
    if verbose {
        println!("Opening {address}");
    }
    webbrowser::open(&address)?;

    if verbose {
        println!("Running server, press Ctrl + C to stop.");
    }
    let addr = listener.local_addr()?;
    let url = address;
    let dir = install_dir.to_path_buf();
    let join = tokio::spawn(async move { serve(listener, &dir).await });
    let abort = join.abort_handle();

    Ok(HtmlServerHandle {
        addr,
        url,
        join,
        abort,
    })
}

/// Start an HTML game and await its server future until it exits.
///
/// CLI convenience wrapper over `start_html_server`: start + await.
/// Aborting the returned handle stops the server so the same persisted
/// port can be rebound on the next launch.
pub async fn play_html_game(
    game: &ValidatedGame,
    install_dir: &Path,
    app_dirs: &AppDirs,
    verbose: bool,
) -> anyhow::Result<()> {
    let handle = start_html_server(game, install_dir, app_dirs, verbose).await?;
    handle.join.await??;

    Ok(())
}

const ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');
