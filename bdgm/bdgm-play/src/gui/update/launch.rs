use std::{path::PathBuf, sync::Arc};

use iced::Task;
use tokio::sync::mpsc::unbounded_channel;

use crate::core::{LaunchRequest, Source, SourceKind, State, launch_request};

use super::super::{
    message::Message,
    state::{AppState, Status},
};
use super::state::GuiStateHandler;

/// Builds an image request using the GUI's runtime override without opening the image.
pub(super) fn create_image_launch_request(state: &AppState, path: PathBuf) -> LaunchRequest {
    LaunchRequest::new(Source::new(path, SourceKind::Image), state.runtime.clone())
}

/// Builds a mounted-disc request for a disc root using the GUI's runtime override.
pub(super) fn create_mounted_launch_request(state: &AppState, path: PathBuf) -> LaunchRequest {
    LaunchRequest::new(
        Source::new(path, SourceKind::MountedDisc),
        state.runtime.clone(),
    )
}

/// Builds a Windows device-path request from a drive letter and the GUI's runtime override.
pub(super) fn create_raw_disc_launch_request(state: &AppState, disc: char) -> LaunchRequest {
    LaunchRequest::new(
        Source::new(
            PathBuf::from(format!("\\\\.\\{disc}:")),
            SourceKind::RawDisc,
        ),
        state.runtime.clone(),
    )
}

/// Closes the disc picker and changes the GUI to its initial preparation state.
pub(super) fn pre_launch(state: &mut AppState) {
    state.disc_picker_discs = None;
    state.status = Status::Starting(Arc::new(State::StartingExtraction));
}

/// Creates a task that launches a game and forwards preparation, start, and exit messages.
///
/// Launch failures become `GameError`; wait failures become failed `GameExited`
/// results. Process exits without a code and HTML server exits both yield `None`.
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

            let state_handler = GuiStateHandler::new(sender);

            tokio::spawn(async move {
                match launch_request(request, state_handler).await {
                    Ok((game, handle)) => {
                        let _ = output
                            .send(Message::GameStarted(game.clone(), handle.clone()))
                            .await;

                        let exit_code = handle
                            .wait()
                            .await
                            .map(|status| status.and_then(|status| status.code()));

                        let _ = output.send(Message::GameExited(game, exit_code)).await;
                    }
                    Err(error) => {
                        let _ = output.send(Message::GameError(error)).await;
                    }
                }
            });
        },
    ))
}
