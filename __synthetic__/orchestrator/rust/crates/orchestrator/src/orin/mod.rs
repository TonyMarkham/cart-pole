pub(crate) mod derived_state;
pub(crate) mod joint_qpos;
pub(crate) mod orin_reply;
pub(crate) mod pose_message;
pub(crate) mod registered;
pub(crate) mod reset_message;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    orin::{
        joint_qpos::JointQpos,
        orin_reply::{OrinReply, POOL_ENTRIES},
        pose_message::PoseMessage,
        registered::Registered,
        reset_message::ResetMessage,
    },
    process::child_stdout,
    simulation::capture::Capture,
};

use anyhow::{Context, Result, anyhow, bail};
use futures_util::{SinkExt, StreamExt};
use physics::Names;
use serde::de::DeserializeOwned;
use std::{
    net::SocketAddr,
    path::Path,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::{
    net::TcpStream,
    process::{Child, Command},
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

pub(crate) fn launch_orin(
    k: usize,
    command: &[String],
    url: &str,
    frames_listen: SocketAddr,
    out: &Path,
    role: &[&str],
) -> Result<Child> {
    let (program, rest) = command
        .split_first()
        .ok_or_else(|| anyhow!("--orin needs a command"))?;
    eprintln!(
        "[{k}] launching synthetic Orin: {} --orchestrator {url} --frames-listen {frames_listen} --out {} {}",
        command.join(" "),
        out.display(),
        role.join(" ")
    );
    Command::new(program)
        .args(rest)
        .arg("--orchestrator")
        .arg(url)
        .arg("--frames-listen")
        .arg(frames_listen.to_string())
        .arg("--out")
        .arg(out)
        .args(role)
        .stdin(Stdio::null())
        .stdout(child_stdout())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("launching {program}"))
}

pub(crate) fn pool_gaps_argument(pool_gaps: &[f64; POOL_ENTRIES]) -> String {
    pool_gaps
        .iter()
        .map(|gap| gap.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) async fn wait_registered(
    orin: &mut WebSocketStream<TcpStream>,
    orin_child: &mut Child,
    player: Option<&mut Child>,
    timeout: Duration,
) -> Result<()> {
    let wait = async {
        let player_exit = async {
            match player {
                Some(player) => player.wait().await,
                None => std::future::pending::<std::io::Result<ExitStatus>>().await,
            }
        };
        tokio::pin!(player_exit);
        loop {
            tokio::select! {
                message = orin.next() => {
                    let message = message.ok_or_else(|| anyhow!("synthetic Orin disconnected before Unity registered"))??;
                    match message {
                        Message::Text(text) => {
                            let registered = serde_json::from_str::<Registered>(text.as_str())
                                .with_context(|| format!("synthetic Orin message {:?}", text.as_str()))?;
                            if registered.kind != "registered" {
                                bail!("synthetic Orin sent {:?} before Unity registered", text.as_str());
                            }
                            return Ok(());
                        }
                        Message::Binary(_) => bail!("synthetic Orin sent binary data before Unity registered"),
                        Message::Close(frame) => bail!("synthetic Orin closed the socket before Unity registered: {frame:?}"),
                        _ => {}
                    }
                }
                status = orin_child.wait() => bail!("synthetic Orin exited before Unity registered: {status:?}"),
                status = &mut player_exit => bail!("the player exited before registering with the synthetic Orin: {status:?}"),
            }
        }
    };
    tokio::time::timeout(timeout, wait).await.map_err(|_| {
        anyhow!(
            "Unity did not register with the synthetic Orin within {} s",
            timeout.as_secs()
        )
    })?
}

pub(crate) async fn send_pose(
    orin: &mut WebSocketStream<TcpStream>,
    id: u32,
    capture: &Capture,
    names: &Names,
) -> Result<()> {
    let message = PoseMessage {
        id,
        time: capture.time,
        joints: vec![
            JointQpos {
                name: names.cart_joint.clone(),
                qpos: capture.pose.cart,
            },
            JointQpos {
                name: names.pole_joint.clone(),
                qpos: capture.pose.pole,
            },
        ],
    };
    orin.send(Message::text(serde_json::to_string(&message)?))
        .await
        .with_context(|| format!("sending pose {id} to the synthetic Orin"))
}

pub(crate) async fn receive_reply(
    orin: &mut WebSocketStream<TcpStream>,
    id: u32,
    timeout: Duration,
) -> Result<OrinReply> {
    let reply = loop {
        let message = tokio::time::timeout(timeout, orin.next())
            .await
            .map_err(|_| {
                anyhow!(
                    "no reply from the synthetic Orin for pose {id} within {} s",
                    timeout.as_secs()
                )
            })?
            .ok_or_else(|| anyhow!("synthetic Orin disconnected"))??;
        match message {
            Message::Text(text) => {
                break serde_json::from_str::<OrinReply>(text.as_str())
                    .with_context(|| format!("synthetic Orin reply {:?}", text.as_str()))?;
            }
            Message::Binary(_) => {
                bail!("synthetic Orin sent binary data for pose {id}, expected its reply")
            }
            Message::Close(frame) => bail!("synthetic Orin closed the socket: {frame:?}"),
            _ => {}
        }
    };
    if reply.id != id {
        bail!(
            "synthetic Orin replied for pose {}, expected {id}",
            reply.id
        );
    }
    // The pool's shape (Contracts): 6 entries, newest first, the newest at
    // age 0 and every older one strictly older.
    if reply.pool.len() != POOL_ENTRIES {
        bail!(
            "pool {id} has {} entries, expected {POOL_ENTRIES}",
            reply.pool.len()
        );
    }
    let ages: Vec<f64> = reply.pool.iter().map(|row| row[0]).collect();
    if ages[0] != 0.0 || ages.windows(2).any(|pair| pair[1] >= pair[0]) {
        bail!("pool {id} ages {ages:?} are not 0 then strictly older, newest first");
    }
    // The break after 14: 3 failed frames in the pool fail the run, and
    // training ends (26).
    if reply.failed {
        bail!("the run failed: 3 of pool {id}'s 6 entries are failed frames");
    }
    Ok(reply)
}

pub(crate) async fn receive_message<T: DeserializeOwned>(
    orin: &mut WebSocketStream<TcpStream>,
    timeout: Duration,
    what: &str,
) -> Result<T> {
    loop {
        let message = tokio::time::timeout(timeout, orin.next())
            .await
            .map_err(|_| {
                anyhow!(
                    "no reply from the synthetic Orin for {what} within {} s",
                    timeout.as_secs()
                )
            })?
            .ok_or_else(|| anyhow!("the synthetic Orin disconnected during {what}"))?
            .with_context(|| format!("the synthetic Orin's socket during {what}"))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_str())
                    .with_context(|| format!("synthetic Orin reply {:?}", text.as_str()));
            }
            Message::Binary(_) => bail!("the synthetic Orin sent binary data for {what}"),
            Message::Close(frame) => {
                bail!("the synthetic Orin closed the socket during {what}: {frame:?}")
            }
            _ => {}
        }
    }
}

pub(crate) async fn reset_orin(
    orin: &mut WebSocketStream<TcpStream>,
    gaps: [f64; POOL_ENTRIES],
    timeout: Duration,
) -> Result<()> {
    let message = ResetMessage {
        kind: "reset",
        gaps,
    };
    orin.send(Message::text(serde_json::to_string(&message)?))
        .await
        .context("sending the reset to the synthetic Orin")?;
    loop {
        let message = tokio::time::timeout(timeout, orin.next())
            .await
            .map_err(|_| {
                anyhow!(
                    "no reset reply from the synthetic Orin within {} s",
                    timeout.as_secs()
                )
            })?
            .ok_or_else(|| anyhow!("synthetic Orin disconnected during the reset"))??;
        match message {
            Message::Text(text) => {
                let reply = serde_json::from_str::<Registered>(text.as_str())
                    .with_context(|| format!("synthetic Orin reset reply {:?}", text.as_str()))?;
                if reply.kind != "reset" {
                    bail!(
                        "synthetic Orin sent {:?}, expected its reset reply",
                        text.as_str()
                    );
                }
                return Ok(());
            }
            Message::Binary(_) => {
                bail!("synthetic Orin sent binary data, expected its reset reply")
            }
            Message::Close(frame) => {
                bail!("synthetic Orin closed the socket during the reset: {frame:?}")
            }
            _ => {}
        }
    }
}

pub(crate) async fn await_close(orin: &mut WebSocketStream<TcpStream>, timeout: Duration) {
    let _ = tokio::time::timeout(timeout, async {
        while let Some(Ok(message)) = orin.next().await {
            if message.is_close() {
                break;
            }
        }
    })
    .await;
}
