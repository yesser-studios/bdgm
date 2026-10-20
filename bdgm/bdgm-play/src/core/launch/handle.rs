use std::sync::Arc;

use super::{process::GameProcess, server::HtmlServer};

#[derive(Clone)]
pub enum LaunchHandle {
    GameProcess(GameProcess),
    HtmlServer(Arc<HtmlServer>),
}

impl LaunchHandle {
    pub async fn kill(&self) {
        match self {
            LaunchHandle::GameProcess(process) => process.kill().await,
            LaunchHandle::HtmlServer(server) => server.kill(),
        }
    }
}
