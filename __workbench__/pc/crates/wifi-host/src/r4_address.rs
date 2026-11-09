use crate::{
    protocol::{R4_RUNNING, R4_STATUS, R4_STOPPED},
    serial_line::SerialLine,
};

use anyhow::{Context, Result, bail};
use std::time::{Duration, Instant};

/// How long one `STATUS` waits for its answer before it is asked again, ms.
/// The R4 can't answer while it joins, so the question is repeated until it
/// is running.
const STATUS_WAIT_MS: u64 = 1_000;

/// What separates the address from the port in the running answer.
const ADDRESS_PORT_SEPARATOR: char = ' ';

/// Where the R4 listens, from its `OK STATUS running <address> <port>`. The
/// bench-host's copy.
pub(crate) struct R4Address {
    /// The address the router gave it, IPv4, as text.
    pub(crate) address: String,
    /// The UDP port it listens on: its `config/wifi.toml` `udp_port`.
    pub(crate) port: u16,
}

impl R4Address {
    /// Asks the R4 `STATUS` until it answers that it is running, for at most
    /// `timeout`. An answer that it stopped ends the wait with its reason.
    pub(crate) fn ask(r4: &mut SerialLine, timeout: Duration) -> Result<Self> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            r4.write_line(R4_STATUS)?;
            let asked = Instant::now();
            while let Some(left) =
                Duration::from_millis(STATUS_WAIT_MS).checked_sub(asked.elapsed())
            {
                let Some(line) = r4.read_line(left)? else {
                    break;
                };
                if let Some(rest) = line.strip_prefix(R4_RUNNING) {
                    return Self::parse(rest);
                }
                if let Some(why) = line.strip_prefix(R4_STOPPED) {
                    bail!("the R4 stopped: {why}");
                }
            }
        }
        bail!(
            "the R4 did not say it was running within {} ms",
            timeout.as_millis()
        )
    }

    fn parse(rest: &str) -> Result<Self> {
        let (address, port) = rest
            .split_once(ADDRESS_PORT_SEPARATOR)
            .with_context(|| format!("the R4's address and port: `{rest}`"))?;
        Ok(Self {
            address: address.to_owned(),
            port: port
                .parse()
                .with_context(|| format!("the R4's port: `{port}`"))?,
        })
    }
}
