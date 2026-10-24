use std::str::FromStr;

/// A pair of whole pixel values written `x,y` — a window's place or the
/// grid's step (synthetic-loop-dia.md, Training instances, The windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixels {
    pub x: i32,
    pub y: i32,
}

impl FromStr for Pixels {
    type Err = String;

    fn from_str(text: &str) -> Result<Pixels, String> {
        let (x, y) = text.split_once(',').ok_or_else(|| format!("{text:?}: expected x,y in pixels"))?;
        let number = |part: &str| part.trim().parse::<i32>().map_err(|_| format!("{part:?} is not a whole number of pixels"));
        Ok(Pixels { x: number(x)?, y: number(y)? })
    }
}

/// Instance k's window in the grid: left to right in rows of `columns`,
/// then down, each cell `step` from the last, starting at `origin`.
pub fn grid_position(k: usize, columns: u16, step: Pixels, origin: Pixels) -> Pixels {
    let columns = usize::from(columns);
    // k ≤ 11 and columns ≥ 1, so both fit an i32.
    let (column, row) = ((k % columns) as i32, (k / columns) as i32);
    Pixels { x: origin.x + column * step.x, y: origin.y + row * step.y }
}
