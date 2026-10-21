use std::{path::PathBuf, sync::Arc};

use iced::Task;
use tokio::sync::mpsc::unbounded_channel;

use crate::core::{LaunchHandle, LaunchRequest, Source, SourceKind, State, launch_request};

use super::super::{
    message::Message,
    state::{AppState, Status},
};
use super::state::GuiStateHandler;

pub(super) fn create_image_launch_request(state: &AppState, path: PathBuf) -> LaunchRequest {
    LaunchRequest::new(Source::new(path, SourceKind::Image), state.runtime.clone())
}

pub(super) fn create_mounted_launch_request(state: &AppState, path: PathBuf) -> LaunchRequest {
    LaunchRequest::new(
        Source::new(path, SourceKind::MountedDisc),
        state.runtime.clone(),
    )
}

pub(super) fn create_raw_disc_launch_request(state: &AppState, disc: char) -> LaunchRequest {
    LaunchRequest::new(
        Source::new(
            PathBuf::from(format!("\\\\.\\{disc}:")),
            SourceKind::RawDisc,
        ),
        state.runtime.clone(),
    )
}

pub(super) fn pre_launch(state: &mut AppState) {
    state.disc_picker_discs = None;
    state.status = Status::Starting(Arc::new(State::StartingExtraction));
}

pub(super) fn launch(request: LaunchRequest) -> Task<Message> {
    Task::stream(iced::stream::channel(
        100,
        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            use iced::futures::SinkExt;

            let (sender, mut receiver) = unbounded_channel::<Message>();

            // Forward core state changes to the iced message stream.
            let mut forward_output = output.clone();
            tokio::spawn(async move {
                while let Some(message) = receiver.recv().await {
                    if forward_output.send(message).await.is_err() {
                        break;
                    }
                }
            });

            let mut state_handler = GuiStateHandler::new(sender);

            tokio::spawn(async move {
                match launch_request(request, &mut state_handler).await {
                    Ok((game, handle)) => {
                        let _ = output
                            .send(Message::GameStarted(game.clone(), handle.clone()))
                            .await;

                        if let LaunchHandle::GameProcess(process) = handle {
                            let exit_code = process
                                .wait()
                                .await
                                .map(|status| status.and_then(|status| status.code()));

                            let _ = output.send(Message::GameExited(game, exit_code)).await;
                        }
                    }
                    Err(error) => {
                        let _ = output.send(Message::GameError(error)).await;
                    }
                }
            });
        },
    ))
}
