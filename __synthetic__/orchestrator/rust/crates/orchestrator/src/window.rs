use std::str::FromStr;

/// A pair of whole pixel values written `x,y` — a window's place or its size
/// (__documentation__/synthetic-loop-dia.md, Training instances, The windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixels {
    pub x: i32,
    pub y: i32,
}

impl FromStr for Pixels {
    type Err = String;

    fn from_str(text: &str) -> Result<Pixels, String> {
        let (x, y) = text
            .split_once(',')
            .ok_or_else(|| format!("{text:?}: expected x,y in pixels"))?;
        let number = |part: &str| {
            part.trim()
                .parse::<i32>()
                .map_err(|_| format!("{part:?} is not a whole number of pixels"))
        };
        Ok(Pixels {
            x: number(x)?,
            y: number(y)?,
        })
    }
}

/// The size of each borderless window when `rows` × `columns` of them fill
/// `screen`. Rounding down can leave fewer than `columns` pixels at the right
/// and `rows` at the bottom. A window under 1 pixel is refused: there would be
/// nothing to watch.
pub fn window_size(screen: Pixels, rows: u16, columns: u16) -> Result<Pixels, String> {
    let size = Pixels {
        x: screen.x / i32::from(columns),
        y: screen.y / i32::from(rows),
    };
    if size.x < 1 || size.y < 1 {
        return Err(format!(
            "a {}x{} screen in {rows} rows and {columns} columns gives windows of {}x{}",
            screen.x, screen.y, size.x, size.y
        ));
    }
    Ok(size)
}

/// Instance k's window in the grid: left to right in rows of `columns`,
/// then down, each cell `step` from the last, starting at `origin`.
pub fn grid_position(k: usize, columns: u16, step: Pixels, origin: Pixels) -> Pixels {
    let columns = usize::from(columns);
    // k ≤ 11 and columns ≥ 1, so both fit an i32.
    let (column, row) = ((k % columns) as i32, (k / columns) as i32);
    Pixels {
        x: origin.x + column * step.x,
        y: origin.y + row * step.y,
    }
}
