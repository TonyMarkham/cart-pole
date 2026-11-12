use crate::error::{app_error::AppError, app_result::AppResult};

use std::time::Duration;
use tokio::{net::TcpStream, process::Child};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::debug;

/// A connected player's WebSocket.
pub(crate) type PlayerSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Connects to a starting player at `url`, trying every `retry` until it
/// accepts: a player listens only once its scene has loaded. Fails when the
/// player exits first, or when `timeout` passes without a connection.
pub(crate) async fn connect(
    url: &str,
    child: &mut Child,
    timeout: Duration,
    retry: Duration,
) -> AppResult<PlayerSocket> {
    let attempts = async {
        loop {
            match connect_async(url).await {
                Ok((socket, _response)) => return Ok(socket),
                Err(error) => {
                    debug!(%url, %error, "player not accepting yet");
                    let exited = child.try_wait().map_err(|error| {
                        AppError::player(format!("checking the player for {url}: {error}"))
                    })?;
                    if let Some(status) = exited {
                        return Err(AppError::player(format!(
                            "the player for {url} exited before accepting a connection: {status}"
                        )));
                    }
                    tokio::time::sleep(retry).await;
                }
            }
        }
    };
    tokio::time::timeout(timeout, attempts).await.map_err(|_| {
        AppError::player(format!(
            "no player accepted a connection on {url} within {} s",
            timeout.as_secs()
        ))
    })?
}
