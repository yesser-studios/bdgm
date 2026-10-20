use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

use axum::Router;
use bimap::BiMap;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use platform_dirs::AppDirs;
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
            axum::serve(listener, app)
                .await
                .map_err(|e| AppError::RunningError(e.into()))
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
    unique_port_in(id, &app_dirs, on_state)
}

fn unique_port_in(id: &str, app_dirs: &AppDirs, on_state: &mut dyn StateHandler) -> Result<u16> {
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

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpStream,
        time::Duration,
    };

    use tempfile::tempdir;

    use super::*;
    use crate::core::test_utils::{RecordingStateHandler, TestEnv};

    #[test]
    fn save_and_load_ports_round_trips() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("ports.json");
        let mut ports = BiMap::new();
        ports.insert("com.example.one".to_string(), 1234);
        ports.insert("com.example.two".to_string(), 5678);

        save_ports(&ports, &path).expect("save ports");
        let mut file = open_file(&path).expect("open ports");
        let loaded = load_ports(&mut file).expect("load ports");

        assert_eq!(loaded, ports);
    }

    #[test]
    fn load_ports_treats_an_empty_file_as_an_empty_map() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("ports.json");
        fs::write(&path, b"").expect("write empty ports file");

        let mut file = open_file(&path).expect("open ports");
        let ports = load_ports(&mut file).expect("load ports");

        assert!(ports.is_empty());
    }

    #[test]
    fn load_ports_rejects_invalid_json() {
        let temp = tempdir().expect("create temporary directory");
        let path = temp.path().join("ports.json");
        fs::write(&path, b"not json").expect("write invalid ports file");

        let mut file = open_file(&path).expect("open ports");
        let result = load_ports(&mut file);

        assert!(matches!(
            result,
            Err(AppError::LaunchError(LaunchError::IoError(_)))
        ));
    }

    #[test]
    fn unique_port_persists_and_reuses_ports() {
        let env = TestEnv::new();
        let mut state = RecordingStateHandler::new();

        let first =
            unique_port_in("com.example.one", &env.app_dirs, &mut state).expect("allocate port");
        let same =
            unique_port_in("com.example.one", &env.app_dirs, &mut state).expect("reuse port");
        let other =
            unique_port_in("com.example.two", &env.app_dirs, &mut state).expect("allocate port");

        assert_eq!(first, same);
        assert_ne!(first, other);

        let ports_path = env.app_dirs.data_dir.join("ports.json");
        let mut file = open_file(&ports_path).expect("open ports");
        let ports = load_ports(&mut file).expect("load ports");
        assert_eq!(ports.get_by_left("com.example.one"), Some(&first));
        assert_eq!(ports.get_by_left("com.example.two"), Some(&other));
    }

    #[test]
    fn unique_port_returns_a_persisted_assignment() {
        let env = TestEnv::new();
        fs::create_dir_all(&env.app_dirs.data_dir).expect("create data directory");
        let ports_path = env.app_dirs.data_dir.join("ports.json");
        let mut ports = BiMap::new();
        ports.insert("com.example.one".to_string(), 4321);
        save_ports(&ports, &ports_path).expect("save ports");
        let mut state = RecordingStateHandler::new();

        let port =
            unique_port_in("com.example.one", &env.app_dirs, &mut state).expect("look up port");

        assert_eq!(port, 4321);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn html_server_serves_files_and_reports_its_url() {
        let temp = tempdir().expect("create temporary directory");
        fs::write(temp.path().join("index.html"), b"<h1>hello</h1>").expect("write index");
        let server = HtmlServer::start(temp.path(), 0)
            .await
            .expect("start HTML server");
        let url = server.get_url().to_string();
        let authority = url
            .strip_prefix("http://")
            .expect("HTTP URL")
            .trim_end_matches('/')
            .to_string();

        let mut stream = TcpStream::connect(authority.as_str()).expect("connect to HTML server");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");
        stream
            .write_all(
                format!(
                    "GET /index.html HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .expect("send HTTP request");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("read HTTP response");

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("<h1>hello</h1>"));

        server.kill();
    }

    #[tokio::test]
    async fn html_server_reports_an_occupied_port() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
        let port = listener.local_addr().expect("read listener address").port();
        let temp = tempdir().expect("create temporary directory");

        let result = HtmlServer::start(temp.path(), port).await;

        assert!(matches!(
            result,
            Err(AppError::LaunchError(LaunchError::PortTaken(taken))) if taken == port
        ));
        drop(listener);
    }

    #[tokio::test]
    async fn html_server_url_encodes_the_entry_point() {
        let temp = tempdir().expect("create temporary directory");

        let server =
            HtmlServer::start_with_entry_point(temp.path(), 0, "dir/game file.html".to_string())
                .await
                .expect("start HTML server");

        assert!(server.get_url().ends_with("/dir/game%20file.html"));

        server.kill();
    }
}
