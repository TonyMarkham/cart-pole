//! The serial line to the Arduino. One task owns the port: it reads every
//! line the board sends, logs it and hands it to whoever waits for a
//! reply, and writes the lines the manager sends. A missing port is retried
//! every `reopen_ms`, so the service starts before the board is plugged
//! in and picks it up after a replug. Flashing and the hard reset pause
//! the task, which closes the port so `arduino-cli`, or the reset's
//! 2,400-baud opening, can have it.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use futures_util::StreamExt;
use tokio::{
    io::AsyncWriteExt,
    sync::{broadcast, mpsc, oneshot, watch},
};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};

use crate::{
    log::{LogBook, Source},
    protocol::Line,
    settings::Serial,
};

/// The longest line read, bytes; a longer one is dropped whole. Far above
/// any line the protocol names.
const MAX_LINE: usize = 4096;

/// What the link reports to whoever listens.
#[derive(Clone, Debug)]
pub enum Event {
    Line(Line),
    /// The port closed: unplugged, or paused for a flash or a hard reset.
    Lost,
}

enum Request {
    Send(String),
    /// Close the port and answer once it is closed; reopen on `Resume`.
    Pause(oneshot::Sender<()>),
    Resume,
}

#[derive(Clone)]
pub struct Link {
    requests: mpsc::Sender<Request>,
    events: broadcast::Sender<Event>,
    connected: watch::Receiver<bool>,
}

impl Link {
    /// Starts the task that owns the port.
    pub fn start(settings: &Serial, log: Arc<Mutex<LogBook>>) -> Self {
        let (requests, receiver) = mpsc::channel(64);
        let (events, _) = broadcast::channel(1024);
        let (connected_tx, connected) = watch::channel(false);
        tokio::spawn(run(
            settings.port.clone(),
            settings.baud,
            Duration::from_millis(settings.reopen_ms),
            receiver,
            events.clone(),
            connected_tx,
            log,
        ));
        Self { requests, events, connected }
    }

    /// Subscribe before sending a command, so its reply cannot be missed.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub fn connected(&self) -> bool {
        *self.connected.borrow()
    }

    /// Queues a line to send. False if the link task has ended.
    pub async fn send(&self, line: String) -> bool {
        self.requests.send(Request::Send(line)).await.is_ok()
    }

    /// Closes the port, returning once it is closed.
    pub async fn pause(&self) {
        let (ack, closed) = oneshot::channel();
        if self.requests.send(Request::Pause(ack)).await.is_ok() {
            let _ = closed.await;
        }
    }

    pub async fn resume(&self) {
        let _ = self.requests.send(Request::Resume).await;
    }
}

fn log(log: &Mutex<LogBook>, source: Source, text: &str) {
    log.lock().unwrap_or_else(std::sync::PoisonError::into_inner).write(source, text);
}

async fn run(
    port: String,
    baud: u32,
    reopen: Duration,
    mut requests: mpsc::Receiver<Request>,
    events: broadcast::Sender<Event>,
    connected: watch::Sender<bool>,
    book: Arc<Mutex<LogBook>>,
) {
    let mut reported_missing = false;
    loop {
        let serial = match tokio_serial::new(&port, baud).open_native_async() {
            Ok(serial) => serial,
            Err(error) => {
                if !reported_missing {
                    log(&book, Source::Note, &format!("cannot open {port}: {error}; retrying every {} ms", reopen.as_millis()));
                    reported_missing = true;
                }
                // Answer requests while the port is missing, so a sender
                // or a flash never waits on it.
                let wait = tokio::time::sleep(reopen);
                tokio::pin!(wait);
                loop {
                    tokio::select! {
                        () = &mut wait => break,
                        request = requests.recv() => match request {
                            Some(Request::Send(line)) => log(&book, Source::Note, &format!("not sent, the port is closed: {line}")),
                            Some(Request::Pause(ack)) => {
                                let _ = ack.send(());
                                if !wait_for_resume(&mut requests).await {
                                    return;
                                }
                            }
                            Some(Request::Resume) => {}
                            None => return,
                        },
                    }
                }
                continue;
            }
        };
        reported_missing = false;
        {
            let mut book = book.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            book.start_file();
            book.write(Source::Note, &format!("opened {port} at {baud} baud"));
        }
        let _ = connected.send(true);

        let (reader, mut writer) = tokio::io::split(serial);
        let mut lines = FramedRead::new(reader, LinesCodec::new_with_max_length(MAX_LINE));
        let paused = loop {
            tokio::select! {
                line = lines.next() => match line {
                    Some(Ok(text)) => {
                        log(&book, Source::Board, &text);
                        let _ = events.send(Event::Line(Line::parse(&text)));
                    }
                    Some(Err(LinesCodecError::MaxLineLengthExceeded)) => {
                        log(&book, Source::Note, &format!("dropped a line longer than {MAX_LINE} bytes"));
                    }
                    // Bytes that are not UTF-8: the codec has consumed
                    // the line, so reading goes on.
                    Some(Err(LinesCodecError::Io(error))) if error.kind() == std::io::ErrorKind::InvalidData => {
                        log(&book, Source::Note, "dropped a line that is not text");
                    }
                    Some(Err(LinesCodecError::Io(error))) => {
                        log(&book, Source::Note, &format!("{port} read failed: {error}"));
                        break None;
                    }
                    None => {
                        log(&book, Source::Note, &format!("{port} closed"));
                        break None;
                    }
                },
                request = requests.recv() => match request {
                    Some(Request::Send(line)) => {
                        let sent = writer.write_all(format!("{line}\n").as_bytes()).await;
                        match sent {
                            Ok(()) => log(&book, Source::Sent, &line),
                            Err(error) => {
                                log(&book, Source::Note, &format!("not sent, {port} write failed ({error}): {line}"));
                                break None;
                            }
                        }
                    }
                    Some(Request::Pause(ack)) => break Some(ack),
                    Some(Request::Resume) => {}
                    None => return,
                },
            }
        };

        drop(lines);
        drop(writer);
        let _ = connected.send(false);
        let _ = events.send(Event::Lost);
        match paused {
            Some(ack) => {
                // The file stays open, every line already flushed, so the
                // flash's own lines land in it; reopening starts the next.
                log(&book, Source::Note, &format!("closed {port} until the flash or reset is done"));
                let _ = ack.send(());
                if !wait_for_resume(&mut requests).await {
                    return;
                }
            }
            None => tokio::time::sleep(reopen).await,
        }
    }
}

/// Holds the port closed until `Resume`. False if the service is ending.
async fn wait_for_resume(requests: &mut mpsc::Receiver<Request>) -> bool {
    loop {
        match requests.recv().await {
            Some(Request::Resume) => return true,
            Some(Request::Pause(ack)) => {
                let _ = ack.send(());
            }
            Some(Request::Send(_)) => {}
            None => return false,
        }
    }
}
