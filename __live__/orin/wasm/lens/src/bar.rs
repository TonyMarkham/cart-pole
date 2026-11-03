use crate::{error::result::AppResult, meter::Meter, page::element};

use web_sys::{Document, Element};

/// One progress bar in `index.html`: the row it sits in, its words, its fill and
/// its tick. Each id is the row's id with `-label`, `-fill` or `-mark` added.
pub(crate) struct Bar {
    row: Element,
    label: Element,
    fill: Element,
    mark: Element,
}

impl Bar {
    pub(crate) fn find(document: &Document, id: &str) -> AppResult<Self> {
        Ok(Self {
            row: element(document, id)?,
            label: element(document, &format!("{id}-label"))?,
            fill: element(document, &format!("{id}-fill"))?,
            mark: element(document, &format!("{id}-mark"))?,
        })
    }

    /// Shows the reading, or hides the whole row when there is none.
    pub(crate) fn show(&self, meter: Option<&Meter>) -> AppResult<()> {
        let Some(meter) = meter else {
            return Ok(self.row.set_attribute("hidden", "")?);
        };
        self.row.remove_attribute("hidden")?;
        self.label.set_text_content(Some(&meter.label));
        self.fill
            .set_attribute("style", &format!("width: {:.1}%", percent(meter.fraction)))?;
        self.fill
            .set_attribute("class", if meter.good { "fill good" } else { "fill" })?;
        match meter.mark {
            Some(at) => {
                self.mark.remove_attribute("hidden")?;
                self.mark
                    .set_attribute("style", &format!("left: {:.1}%", percent(at)))?;
            }
            None => self.mark.set_attribute("hidden", "")?,
        }
        Ok(())
    }
}

/// A share as a percentage, kept within the bar.
fn percent(fraction: f64) -> f64 {
    (fraction * 100.0).clamp(0.0, 100.0)
}
