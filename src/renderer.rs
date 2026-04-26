use std::io::{self, Write};
use std::f64::consts::PI;
use crate::palette::Rgb;
use crate::planets::Planet;
use crate::lighting;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cell {
    pub fg: Rgb,
    pub bg: Rgb,
    pub ch: char,
}

impl Cell {
    fn blank() -> Cell {
        Cell { fg: Rgb(0, 0, 0), bg: Rgb(0, 0, 0), ch: ' ' }
    }
}

// Fixed inclination of our "orbit" above the equator.
const ORBIT_TILT: f64 = 0.32;

// Fixed light direction (world space, normalized). Light comes from upper-right.
const SUN_DIR: [f64; 3] = [0.816, 0.408, 0.408];

// Number of background stars.
const NUM_STARS: usize = 180;

pub struct Scene<'a> {
    pub width: usize,
    pub height: usize,
    pub planet: &'a dyn Planet,
    pub spin: f64,
    pub orbit_angle: f64,
    pub zoom: f64,
    pub planet_name: &'a str,
    pub show_help: bool,
}

// View basis vectors (right, up, look_toward_planet) from orbit parameters.
fn view_basis(orbit_angle: f64, orbit_tilt: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
    let (oa_s, oa_c) = orbit_angle.sin_cos();
    let (ot_s, ot_c) = orbit_tilt.sin_cos();

    // Camera sits at this direction from planet center.
    let cam = [oa_s * ot_c, ot_s, oa_c * ot_c];

    // Look direction: toward planet = -cam.
    let look = [-cam[0], -cam[1], -cam[2]];

    // right = cross(look, world_up), world_up = (0,1,0)
    // cross((lx,ly,lz), (0,1,0)) = (ly*0 - lz*1, lz*0 - lx*0, lx*1 - ly*0) = (-lz, 0, lx)
    let rx = -look[2];
    let ry = 0.0;
    let rz = look[0];
    let rlen = (rx * rx + rz * rz).sqrt().max(1e-8);
    let right = [rx / rlen, ry, rz / rlen];

    // true_up = cross(right, look)
    let up = cross3(right, look);

    (right, up, look)
}

// Transform world-space planet-surface normal to lat/lon.
fn normal_to_lonlat(nx: f64, ny: f64, nz: f64, spin: f64) -> (f64, f64) {
    let lat = ny.clamp(-1.0, 1.0).asin();
    let lon = nx.atan2(nz) + spin;
    (lat, lon)
}

// Star brightness/color.
fn star_brightness(i: usize) -> (u8, u8) {
    let h = star_rand(i as u32 * 3 + 2, 29);
    let b = if h < 0.08 {
        (180u8, 0u8)  // bright white
    } else if h < 0.35 {
        (120u8, 1u8)  // medium warm
    } else {
        (70u8, 2u8)   // dim blue-white
    };
    b
}

pub fn render(scene: &Scene) -> Vec<Cell> {
    let width = scene.width;
    let height = scene.height;
    let pixel_h = height * 2;

    let base = (width.min(pixel_h) as f64) * 0.40;
    let radius = (base * scene.zoom).max(1.0);
    let cx = width as f64 / 2.0;
    let cy = pixel_h as f64 / 2.0;

    let mut pixels: Vec<Option<Rgb>> = vec![None; width * pixel_h];

    let (right, up, look) = view_basis(scene.orbit_angle, ORBIT_TILT);

    // Stars in cell-space using '·' character - visually smaller than half-block pixels.
    let mut star_layer: Vec<Option<(Rgb, bool)>> = vec![None; width * height];
    let star_rotation = scene.spin;

    for i in 0..NUM_STARS {
        let azimuth = star_rand(i as u32 * 3, 7) * 2.0 * PI + star_rotation;
        let elev_u = star_rand(i as u32 * 3 + 1, 13);
        let elevation = (2.0 * elev_u - 1.0).clamp(-1.0, 1.0).asin();

        let x_frac = ((azimuth / (2.0 * PI)).fract() + 1.0).fract();
        let y_frac = 0.5 - elevation / PI;

        let icx = (x_frac * width as f64) as i64;
        let icy = (y_frac * height as f64) as i64;
        if icx < 0 || icx >= width as i64 || icy < 0 || icy >= height as i64 { continue; }

        let (brightness, kind) = star_brightness(i);
        let color = match kind {
            0 => Rgb(brightness, brightness, brightness),
            1 => Rgb(brightness, (brightness as f64 * 0.9) as u8, (brightness as f64 * 0.75) as u8),
            _ => Rgb((brightness as f64 * 0.85) as u8, (brightness as f64 * 0.9) as u8, brightness),
        };
        let size = star_rand(i as u32 * 3 + 5, 41);
        star_layer[icy as usize * width + icx as usize] = Some((color, size < 0.10));
    }

    let has_rings = scene.planet.has_rings();
    let ring_inner = 1.25;
    let ring_outer = 2.10;

    for py in 0..pixel_h {
        for px in 0..width {
            let dx = (px as f64 - cx + 0.5) / radius;
            let dy = (py as f64 - cy + 0.5) / radius;
            let r2 = dx * dx + dy * dy;

            let mut sphere_color: Option<Rgb> = None;
            let mut sphere_z: f64 = f64::NEG_INFINITY;

            if r2 <= 1.0 {
                let z_front = (1.0 - r2).sqrt();
                sphere_z = z_front;

                // Reconstruct world-space normal: screen coords (dx, dy) with z_front = z_cam+.
                // n_world = dx*right + dy*up + z_front*(-look)
                let nx = dx * right[0] + dy * up[0] + z_front * (-look[0]);
                let ny = dx * right[1] + dy * up[1] + z_front * (-look[1]);
                let nz = dx * right[2] + dy * up[2] + z_front * (-look[2]);

                let (lat, lon) = normal_to_lonlat(nx, ny, nz, scene.spin);
                let color = scene.planet.surface_color(lat, lon);
                let lit = lighting::compute(nx, ny, nz, SUN_DIR);

                // Limb darkening: edges are darker (shadow effect)
                let limb = z_front.powf(0.3).max(0.18);
                sphere_color = Some(color.scale(lit * limb));
            }

            // Rings (Saturn).
            let mut ring_pixel: Option<(Rgb, f64)> = None;
            if has_rings {
                // Ring plane is body Y=0. We need to intersect the ray.
                // Ray in world: P(t) = (dx*right + dy*up) + t*(-look), orthographic.
                // n_world = dx*right + dy*up + t*(-look).
                // y_world = 0: dx*up[1] + dy*up[1] ... no wait:
                // y_world = dx*right[1] + dy*up[1] + t*(-look[1]) = 0
                // t = (dx*right[1] + dy*up[1]) / look[1]
                if look[1].abs() > 1e-4 {
                    let t_ring = (dx * right[1] + dy * up[1]) / look[1];
                    // World pos of ring hit: P = dx*right + dy*up + t*(-look)
                    let rx_w = dx * right[0] + dy * up[0] + t_ring * (-look[0]);
                    let rz_w = dx * right[2] + dy * up[2] + t_ring * (-look[2]);
                    let ring_r = (rx_w * rx_w + rz_w * rz_w).sqrt();

                    if ring_r >= ring_inner && ring_r <= ring_outer {
                        let ring_phi = rz_w.atan2(rx_w);
                        let ring_c = ring_shade(ring_r, ring_inner, ring_outer, ring_phi, scene.spin);
                        ring_pixel = Some((ring_c, t_ring));
                    }
                }
            }

            let chosen = match (sphere_color, ring_pixel) {
                (Some(sc), Some((rc, rz))) => {
                    if rz > sphere_z { Some(rc) } else { Some(sc) }
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

    let mut cells = pixels_to_cells(&pixels, width, pixel_h);

    for (idx, star) in star_layer.iter().enumerate() {
        if let Some((color, small)) = star {
            if cells[idx] == Cell::blank() {
                let ch = if *small { '◉' } else { '●' };
                cells[idx] = Cell { fg: *color, bg: Rgb(0, 0, 0), ch };
            }
        }
    }

    draw_title(&mut cells, width, scene.planet_name);
    if scene.show_help {
        draw_help(&mut cells, width, height);
    }
    cells
}

fn ring_shade(r: f64, inner: f64, outer: f64, phi: f64, spin: f64) -> Rgb {
    let t = (r - inner) / (outer - inner);

    let stops = [0.0_f64, 0.30, 0.50, 0.65, 0.80, 1.00];
    let colors = [
        Rgb(215, 190, 145), Rgb(175, 145, 100), Rgb(210, 185, 140),
        Rgb(165, 135, 95),  Rgb(205, 178, 132),
    ];

    let mut c = colors[0];
    for i in 0..stops.len() - 1 {
        if t >= stops[i] && t < stops[i + 1] {
            let f = (t - stops[i]) / (stops[i + 1] - stops[i]);
            c = colors[i].lerp(colors[(i + 1).min(4)], f);
            break;
        }
    }

    let edge = ((t - 0.85) / 0.15).clamp(0.0, 1.0);

    // Keplerian motion: inner rings orbit faster, creating slowly sweeping brightness variation.
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
                (Some(t), Some(b)) => Cell { fg: t, bg: b, ch: '▀' },
                (Some(t), None) => Cell { fg: t, bg: Rgb(0, 0, 0), ch: '▀' },
                (None, Some(b)) => Cell { fg: b, bg: Rgb(0, 0, 0), ch: '▄' },
                (None, None) => Cell::blank(),
            };
        }
    }
    cells
}

fn draw_title(cells: &mut [Cell], width: usize, name: &str) {
    let label = format!("  {}  ", name.to_uppercase());
    let w = label.chars().count();
    if w + 2 >= width { return; }
    let start = (width - w) / 2;
    let fg = Rgb(230, 230, 240);
    let bg = Rgb(0, 0, 0);
    for (i, ch) in label.chars().enumerate() {
        let col = start + i;
        if col < width {
            cells[col] = Cell { fg, bg, ch };
        }
    }
    let left = start.saturating_sub(1);
    let right = start + w;
    if left < width { cells[left] = Cell { fg: Rgb(120, 120, 160), bg: Rgb(0, 0, 0), ch: '[' }; }
    if right < width { cells[right] = Cell { fg: Rgb(120, 120, 160), bg: Rgb(0, 0, 0), ch: ']' }; }
}

fn draw_help(cells: &mut [Cell], width: usize, height: usize) {
    let lines = [
        " Controls ",
        "",
        " 1-8    switch planet",
        " +  -   zoom in / out",
        " [  ]   rotation speed",
        " r      reset rotation",
        " h      toggle this help",
        " q      quit (or close help)",
    ];

    let inner_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 4;
    let inner_h = lines.len() + 2;
    if inner_w + 2 > width || inner_h + 2 > height { return; }

    let x0 = (width as f32 * 0.01) as usize;
    let y0 = (height as f32 * 0.02) as usize;

    let border_fg = Rgb(200, 200, 230);
    let text_fg = Rgb(230, 230, 240);
    let title_fg = Rgb(255, 220, 140);

    for yy in 0..inner_h {
        for xx in 0..inner_w {
            let col = x0 + xx;
            let row = y0 + yy;
            if col >= width || row >= height { continue; }
            let is_border = yy == 0 || yy == inner_h - 1 || xx == 0 || xx == inner_w - 1;
            if !is_border { continue; }
            let ch = if yy == 0 && xx == 0 { '╭' }
                else if yy == 0 && xx == inner_w - 1 { '╮' }
                else if yy == inner_h - 1 && xx == 0 { '╰' }
                else if yy == inner_h - 1 && xx == inner_w - 1 { '╯' }
                else if yy == 0 || yy == inner_h - 1 { '─' }
                else { '│' };
            let bg = cells[row * width + col].bg;
            cells[row * width + col] = Cell { fg: border_fg, bg, ch };
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let row = y0 + 1 + i;
        if row >= height { break; }
        let fg = if i == 0 { title_fg } else { text_fg };
        for (j, ch) in line.chars().enumerate() {
            let col = x0 + 2 + j;
            if col >= width - 1 { break; }
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
            if old == Some(cell) { continue; }
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
