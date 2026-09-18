use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use axum::Router;
use bimap::BiMap;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

pub(crate) async fn create_listener(port: Option<u16>) -> Result<TcpListener> {
    match port {
        Some(port) => Ok(TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("failed to bind persisted port {}", port))?),
        None => Ok(TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind free port")?),
    }
}

pub(crate) async fn serve(listener: TcpListener, directory: PathBuf) -> Result<()> {
    let app = Router::new().fallback_service(ServeDir::new(directory));

    axum::serve(listener, app).await.context("Server crashed")?;

    Ok(())
}

const PORTLIST_FILE_NAME: &'static str = "ports.json";

pub(crate) fn get_file(path: &PathBuf) -> Result<File> {
    let file = File::options()
        .write(true)
        .read(true)
        .create(true)
        .open(path)?;
    file.lock()?;
    Ok(file)
}

pub(crate) fn get_portlist_file_path(data_dir: &PathBuf) -> PathBuf {
    data_dir.join(PORTLIST_FILE_NAME)
}

pub(crate) fn load_ports(file: &mut File) -> Result<BiMap<String, u16>> {
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

pub(crate) fn save_ports(ports: BiMap<String, u16>, file: File, data_dir: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(&ports)?;

    let path = get_portlist_file_path(data_dir);
    let temp_path = path.with_added_extension("tmp");
    let mut temp_file = get_file(&temp_path)?;
    temp_file.write_all(json.as_bytes())?;
    temp_file.sync_all()?;
    drop(temp_file);

    fs::rename(&temp_path, &path)?;
    drop(file);

    Ok(())
}
