use std::{process::ExitStatus, sync::Arc};

use crate::core::Result;

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

    /// Waits for the launched game to stop. HTML servers have no exit code.
    pub async fn wait(self) -> Result<Option<ExitStatus>> {
        match self {
            LaunchHandle::GameProcess(process) => process.wait().await,
            LaunchHandle::HtmlServer(server) => server.wait().await.map(|()| None),
        }
    }
}
