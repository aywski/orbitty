use crate::lighting;
use crate::palette::Rgb;
use crate::planets::Planet;
use std::f64::consts::PI;
use std::io::{self, Write};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub fg: Rgb,
    pub bg: Rgb,
    pub ch: char,
}

impl Cell {
    const fn blank() -> Cell {
        Cell {
            fg: Rgb(0, 0, 0),
            bg: Rgb(0, 0, 0),
            ch: ' ',
        }
    }
}

// Fixed inclination of our "orbit" above the equator.
const ORBIT_TILT: f64 = 0.32;

// Fixed light direction (world space, normalized). Light comes from upper-right.
const SUN_DIR: [f64; 3] = [0.816, 0.408, 0.408];

// Number of background stars.
const NUM_STARS: usize = 180;

// Star field drift speed in radians per second.
const STAR_DRIFT: f64 = 0.006;

// Vertical amplification for moon orbits (compensates for terminal aspect ratio).
const MOON_V_SCALE: f64 = 1.7;

pub struct FallingStar {
    pub hx: f64,    // head x in cell space
    pub hy: f64,    // head y in cell space
    pub alpha: f64, // brightness 0..1
}

pub struct Scene<'a> {
    pub width: usize,
    pub height: usize,
    pub planet: &'a dyn Planet,
    pub spin: f64,
    pub orbit_angle: f64,
    pub zoom: f64,
    pub planet_name: &'a str,
    pub show_help: bool,
    pub planet_seed: Option<u64>,
    pub seed_input: Option<&'a str>,
    pub time: f64,
    pub falling_star: Option<FallingStar>,
}

fn view_basis(orbit_angle: f64, orbit_tilt: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let (oa_s, oa_c) = orbit_angle.sin_cos();
    let (ot_s, ot_c) = orbit_tilt.sin_cos();
    let cam = [oa_s * ot_c, ot_s, oa_c * ot_c];
    let look = [-cam[0], -cam[1], -cam[2]];
    let rx = -look[2];
    let rz = look[0];
    let rlen = (rx * rx + rz * rz).sqrt().max(1e-8);
    let right = [rx / rlen, 0.0, rz / rlen];
    let up = cross3(right, look);
    (right, up, look)
}

// Transform world-space planet-surface normal to lat/lon.
fn normal_to_lonlat(nx: f64, ny: f64, nz: f64, spin: f64) -> (f64, f64) {
    let lat = ny.clamp(-1.0, 1.0).asin();
    let lon = nx.atan2(nz) + spin;
    (lat, lon)
}

fn star_color(i: usize) -> Rgb {
    let h = star_rand(i as u32 * 3 + 2, 29);
    if h < 0.08 {
        Rgb(180, 180, 180)
    } else if h < 0.35 {
        Rgb(120, 108, 90)
    } else {
        Rgb(60, 63, 70)
    }
}

// Persistent per-pixel trail buffer: (r, g, b) at full saturation + intensity in [0, 1].
// Size must match width * pixel_h; resize (clearing) on dimension change is handled inside render().
pub type TrailBuf = Vec<(Rgb, f32)>;

pub fn render(scene: &Scene, trail: &mut TrailBuf) -> Vec<Cell> {
    let width = scene.width;
    let height = scene.height;
    let pixel_h = height * 2;

    let base = (width.min(pixel_h) as f64) * 0.40;
    let radius = (base * scene.zoom).max(1.0);
    let cx = width as f64 / 2.0;
    let cy = pixel_h as f64 / 2.0;

    let mut pixels: Vec<Option<Rgb>> = vec![None; width * pixel_h];

    // Resize trail if terminal dimensions changed (clears old data)
    let trail_size = width * pixel_h;
    if trail.len() != trail_size {
        trail.clear();
        trail.resize(trail_size, (Rgb(0, 0, 0), 0.0));
    }
    // Decay existing trail pixels 18% per frame; clear near-zero
    for p in trail.iter_mut() {
        p.1 *= 0.82;
        if p.1 < 0.018 {
            p.1 = 0.0;
        }
    }

    let (right, up, look) = view_basis(scene.orbit_angle, ORBIT_TILT);

    // Stars in cell-space using '·' character - visually smaller than half-block pixels.
    let mut star_layer: Vec<Option<(Rgb, bool)>> = vec![None; width * height];
    for i in 0..NUM_STARS {
        let azimuth = star_rand(i as u32 * 3, 7) * 2.0 * PI + scene.time * STAR_DRIFT;
        let elev_u = star_rand(i as u32 * 3 + 1, 13);
        let elevation = (2.0 * elev_u - 1.0).clamp(-1.0, 1.0).asin();

        let x_frac = ((azimuth / (2.0 * PI)).fract() + 1.0).fract();
        let y_frac = 0.5 - elevation / PI;

        let fx = x_frac * width as f64;
        let fy = y_frac * height as f64;
        let icx = fx as i64;
        let icy = fy as i64;
        if icx < 0 || icx >= width as i64 || icy < 0 || icy >= height as i64 {
            continue;
        }

        // Fractional distance to cell center -> dim star when it's between cells
        let dx = (fx - icx as f64 - 0.5).abs();
        let dy = (fy - icy as f64 - 0.5).abs();
        let sub_pixel = (1.0 - dx * 2.0) * (1.0 - dy * 2.0);

        let color = star_color(i).scale(sub_pixel);
        let size = star_rand(i as u32 * 3 + 5, 41);
        star_layer[icy as usize * width + icx as usize] = Some((color, size < 0.10));
    }

    let ring_cfg = scene.planet.ring_config();

    for py in 0..pixel_h {
        for px in 0..width {
            let dx = (px as f64 - cx + 0.5) / radius;
            let dy = -(py as f64 - cy + 0.5) / radius;
            let r2 = dx * dx + dy * dy;

            let mut sphere_color: Option<Rgb> = None;
            let mut sphere_z: f64 = f64::NEG_INFINITY;

            if r2 <= 1.0 {
                let z_front = (1.0 - r2).sqrt();
                sphere_z = z_front;

                let nx = dx * right[0] + dy * up[0] + z_front * (-look[0]);
                let ny = dx * right[1] + dy * up[1] + z_front * (-look[1]);
                let nz = dx * right[2] + dy * up[2] + z_front * (-look[2]);

                let (lat, lon) = normal_to_lonlat(nx, ny, nz, scene.spin);
                let color = scene.planet.surface_color(lat, lon);
                let lit = lighting::compute(nx, ny, nz, SUN_DIR);
                let limb = z_front.powf(0.3).max(0.18);
                sphere_color = Some(color.scale(lit * limb));
            }

            let mut ring_pixel: Option<(Rgb, f64)> = None;
            if let Some(ref rcfg) = ring_cfg {
                if look[1].abs() > 1e-4 {
                    let t_ring = (dx * right[1] + dy * up[1]) / look[1];
                    let rx_w = dx * right[0] + dy * up[0] + t_ring * (-look[0]);
                    let rz_w = dx * right[2] + dy * up[2] + t_ring * (-look[2]);
                    let ring_r = (rx_w * rx_w + rz_w * rz_w).sqrt();

                    if ring_r >= rcfg.inner && ring_r <= rcfg.outer {
                        let ring_phi = rz_w.atan2(rx_w);
                        let ring_c = ring_shade(
                            ring_r,
                            rcfg.inner,
                            rcfg.outer,
                            ring_phi,
                            scene.spin,
                            &rcfg.colors,
                        );
                        ring_pixel = Some((ring_c, t_ring));
                    }
                }
            }

            let chosen = match (sphere_color, ring_pixel) {
                (Some(sc), Some((rc, rz))) => {
                    if rz > sphere_z {
                        Some(rc)
                    } else {
                        Some(sc)
                    }
                }
                (Some(sc), None) => Some(sc),
                (None, Some((rc, _))) => Some(rc),
                (None, None) => None,
            };

            if let Some(c) = chosen {
                pixels[py * width + px] = Some(c);
            }
        }
    }

    render_moons(
        &mut pixels,
        trail,
        width,
        pixel_h,
        cx,
        cy,
        radius,
        scene,
        right,
        up,
        look,
    );

    // Write falling star head into trail buffer so it gets the same decay as moons
    if let Some(ref fs) = scene.falling_star {
        let px = fs.hx.round() as i64;
        let py = (fs.hy * 2.0).round() as i64;
        if px >= 0 && px < width as i64 && py >= 0 && py < pixel_h as i64 {
            let idx = py as usize * width + px as usize;
            let intensity = fs.alpha as f32;
            if intensity > trail[idx].1 {
                trail[idx] = (Rgb(255, 240, 210), intensity);
            }
        }
    }

    // Composite decayed trail into empty pixel slots (planet/rings naturally overwrite)
    for i in 0..pixels.len() {
        if pixels[i].is_none() && trail[i].1 > 0.0 {
            pixels[i] = Some(trail[i].0.scale(trail[i].1 as f64));
        }
    }

    let mut cells = pixels_to_cells(&pixels, width, pixel_h);

    for (idx, star) in star_layer.iter().enumerate() {
        if let Some((color, small)) = star {
            if cells[idx] == Cell::blank() {
                let ch = if *small { '◉' } else { '●' };
                cells[idx] = Cell {
                    fg: *color,
                    bg: Rgb(0, 0, 0),
                    ch,
                };
            }
        }
    }

    draw_title(&mut cells, width, scene.planet_name);
    if let Some(seed) = scene.planet_seed {
        draw_seed(&mut cells, width, seed);
    }
    if let Some(input) = scene.seed_input {
        draw_seed_input(&mut cells, width, height, input);
    }
    if scene.show_help {
        draw_help(&mut cells, width, height);
    }
    cells
}

fn moon_world_pos(r: f64, theta: f64, inclination: f64) -> [f64; 3] {
    [
        r * theta.cos(),
        r * theta.sin() * inclination.sin(),
        r * theta.sin() * inclination.cos(),
    ]
}

fn moon_occluded(pos: [f64; 3], right: [f64; 3], up: [f64; 3], look: [f64; 3]) -> bool {
    let m_dx = dot3(pos, right);
    let m_dy = dot3(pos, up);
    let r2_world = m_dx * m_dx + m_dy * m_dy;
    // Use screen-space disk check (with vertical scale) so occlusion matches what the user sees
    let r2_screen = m_dx * m_dx + (m_dy * MOON_V_SCALE) * (m_dy * MOON_V_SCALE);
    r2_screen <= 1.0 && dot3(pos, look) > (1.0 - r2_world).max(0.0).sqrt()
}

// Lit sphere disc, used for the moon itself
fn draw_disc_sphere(
    pixels: &mut Vec<Option<Rgb>>,
    width: usize,
    pixel_h: usize,
    cx: f64,
    cy: f64,
    eff_r: f64,
    color: Rgb,
    right: [f64; 3],
    up: [f64; 3],
    look: [f64; 3],
) {
    let x0 = (cx - eff_r - 1.0).floor() as i64;
    let x1 = (cx + eff_r + 1.0).ceil() as i64;
    let y0 = (cy - eff_r - 1.0).floor() as i64;
    let y1 = (cy + eff_r + 1.0).ceil() as i64;
    for py in y0..=y1 {
        for px in x0..=x1 {
            if px < 0 || px >= width as i64 || py < 0 || py >= pixel_h as i64 {
                continue;
            }
            let ddx = (px as f64 + 0.5 - cx) / eff_r;
            let ddy = -(py as f64 + 0.5 - cy) / eff_r;
            let r2 = ddx * ddx + ddy * ddy;
            if r2 > 1.0 {
                continue;
            }
            let z_front = (1.0 - r2).sqrt();
            let nx = ddx * right[0] + ddy * up[0] + z_front * (-look[0]);
            let ny = ddx * right[1] + ddy * up[1] + z_front * (-look[1]);
            let nz = ddx * right[2] + ddy * up[2] + z_front * (-look[2]);
            let lit = lighting::compute(nx, ny, nz, SUN_DIR);
            let limb = z_front.powf(0.3).max(0.18);
            pixels[py as usize * width + px as usize] = Some(color.scale(lit * limb));
        }
    }
}

fn render_moons(
    pixels: &mut Vec<Option<Rgb>>,
    trail: &mut TrailBuf,
    width: usize,
    pixel_h: usize,
    cx: f64,
    cy: f64,
    radius: f64,
    scene: &Scene,
    right: [f64; 3],
    up: [f64; 3],
    look: [f64; 3],
) {
    for moon in scene.planet.moons() {
        let theta = scene.spin * moon.speed + moon.phase;
        let r = moon.orbital_radius;
        let pos = moon_world_pos(r, theta, moon.inclination);
        if moon_occluded(pos, right, up, look) {
            continue;
        }
        let m_dx = dot3(pos, right);
        let m_dy = dot3(pos, up);
        let mx = cx + m_dx * radius;
        let my = cy - m_dy * radius * MOON_V_SCALE;
        let eff_r = (moon.radius * radius).max(0.6);

        // Write current position into trail buffer at full intensity
        let trail_r = (eff_r * 0.55).max(0.5);
        let tx0 = (mx - trail_r - 1.0).floor() as i64;
        let tx1 = (mx + trail_r + 1.0).ceil() as i64;
        let ty0 = (my - trail_r - 1.0).floor() as i64;
        let ty1 = (my + trail_r + 1.0).ceil() as i64;
        for py in ty0..=ty1 {
            for px in tx0..=tx1 {
                if px < 0 || px >= width as i64 || py < 0 || py >= pixel_h as i64 {
                    continue;
                }
                let dx = (px as f64 + 0.5 - mx) / trail_r;
                let dy = (py as f64 + 0.5 - my) / trail_r;
                if dx * dx + dy * dy <= 1.0 {
                    let idx = py as usize * width + px as usize;
                    trail[idx] = (moon.color, 1.0);
                }
            }
        }

        // Draw the moon with sphere shading into the pixel buffer
        draw_disc_sphere(
            pixels, width, pixel_h, mx, my, eff_r, moon.color, right, up, look,
        );
    }
}

fn ring_shade(r: f64, inner: f64, outer: f64, phi: f64, spin: f64, colors: &[Rgb; 5]) -> Rgb {
    let t = (r - inner) / (outer - inner);
    let stops = [0.0_f64, 0.30, 0.50, 0.65, 0.80, 1.00];

    let mut c = colors[0];
    for i in 0..stops.len() - 1 {
        if t >= stops[i] && t < stops[i + 1] {
            let f = (t - stops[i]) / (stops[i + 1] - stops[i]);
            c = colors[i].lerp(colors[(i + 1).min(4)], f);
            break;
        }
    }

    let edge = ((t - 0.85) / 0.15).clamp(0.0, 1.0);
    // Keplerian shimmer: inner rings orbit faster
    let angle = phi + spin * 1.2 / r.powf(1.5);
    let shimmer = angle.cos() * 0.08 + (angle * 2.5).cos() * 0.04;

    c.scale(((1.0 - edge * 0.6) * 0.82 + shimmer).clamp(0.50, 1.0))
}

fn pixels_to_cells(pixels: &[Option<Rgb>], width: usize, pixel_h: usize) -> Vec<Cell> {
    let height = pixel_h / 2;
    let mut cells = vec![Cell::blank(); width * height];
    for cy in 0..height {
        for cx in 0..width {
            let top = pixels[(cy * 2) * width + cx];
            let bot = pixels[(cy * 2 + 1) * width + cx];
            cells[cy * width + cx] = match (top, bot) {
                (Some(t), Some(b)) => Cell {
                    fg: t,
                    bg: b,
                    ch: '▀',
                },
                (Some(t), None) => Cell {
                    fg: t,
                    bg: Rgb(0, 0, 0),
                    ch: '▀',
                },
                (None, Some(b)) => Cell {
                    fg: b,
                    bg: Rgb(0, 0, 0),
                    ch: '▄',
                },
                (None, None) => Cell::blank(),
            };
        }
    }
    cells
}

fn put_str(
    cells: &mut [Cell],
    width: usize,
    row: usize,
    col0: usize,
    text: &str,
    fg: Rgb,
    bg: Rgb,
) {
    for (i, ch) in text.chars().enumerate() {
        let col = col0 + i;
        if col >= width {
            break;
        }
        cells[row * width + col] = Cell { fg, bg, ch };
    }
}

fn draw_title(cells: &mut [Cell], width: usize, name: &str) {
    let label = format!("  {}  ", name.to_uppercase());
    let w = label.chars().count();
    if w + 2 >= width {
        return;
    }
    let start = (width - w) / 2;
    put_str(
        cells,
        width,
        0,
        start,
        &label,
        Rgb(230, 230, 240),
        Rgb(0, 0, 0),
    );
    let bracket = Rgb(120, 120, 160);
    let bg = Rgb(0, 0, 0);
    let left = start.saturating_sub(1);
    let right = start + w;
    if left < width {
        cells[left] = Cell {
            fg: bracket,
            bg,
            ch: '[',
        };
    }
    if right < width {
        cells[right] = Cell {
            fg: bracket,
            bg,
            ch: ']',
        };
    }
}

fn draw_seed(cells: &mut [Cell], width: usize, seed: u64) {
    let label = format!("{:016x}", seed);
    let col0 = width.saturating_sub(label.len() + 1);
    put_str(cells, width, 0, col0, &label, Rgb(65, 65, 78), Rgb(0, 0, 0));
}

fn draw_seed_input(cells: &mut [Cell], width: usize, height: usize, input: &str) {
    let prompt = format!(" seed: {}_ ", input);
    let w = prompt.chars().count();
    if w + 2 >= width || height < 3 {
        return;
    }
    let col0 = (width.saturating_sub(w)) / 2;
    put_str(
        cells,
        width,
        height - 2,
        col0,
        &prompt,
        Rgb(220, 220, 240),
        Rgb(30, 30, 45),
    );
}

fn draw_help(cells: &mut [Cell], width: usize, height: usize) {
    let lines = [
        " Controls ",
        "",
        " 1-8    switch planet",
        " +  -   zoom in / out",
        " [  ]   rotation speed",
        " r      random planet",
        " s      enter seed",
        " h      toggle this help",
        " q      quit (or close help)",
    ];

    let inner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 4;
    let inner_h = lines.len() + 2;
    if inner_w + 2 > width || inner_h + 2 > height {
        return;
    }

    let x0 = (width as f32 * 0.01) as usize;
    let y0 = (height as f32 * 0.02) as usize;

    let border_fg = Rgb(200, 200, 230);
    let text_fg = Rgb(230, 230, 240);
    let title_fg = Rgb(255, 220, 140);

    for yy in 0..inner_h {
        for xx in 0..inner_w {
            let col = x0 + xx;
            let row = y0 + yy;
            if col >= width || row >= height {
                continue;
            }
            let is_border = yy == 0 || yy == inner_h - 1 || xx == 0 || xx == inner_w - 1;
            if !is_border {
                continue;
            }
            let ch = if yy == 0 && xx == 0 {
                '╭'
            } else if yy == 0 && xx == inner_w - 1 {
                '╮'
            } else if yy == inner_h - 1 && xx == 0 {
                '╰'
            } else if yy == inner_h - 1 && xx == inner_w - 1 {
                '╯'
            } else if yy == 0 || yy == inner_h - 1 {
                '─'
            } else {
                '│'
            };
            let bg = cells[row * width + col].bg;
            cells[row * width + col] = Cell {
                fg: border_fg,
                bg,
                ch,
            };
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let row = y0 + 1 + i;
        if row >= height {
            break;
        }
        let fg = if i == 0 { title_fg } else { text_fg };
        for (j, ch) in line.chars().enumerate() {
            let col = x0 + 2 + j;
            if col >= width - 1 {
                break;
            }
            let bg = cells[row * width + col].bg.scale(0.35);
            cells[row * width + col] = Cell { fg, bg, ch };
        }
    }
}

fn star_rand(x: u32, y: u32) -> f64 {
    let mut h = x.wrapping_mul(0x27d4eb2d);
    h ^= y.wrapping_mul(0x165667b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    (h as f64) / (u32::MAX as f64)
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn flush(
    stdout: &mut impl Write,
    frame: &[Cell],
    prev: &[Cell],
    width: usize,
    height: usize,
) -> io::Result<()> {
    let mut buf = String::with_capacity(frame.len() * 20);

    for row in 0..height {
        for col in 0..width {
            let idx = row * width + col;
            let cell = &frame[idx];
            let old = prev.get(idx);
            if old == Some(cell) {
                continue;
            }
            buf.push_str(&format!("\x1b[{};{}H", row + 1, col + 1));
            buf.push_str(&cell.bg.bg_seq());
            buf.push_str(&cell.fg.fg_seq());
            buf.push(cell.ch);
        }
    }

    buf.push_str("\x1b[0m");
    stdout.write_all(buf.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
