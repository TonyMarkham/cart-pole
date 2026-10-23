use anyhow::{Result, bail};

use crate::grey::Grey;

// OpenCV's INTER_LINEAR on 8-bit images works in fixed point: each
// weight is scaled by 2^11 and rounded (INTER_RESIZE_COEF_BITS).
const COEF_BITS: u32 = 11;
const COEF_SCALE: f32 = (1 << COEF_BITS) as f32;

/// Ultralytics' letterbox for one frame size (D1; `data/augment.py`
/// `LetterBox`, `auto` off): the frame is scaled by `scale` to
/// `width × height`, centered in the `size × size` input with `left` /
/// `top` columns / rows of padding before it.
///
/// For the 728×544 loop frame into 640: scale = 640/728 = 0.879121,
/// resized 640×478, left 0, top 81 (81 rows of padding above and below).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Letterbox {
    pub size: usize,
    pub scale: f64,
    pub width: usize,
    pub height: usize,
    pub left: usize,
    pub top: usize,
}

impl Letterbox {
    /// The geometry for a `frame_width × frame_height` frame. Python's
    /// `round` (ties to even) is used wherever Ultralytics rounds.
    pub fn new(frame_width: usize, frame_height: usize, size: usize) -> Result<Letterbox> {
        if frame_width == 0 || frame_height == 0 {
            bail!("frame is {frame_width}x{frame_height}");
        }
        let side = size as f64;
        let scale = (side / frame_height as f64).min(side / frame_width as f64);
        let width = (frame_width as f64 * scale).round_ties_even() as usize;
        let height = (frame_height as f64 * scale).round_ties_even() as usize;
        // The padding is split round(d/2 - 0.1) before and round(d/2 + 0.1)
        // after, so an odd remainder puts the extra row after.
        let before = |d: usize| (d as f64 / 2.0 - 0.1).round_ties_even() as usize;
        Ok(Letterbox { size, scale, width, height, left: before(size - width), top: before(size - height) })
    }

    /// The model input: the frame resized (D2) into the letterbox, the
    /// rest filled with `pad_value`, every value ÷ 255; `size²` values,
    /// row-major.
    pub fn apply(&self, frame: &Grey, pad_value: u8) -> Vec<f32> {
        let resized = resize_linear(frame, self.width, self.height);
        let mut input = vec![f32::from(pad_value) / 255.0; self.size * self.size];
        for (y, row) in resized.chunks_exact(self.width).enumerate() {
            let at = (self.top + y) * self.size + self.left;
            for (value, &pixel) in input[at..at + self.width].iter_mut().zip(row) {
                *value = f32::from(pixel) / 255.0;
            }
        }
        input
    }

    /// A point in the input's pixels mapped back to the frame's pixels.
    pub fn to_frame(&self, x: f32, y: f32) -> (f64, f64) {
        ((f64::from(x) - self.left as f64) / self.scale, (f64::from(y) - self.top as f64) / self.scale)
    }
}

// OpenCV `resize` with INTER_LINEAR on one 8-bit channel (D2), as its
// x86 SIMD path computes it (imgproc resize.cpp, HResizeLinear and
// VResizeLinearVec_32s8u). Per axis, destination index d samples source
// coordinate f = (d + 0.5) · src/dst − 0.5: s = floor(f) and s + 1,
// weighted 1 − (f − s) and f − s, each × 2^11 rounded. Rows are first
// interpolated horizontally to sums scaled by 2^11; the vertical pass
// keeps each sum's top bits (>> 4), multiplies by the row weight keeping
// the high 16 bits of the product, adds the two, and rounds the last
// 2 bits away: 4 + 16 + 2 = 22 = 2 × 11 bits in all.
fn resize_linear(src: &Grey, width: usize, height: usize) -> Vec<u8> {
    let xs = taps(src.width, width);
    let ys = taps(src.height, height);
    let mut out = Vec::with_capacity(width * height);
    let row = |y: usize| -> Vec<i32> {
        let line = &src.pixels[y * src.width..(y + 1) * src.width];
        xs.iter()
            .map(|t| i32::from(line[t.at]) * t.weights[0] + i32::from(line[t.next]) * t.weights[1])
            .collect()
    };
    for t in &ys {
        let (upper, lower) = (row(t.at), row(t.next));
        for (&a, &b) in upper.iter().zip(&lower) {
            let mul_hi = |sum: i32, weight: i32| ((sum >> 4) * weight) >> 16;
            let value = (mul_hi(a, t.weights[0]) + mul_hi(b, t.weights[1]) + 2) >> 2;
            out.push(value.clamp(0, 255) as u8);
        }
    }
    out
}

// One destination index's two source indices and their fixed-point
// weights.
struct Tap {
    at: usize,
    next: usize,
    weights: [i32; 2],
}

// OpenCV computes f in double, stores it as float, and clamps at the
// edges: before the first source index the weight goes wholly to it, at
// or past the last likewise.
fn taps(src_len: usize, dst_len: usize) -> Vec<Tap> {
    let ratio = src_len as f64 / dst_len as f64;
    let last = src_len - 1;
    (0..dst_len)
        .map(|d| {
            let f = ((d as f64 + 0.5) * ratio - 0.5) as f32;
            let floor = f.floor();
            let (at, frac) = if floor < 0.0 {
                (0, 0.0)
            } else if floor as usize >= last {
                (last, 0.0)
            } else {
                (floor as usize, f - floor)
            };
            // saturate_cast<short>: round half to even.
            let weight = |w: f32| (w * COEF_SCALE).round_ties_even() as i32;
            Tap { at, next: (at + 1).min(last), weights: [weight(1.0 - frac), weight(frac)] }
        })
        .collect()
}
