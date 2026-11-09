#[cfg(target_os = "linux")]
use crate::constants::LED_CONSUMER;

use anyhow::Result;
#[cfg(not(target_os = "linux"))]
use anyhow::bail;
#[cfg(target_os = "linux")]
use anyhow::{Context, anyhow};

/// The LED on its GPIO line, driven through Linux's GPIO character device
/// (`gpiocdev`), held for the whole run so each switch is one write, a few
/// µs, with no process to start. Requested off. Dropping it releases the line.
/// On anything but Linux it can't be opened: the LED is the Orin's.
pub(crate) struct Led {
    #[cfg(target_os = "linux")]
    request: gpiocdev::Request,
}

impl Led {
    /// The line named `line_name`, as `gpioinfo` lists it, requested as an
    /// output, off.
    #[cfg(target_os = "linux")]
    pub(crate) fn open(line_name: &str) -> Result<Led> {
        if line_name.is_empty() {
            return Err(anyhow!(
                "[led] line_name is empty: set it to the name `gpioinfo` lists for the LED's pin"
            ));
        }
        let found = gpiocdev::find_named_line(line_name)
            .ok_or_else(|| anyhow!("no GPIO line named `{line_name}` (`gpioinfo` lists them)"))?;
        let request = gpiocdev::Request::builder()
            .with_found_line(&found)
            .with_consumer(LED_CONSUMER)
            .as_output(gpiocdev::line::Value::Inactive)
            .request()
            .with_context(|| format!("requesting GPIO line `{line_name}`"))?;
        Ok(Led { request })
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn open(_line_name: &str) -> Result<Led> {
        bail!("the LED is on the Orin's GPIO: camera-timing runs only on the Orin")
    }

    /// Lights the LED.
    #[cfg(target_os = "linux")]
    pub(crate) fn on(&self) -> Result<()> {
        Ok(self.request.set_lone_value(gpiocdev::line::Value::Active)?)
    }

    /// Puts the LED out.
    #[cfg(target_os = "linux")]
    pub(crate) fn off(&self) -> Result<()> {
        Ok(self
            .request
            .set_lone_value(gpiocdev::line::Value::Inactive)?)
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn on(&self) -> Result<()> {
        bail!("the LED is on the Orin's GPIO")
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn off(&self) -> Result<()> {
        bail!("the LED is on the Orin's GPIO")
    }
}
