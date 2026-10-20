use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

use axum::Router;
use bimap::BiMap;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

use crate::core::{
    dirs::{GameDirs, get_app_dirs},
    error::{AppError, LaunchError, Result},
    fs::{acquire_lock, open_file, open_part_file, truncate},
    state::StateHandler,
};

pub struct HtmlServer {
    #[allow(dead_code)]
    port: u16,
    #[allow(dead_code)]
    entry_point: String,
    url: String,
    abort: tokio::task::AbortHandle,
}

impl HtmlServer {
    pub async fn start(dir: &Path, port: u16) -> Result<Self> {
        Self::start_with_entry_point(dir, port, String::new()).await
    }

    async fn start_with_entry_point(dir: &Path, port: u16, entry_point: String) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AddrInUse {
                    AppError::from(LaunchError::PortTaken(port))
                } else {
                    AppError::from(LaunchError::IoError(error))
                }
            })?;
        let port = listener.local_addr().map_err(LaunchError::IoError)?.port();

        let directory = dir.to_path_buf();
        let task = tokio::spawn(async move {
            let app = Router::new().fallback_service(ServeDir::new(directory));
            axum::serve(listener, app).await
        });
        let abort = task.abort_handle();

        let encoded_entry_point = utf8_percent_encode(&entry_point, ENCODE_SET);
        let url = format!("http://127.0.0.1:{port}/{encoded_entry_point}");

        Ok(Self {
            port,
            entry_point,
            url,
            abort,
        })
    }

    pub fn get_url(&self) -> &str {
        &self.url
    }

    pub fn open(&self) {
        let _ = webbrowser::open(self.get_url());
    }

    pub fn kill(&self) {
        self.abort.abort();
    }
}

pub(super) async fn start_server(
    game_dirs: &GameDirs,
    entry_point: String,
    on_state: &mut dyn StateHandler,
) -> Result<HtmlServer> {
    let port = unique_port(game_dirs.id(), on_state)?;
    let server =
        HtmlServer::start_with_entry_point(&game_dirs.install_dir(), port, entry_point).await?;
    server.open();

    Ok(server)
}

fn unique_port(id: &str, on_state: &mut dyn StateHandler) -> Result<u16> {
    let app_dirs = get_app_dirs()?;
    let ports_path = app_dirs.data_dir.join("ports.json");

    if let Some(parent) = ports_path.parent() {
        fs::create_dir_all(parent).map_err(LaunchError::IoError)?;
    }

    let _lock = acquire_lock(&ports_path, true, Some(on_state)).map_err(LaunchError::IoError)?;

    let mut file = open_file(&ports_path).map_err(LaunchError::IoError)?;
    let mut ports = load_ports(&mut file)?;
    drop(file);

    if let Some(port) = ports.get_by_left(id) {
        return Ok(*port);
    }

    let port = find_free_port(&ports)?;
    ports.insert(id.to_string(), port);
    save_ports(&ports, &ports_path)?;

    Ok(port)
}

fn find_free_port(ports: &BiMap<String, u16>) -> Result<u16> {
    let mut claimed_listeners = Vec::new();

    for _ in 0..5000 {
        let listener =
            std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(LaunchError::IoError)?;
        let port = listener.local_addr().map_err(LaunchError::IoError)?.port();

        if ports.get_by_right(&port).is_none() {
            return Ok(port);
        }

        claimed_listeners.push(listener);
    }

    Err(AppError::Other(
        "Could not find an unclaimed port".to_string(),
    ))
}

fn load_ports(file: &mut File) -> Result<BiMap<String, u16>> {
    let mut json = String::new();
    file.read_to_string(&mut json)
        .map_err(LaunchError::IoError)?;

    if json.is_empty() {
        return Ok(BiMap::new());
    }

    serde_json::from_str(&json).map_err(|error| {
        LaunchError::IoError(io::Error::new(io::ErrorKind::InvalidData, error)).into()
    })
}

fn save_ports(ports: &BiMap<String, u16>, path: &Path) -> Result<()> {
    let part_path = path.with_added_extension("part");
    let mut file = open_part_file(path).map_err(LaunchError::IoError)?;

    truncate(&mut file).map_err(LaunchError::IoError)?;
    serde_json::to_writer_pretty(&mut file, ports)
        .map_err(|error| LaunchError::IoError(io::Error::new(io::ErrorKind::InvalidData, error)))?;
    file.sync_all().map_err(LaunchError::IoError)?;
    drop(file);

    fs::rename(&part_path, path).map_err(LaunchError::IoError)?;

    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        File::open(parent)
            .map_err(LaunchError::IoError)?
            .sync_all()
            .map_err(LaunchError::IoError)?;
    }

    Ok(())
}

const ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');
