use crate::error::app_result::AppResult;

use axum::{
    body::Bytes,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use tokio::sync::watch;

/// Opens one viewer's WebSocket. A failed socket is logged and closed; it never
/// stops the server.
pub(crate) async fn ws_handler(
    upgrade: WebSocketUpgrade,
    State(images): State<watch::Sender<Option<Bytes>>>,
) -> Response {
    let images = images.subscribe();

    upgrade.on_upgrade(move |socket| async move {
        if let Err(error) = push_images(socket, images).await {
            eprintln!("{error}");
        }
    })
}

/// Sends the latest image at once, then each new one, until the page closes.
async fn push_images(
    mut socket: WebSocket,
    mut images: watch::Receiver<Option<Bytes>>,
) -> AppResult<()> {
    images.mark_changed();

    loop {
        tokio::select! {
            changed = images.changed() => {
                changed?;

                let image = images.borrow_and_update().clone();

                if let Some(image) = image {
                    socket.send(Message::Binary(image)).await?;
                }
            }
            message = socket.recv() => {
                match message {
                    Some(message) => { message?; }
                    None => return Ok(()),
                }
            }
        }
    }
}
