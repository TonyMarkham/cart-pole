//! The join: from a chip that may still be booting, to a UDP socket that
//! pushes each datagram to the board. The sequence is calibration's
//! `Wifi::join`, which joined the network on 2026-10-31; every wait sleeps.

use crate::{
    config::{
        wifi::{AT_BEGINSTA, AT_HOSTNAME, AT_UDPBEGIN},
        wifi_timing::{
            ADDRESS_PAUSE_MS, ADDRESS_TRIES, BEGIN_STA_TIMEOUT_MS, HANDSHAKE_PAUSE_MS,
            HANDSHAKE_TIMEOUT_MS, HANDSHAKE_TRIES, REPLY_TIMEOUT_MS, SOFT_RESET_TIMEOUT_MS,
            SOFT_RESET_TRIES, STATUS_PAUSE_MS, STATUS_TRIES, WIFI_MODE_TIMEOUT_MS,
        },
    },
    constants::{
        AT_GETSTATUS, AT_HANDSHAKE, AT_IPSTA, AT_SOFTRESETWIFI, AT_WIFIMODE_OFF,
        AT_WIFIMODE_STATION, PROMPT_GETSTATUS, PROMPT_IPSTA, PROMPT_UDPBEGIN, WL_CONNECT_FAILED,
        WL_CONNECTED, WL_NETWORK_NOT_FOUND,
    },
    wifi::{
        at,
        error::{join_error::JoinError, join_result::JoinResult},
        reply::Reply,
    },
};

use unor4_hal::wake::WakeTimer;

/// Joins the network, opens the UDP socket and turns push mode on, and returns
/// the socket's number. The link must be attached, with interrupts on.
pub(crate) fn join(wake: &mut WakeTimer) -> JoinResult<i32> {
    // Any reply means the chip's AT server is up. It may still be booting,
    // so it is asked again after a pause.
    let mut alive = false;
    for _ in 0..HANDSHAKE_TRIES {
        if at::command(wake, AT_HANDSHAKE, HANDSHAKE_TIMEOUT_MS)?.is_some() {
            alive = true;
            break;
        }
        at::pause(wake, HANDSHAKE_PAUSE_MS)?;
    }
    if !alive {
        return Err(JoinError::NoAnswer);
    }

    // Clears WiFi state kept from an earlier program, push mode included,
    // which survives a reset of the processor. Best-effort, as in WiFiS3's
    // ModemClass::begin: the result gates nothing.
    for _ in 0..SOFT_RESET_TRIES {
        if done(at::command(wake, AT_SOFTRESETWIFI, SOFT_RESET_TIMEOUT_MS)?) {
            break;
        }
    }

    // The hostname takes effect when the mode changes into station mode, so
    // the mode goes to 0 first. Both best-effort.
    at::command(wake, AT_WIFIMODE_OFF, WIFI_MODE_TIMEOUT_MS)?;
    at::command(wake, AT_HOSTNAME, REPLY_TIMEOUT_MS)?;
    if !done(at::command(
        wake,
        AT_WIFIMODE_STATION,
        WIFI_MODE_TIMEOUT_MS,
    )?) {
        return Err(JoinError::NoStationMode);
    }
    if !done(at::command(wake, AT_BEGINSTA, BEGIN_STA_TIMEOUT_MS)?) {
        return Err(JoinError::JoinRefused);
    }

    // The chip joins in the background, so it is asked until it has.
    let mut status = None;
    for _ in 0..STATUS_TRIES {
        at::pause(wake, STATUS_PAUSE_MS)?;
        if let Some(reply) = at::command(wake, AT_GETSTATUS, REPLY_TIMEOUT_MS)?
            && reply.ok()
        {
            status = reply.number_after::<u32>(PROMPT_GETSTATUS);
            if status == Some(WL_CONNECTED) {
                break;
            }
        }
    }
    match status {
        Some(WL_CONNECTED) => {}
        Some(WL_NETWORK_NOT_FOUND) => return Err(JoinError::NetworkNotFound),
        Some(WL_CONNECT_FAILED) => return Err(JoinError::ConnectFailed),
        _ => return Err(JoinError::JoinTimedOut),
    }

    // "Connected" comes before the router's lease: the address reads 0.0.0.0
    // until then.
    let mut addressed = false;
    for _ in 0..ADDRESS_TRIES {
        if let Some(reply) = at::command(wake, AT_IPSTA, REPLY_TIMEOUT_MS)?
            && reply.ok()
            && reply
                .after(PROMPT_IPSTA)
                .is_some_and(|address| address.iter().any(|&byte| byte != b'0' && byte != b'.'))
        {
            addressed = true;
            break;
        }
        at::pause(wake, ADDRESS_PAUSE_MS)?;
    }
    if !addressed {
        return Err(JoinError::NoAddress);
    }

    let socket = at::command(wake, AT_UDPBEGIN, REPLY_TIMEOUT_MS)?
        .filter(Reply::ok)
        .and_then(|reply| reply.number_after::<i32>(PROMPT_UDPBEGIN))
        .filter(|&socket| socket >= 0)
        .ok_or(JoinError::NoSocket)?;

    let push = at::command_fmt(
        wake,
        format_args!("AT+UDPPUSH={socket},1\r\n"),
        REPLY_TIMEOUT_MS,
    )?;
    if !done(push) {
        return Err(JoinError::NoPushMode);
    }

    Ok(socket)
}

/// Whether a reply came, and ended in `OK`.
fn done(reply: Option<Reply>) -> bool {
    reply.is_some_and(|reply| reply.ok())
}
