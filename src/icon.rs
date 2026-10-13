//! Procedural energy-drink-can icon, shared by the tray (`main.rs`) and the
//! executable's icon resource (`build.rs`). Coordinates are in the unit square.

const SAMPLES: u32 = 6;

const BLUE: [f32; 3] = [0.07, 0.22, 0.58];
const SILVER: [f32; 3] = [0.80, 0.82, 0.85];
const LID: [f32; 3] = [0.62, 0.64, 0.68];
const YELLOW: [f32; 3] = [1.00, 0.80, 0.05];
const RED: [f32; 3] = [0.86, 0.04, 0.18];

const CENTER_X: f32 = 0.5;
const LOGO_Y: f32 = 0.53;

/// Renders a `size`x`size` straight-alpha RGBA image. The inactive variant is
/// a faded greyscale version of the same can.
pub fn render(size: u32, active: bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let step = 1.0 / (size * SAMPLES) as f32;
    for py in 0..size {
        for px in 0..size {
            let mut sum = [0.0f32; 3];
            let mut hits = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = ((px * SAMPLES + sx) as f32 + 0.5) * step;
                    let y = ((py * SAMPLES + sy) as f32 + 0.5) * step;
                    if let Some(color) = shade(x, y) {
                        for (acc, channel) in sum.iter_mut().zip(color) {
                            *acc += channel;
                        }
                        hits += 1;
                    }
                }
            }
            if hits == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let mut color = sum.map(|channel| channel / hits as f32);
            let mut alpha = hits as f32 / (SAMPLES * SAMPLES) as f32;
            if !active {
                let grey = 0.30 * color[0] + 0.59 * color[1] + 0.11 * color[2];
                color = [grey; 3];
                alpha *= 0.65;
            }
            rgba.extend(color.map(to_byte));
            rgba.push(to_byte(alpha));
        }
    }
    rgba
}

fn to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Half-width of the can's silhouette at height `y`, or `None` outside it.
fn half_width(y: f32) -> Option<f32> {
    let lerp = |from: f32, to: f32, start: f32, end: f32| from + (to - from) * (y - start) / (end - start);
    match y {
        y if y < 0.03 => None,
        y if y < 0.07 => Some(0.20),
        y if y < 0.17 => Some(lerp(0.20, 0.27, 0.07, 0.17)),
        y if y < 0.91 => Some(0.27),
        y if y < 0.97 => Some(lerp(0.27, 0.22, 0.91, 0.97)),
        _ => None,
    }
}

fn in_ellipse(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> bool {
    let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
    dx * dx + dy * dy <= 1.0
}

/// One charging bull, drawn for the left side; `x` is mirrored by the caller.
fn in_bull(x: f32, y: f32) -> bool {
    in_ellipse(x, y, 0.345, LOGO_Y, 0.075, 0.050) // body
        || in_ellipse(x, y, 0.375, LOGO_Y - 0.040, 0.045, 0.035) // hump
        || in_ellipse(x, y, 0.435, LOGO_Y + 0.020, 0.035, 0.030) // head
        || in_ellipse(x, y, 0.445, LOGO_Y - 0.025, 0.012, 0.030) // horn
}

fn shade(x: f32, y: f32) -> Option<[f32; 3]> {
    let half = half_width(y)?;
    let t = (x - CENTER_X) / half;
    if t.abs() > 1.0 {
        return None;
    }

    let mirrored_x = CENTER_X - (x - CENTER_X).abs();
    let base = if y < 0.07 {
        LID
    } else if !(0.17..0.91).contains(&y) {
        SILVER
    } else if in_bull(mirrored_x, y) {
        RED
    } else if in_ellipse(x, y, CENTER_X, LOGO_Y, 0.105, 0.105) {
        YELLOW
    } else {
        // Blue and silver quadrants split by a slanted seam through the logo.
        let above_seam = y < LOGO_Y - 0.35 * (x - CENTER_X);
        if (x < CENTER_X) == above_seam { BLUE } else { SILVER }
    };

    // Cylindrical shading: darker towards the edges, highlight left of centre.
    let highlight = (-((t + 0.45) / 0.22).powi(2)).exp();
    let light = 1.0 - 0.50 * t * t + 0.22 * highlight;
    Some(base.map(|channel| channel * light))
}
