//! Procedural energy-drink-can icon with folio's cut gem in the bottom right corner,
//! shared by the tray (`main.rs`) and the executable's icon resource (`build.rs`).
//! Coordinates are in the unit square.

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
                    if let Some(color) = gem(x, y).or_else(|| shade(x, y)) {
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

// The gem from folio's icon (`assets/icon/folio.svg`), drawn in its own 100x150 box and
// placed over the bottom right corner exactly as folio places it over its sheet.
const GEM_ORIGIN: (f32, f32) = (178.0 / 256.0, 151.0 / 256.0);
const GEM_SCALE: f32 = 0.64 / 256.0;

const GEM_OUTLINE: [(f32, f32); 10] = [
    (48.0, 2.0), (75.0, 22.0), (95.0, 52.0), (97.0, 84.0), (84.0, 117.0),
    (58.0, 148.0), (32.0, 133.0), (11.0, 106.0), (3.0, 72.0), (19.0, 35.0),
];
const GEM_RIM_WIDTH: f32 = 7.0;
const GEM_BODY: u32 = 0x343a9e;
const GEM_RIM_LIT: u32 = 0x8f9de9;
const GEM_RIM_DARK: u32 = 0x1d2176;

/// Facets as (colour, corners), the one four-sided facet included.
const GEM_FACETS: [(u32, &[(f32, f32)]); 21] = [
    (0xe9eefd, &[(48.0, 2.0), (75.0, 22.0), (45.0, 30.0)]),
    (0xd3dcf8, &[(48.0, 2.0), (45.0, 30.0), (19.0, 35.0)]),
    (0x8494e3, &[(75.0, 22.0), (95.0, 52.0), (64.0, 48.0)]),
    (0xf7f9ff, &[(75.0, 22.0), (64.0, 48.0), (45.0, 30.0)]),
    (0x5566cf, &[(95.0, 52.0), (97.0, 84.0), (77.0, 93.0)]),
    (0x7889df, &[(95.0, 52.0), (77.0, 93.0), (64.0, 48.0)]),
    (0xc2cdf5, &[(19.0, 35.0), (45.0, 30.0), (36.0, 62.0)]),
    (0xa3b2ee, &[(19.0, 35.0), (36.0, 62.0), (3.0, 72.0)]),
    (0xffffff, &[(45.0, 30.0), (64.0, 48.0), (59.0, 80.0), (36.0, 62.0)]),
    (0xc9d4f7, &[(64.0, 48.0), (77.0, 93.0), (59.0, 80.0)]),
    (0xc6d1f6, &[(3.0, 72.0), (36.0, 62.0), (38.0, 101.0)]),
    (0x8b9be5, &[(3.0, 72.0), (38.0, 101.0), (11.0, 106.0)]),
    (0xf3f6ff, &[(36.0, 62.0), (59.0, 80.0), (38.0, 101.0)]),
    (0xe4eafc, &[(59.0, 80.0), (56.0, 121.0), (38.0, 101.0)]),
    (0x6f80da, &[(59.0, 80.0), (77.0, 93.0), (56.0, 121.0)]),
    (0x3f4bb8, &[(97.0, 84.0), (84.0, 117.0), (77.0, 93.0)]),
    (0x4b59c4, &[(84.0, 117.0), (56.0, 121.0), (77.0, 93.0)]),
    (0x2c349f, &[(84.0, 117.0), (58.0, 148.0), (56.0, 121.0)]),
    (0x5564cc, &[(58.0, 148.0), (32.0, 133.0), (56.0, 121.0)]),
    (0xaebbf0, &[(32.0, 133.0), (38.0, 101.0), (56.0, 121.0)]),
    (0x7485dd, &[(32.0, 133.0), (11.0, 106.0), (38.0, 101.0)]),
];

/// The ridges where facets meet, which catch the light.
const GEM_RIDGES: [[(f32, f32); 2]; 8] = [
    [(48.0, 2.0), (45.0, 30.0)],
    [(45.0, 30.0), (64.0, 48.0)],
    [(45.0, 30.0), (36.0, 62.0)],
    [(36.0, 62.0), (59.0, 80.0)],
    [(64.0, 48.0), (59.0, 80.0)],
    [(59.0, 80.0), (38.0, 101.0)],
    [(59.0, 80.0), (56.0, 121.0)],
    [(36.0, 62.0), (38.0, 101.0)],
];

/// A streak of reflection down the lit flank.
const GEM_STREAK: [(f32, f32); 4] = [(17.0, 52.0), (25.0, 43.0), (29.0, 73.0), (22.0, 81.0)];

fn rgb(hex: u32) -> [f32; 3] {
    [hex >> 16, hex >> 8, hex].map(|channel| (channel & 0xff) as f32 / 255.0)
}

fn mix(under: [f32; 3], over: [f32; 3], alpha: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| under[i] + (over[i] - under[i]) * alpha)
}

/// Even-odd point-in-polygon test.
fn in_polygon(x: f32, y: f32, corners: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut previous = corners[corners.len() - 1];
    for &corner in corners {
        if (corner.1 > y) != (previous.1 > y)
            && x < corner.0 + (previous.0 - corner.0) * (y - corner.1) / (previous.1 - corner.1)
        {
            inside = !inside;
        }
        previous = corner;
    }
    inside
}

fn distance_to_segment(x: f32, y: f32, (ax, ay): (f32, f32), (bx, by): (f32, f32)) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (x - ax - t * dx).hypot(y - ay - t * dy)
}

/// Linear interpolation through (offset, value) stops.
fn ramp(t: f32, stops: &[(f32, f32)]) -> f32 {
    let t = t.clamp(0.0, 1.0);
    stops
        .windows(2)
        .find(|pair| t <= pair[1].0)
        .map(|pair| pair[0].1 + (pair[1].1 - pair[0].1) * (t - pair[0].0) / (pair[1].0 - pair[0].0))
        .unwrap_or(stops[stops.len() - 1].1)
}

/// The gem's colour at a point, or `None` where the can shows through.
fn gem(x: f32, y: f32) -> Option<[f32; 3]> {
    let (gx, gy) = ((x - GEM_ORIGIN.0) / GEM_SCALE, (y - GEM_ORIGIN.1) / GEM_SCALE);
    let edges = GEM_OUTLINE.iter().zip(GEM_OUTLINE.iter().cycle().skip(1));

    if !in_polygon(gx, gy, &GEM_OUTLINE) {
        // The outer half of the rim, lit along the upper left and dark below.
        let edge = edges.map(|(&a, &b)| distance_to_segment(gx, gy, a, b)).fold(f32::MAX, f32::min);
        if edge > GEM_RIM_WIDTH / 2.0 {
            return None;
        }
        let t = ((gx - 3.0) / 94.0 + (gy - 2.0) / 146.0) / 2.0;
        return Some(mix(rgb(GEM_RIM_LIT), rgb(GEM_RIM_DARK), t.clamp(0.0, 1.0)));
    }

    let facet = GEM_FACETS.iter().find(|(_, corners)| in_polygon(gx, gy, corners));
    let mut color = rgb(facet.map_or(GEM_BODY, |(hex, _)| *hex));

    // Shade towards the lower right and a soft glow at the upper left, so the stone reads
    // as a rounded body rather than a flat pattern.
    let shade = ((gx - 84.0) / 80.0).hypot((gy - 129.0) / 120.0);
    let shade_color = mix(rgb(0x14176a), rgb(0x1c2080), (shade / 0.55).min(1.0));
    color = mix(color, shade_color, ramp(shade, &[(0.0, 0.8), (0.55, 0.3), (1.0, 0.0)]));
    let glow = ((gx - 36.0) / 50.0).hypot((gy - 51.0) / 75.0);
    color = mix(color, [1.0; 3], ramp(glow, &[(0.0, 0.95), (0.45, 0.35), (1.0, 0.0)]));

    if GEM_RIDGES.iter().any(|&[a, b]| distance_to_segment(gx, gy, a, b) <= 0.7) {
        color = mix(color, [1.0; 3], 0.7);
    }
    if in_polygon(gx, gy, &GEM_STREAK) {
        color = mix(color, [1.0; 3], 0.55);
    }
    Some(color)
}
