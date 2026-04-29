use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, Moon, RingConfig, noise2};

pub struct RandomPlanet {
    pub name: String,
    base_color: Rgb,    // dominant surface / ocean
    alt_color: Rgb,     // secondary terrain
    high_color: Rgb,    // highlands / peaks
    pole_color: Rgb,    // polar caps
    style: u8,          // 0=rocky 1=smooth 2=terrestrial 3=volcanic
    ring: Option<RingConfig>,
    stored_moons: Vec<Moon>,
    no: f64,            // primary noise offset
    no2: f64,           // secondary noise offset (domain warp / detail)
    terrain_scale: f64,
    polar_strength: f64,
}

fn rng(seed: u64, i: u64) -> f64 {
    let mut h = seed.wrapping_add(i.wrapping_mul(0x9e3779b97f4a7c15));
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    (h as f64) / (u64::MAX as f64)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h / 60.0) % 6.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if hp < 1.0 { (c, x, 0.0) }
        else if hp < 2.0 { (x, c, 0.0) }
        else if hp < 3.0 { (0.0, c, x) }
        else if hp < 4.0 { (0.0, x, c) }
        else if hp < 5.0 { (x, 0.0, c) }
        else { (c, 0.0, x) };
    Rgb(
        ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

// Fractal Brownian Motion: sums noise octaves at increasing frequency, decreasing amplitude.
fn fbm(x: f64, y: f64, octaves: u32) -> f64 {
    let mut val = 0.0;
    let mut amp = 0.50;
    let mut freq = 1.0;
    for _ in 0..octaves {
        val += noise2(x * freq, y * freq) * amp;
        amp *= 0.50;
        freq *= 2.1;
    }
    val // approx [0, 0.97] for 4 octaves; treat as ~[0, 1]
}

fn gen_name(seed: u64) -> String {
    const CONS: &[&str] = &[
        "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "r", "s", "t", "v", "x", "z",
        "th", "sh", "ch", "kr", "tr", "gr", "vr", "zr",
    ];
    const VOWS: &[&str] = &["a", "e", "i", "o", "u", "ae", "io", "ya", "ei", "ou"];
    let nsyl = 2 + (rng(seed, 0) * 3.0) as usize;
    let mut name = String::new();
    for i in 0..nsyl {
        let ci = (rng(seed, 10 + i as u64) * CONS.len() as f64) as usize % CONS.len();
        let vi = (rng(seed, 20 + i as u64) * VOWS.len() as f64) as usize % VOWS.len();
        name.push_str(CONS[ci]);
        name.push_str(VOWS[vi]);
        if rng(seed, 30 + i as u64) < 0.35 {
            let ci2 = (rng(seed, 40 + i as u64) * CONS.len() as f64) as usize % CONS.len();
            name.push_str(CONS[ci2]);
        }
    }
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn gen_ring_colors(hue: f64, seed: u64) -> [Rgb; 5] {
    let s = 0.25 + rng(seed, 700) * 0.45;
    let l_base = 0.38 + rng(seed, 701) * 0.22;
    [
        hsl_to_rgb(hue, s, l_base * 1.05),
        hsl_to_rgb(hue, s * 0.65, l_base * 0.72),
        hsl_to_rgb(hue, s, l_base),
        hsl_to_rgb(hue, s * 0.55, l_base * 0.68),
        hsl_to_rgb(hue, s * 0.90, l_base * 0.90),
    ]
}

pub fn make(seed: u64) -> RandomPlanet {
    let style = (rng(seed, 8) * 4.0) as u8; // 0=rocky 1=smooth 2=terrestrial 3=volcanic

    let hue = rng(seed, 1) * 360.0;
    // Consistent saturation across the whole palette - no mixing saturated with desaturated
    // Cap saturation: real planets are rarely neon, 0.20-0.55 covers the whole range
    let sat = 0.20 + rng(seed, 2) * 0.35;

    let hue_range = if style == 2 { 18.0 } else { 10.0 };
    let alt_hue = (hue + (rng(seed, 4) * 2.0 - 1.0) * hue_range).rem_euclid(360.0);

    // Lightness: strict monotone progression dark < mid < light < pale
    // Raised floor to 0.20 so nothing goes near-black on non-volcanic styles
    let lum_mid = 0.36 + rng(seed, 3) * 0.18;
    let lum_dark = (lum_mid - 0.08 - rng(seed, 63) * 0.10).max(0.20);
    let lum_light = (lum_mid + 0.16 + rng(seed, 64) * 0.12).min(0.86);
    let lum_pale = (lum_light + 0.08 + rng(seed, 65) * 0.10).min(0.94);

    // Saturation decreases with lightness
    let sat_alt = (sat * (0.85 + rng(seed, 66) * 0.20)).min(1.0);
    let sat_high = sat * (0.30 + rng(seed, 67) * 0.20);
    let sat_pole = sat * (0.10 + rng(seed, 68) * 0.12);

    let (base_color, alt_color, high_color, pole_color) = if style == 3 {
        // Volcanic: dark crust + bright lava, lava stays near base hue
        // Crust min 0.14 so it's dark but not pure black
        let lava_hue = (hue + (rng(seed, 4) * 2.0 - 1.0) * 15.0).rem_euclid(360.0);
        (
            hsl_to_rgb(hue, sat * 0.45, 0.14 + rng(seed, 3) * 0.08),
            hsl_to_rgb(lava_hue, (sat * 1.4).min(0.82), 0.48 + rng(seed, 69) * 0.10),
            hsl_to_rgb(lava_hue, (sat * 1.5).min(0.85), 0.60 + rng(seed, 70) * 0.08),
            hsl_to_rgb(hue, sat * 0.18, (lum_mid + 0.35).min(0.92)),
        )
    } else {
        (
            hsl_to_rgb(hue, sat, lum_mid),
            hsl_to_rgb(alt_hue, sat_alt, lum_dark),
            hsl_to_rgb(hue, sat_high, lum_light),
            hsl_to_rgb(hue, sat_pole, lum_pale),
        )
    };

    let no = rng(seed, 9) * 800.0 + 50.0;
    let no2 = rng(seed, 62) * 800.0 + 50.0;
    let terrain_scale = 1.8 + rng(seed, 60) * 2.2;
    let polar_strength = 0.40 + rng(seed, 61) * 0.55;

    let ring = if rng(seed, 11) < 0.35 {
        let ring_inner = 1.20 + rng(seed, 12) * 0.30;
        let ring_outer = ring_inner + 0.50 + rng(seed, 13) * 0.80;
        let ring_hue = (hue + (rng(seed, 14) * 2.0 - 1.0) * 12.0).rem_euclid(360.0);
        Some(RingConfig {
            inner: ring_inner,
            outer: ring_outer,
            colors: gen_ring_colors(ring_hue, seed),
        })
    } else {
        None
    };

    let num_moons = (rng(seed, 15) * 5.5) as usize;
    let ring_outer_r = ring.as_ref().map(|r| r.outer).unwrap_or(1.0);
    let mut stored_moons = Vec::new();
    for i in 0..num_moons {
        let orbit_r = ring_outer_r + 0.30 + i as f64 * 0.80 + rng(seed, 600 + i as u64) * 0.50;
        let moon_hue = (hue + rng(seed, 500 + i as u64) * 60.0 - 30.0).rem_euclid(360.0);
        let moon_color = hsl_to_rgb(
            moon_hue,
            0.10 + rng(seed, 510 + i as u64) * 0.30,
            0.50 + rng(seed, 520 + i as u64) * 0.30,
        );
        stored_moons.push(Moon {
            color: moon_color,
            radius: 0.03 + rng(seed, 530 + i as u64) * 0.06,
            orbital_radius: orbit_r,
            inclination: rng(seed, 540 + i as u64) * 0.40,
            speed: 0.05 + rng(seed, 550 + i as u64) * 1.50,
            phase: rng(seed, 560 + i as u64) * 2.0 * PI,
        });
    }

    RandomPlanet {
        name: gen_name(seed),
        base_color,
        alt_color,
        high_color,
        pole_color,
        style,
        ring,
        stored_moons,
        no,
        no2,
        terrain_scale,
        polar_strength,
    }
}

impl Planet for RandomPlanet {
    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        let t_abs = (lat / (PI / 2.0)).abs().clamp(0.0, 1.0);
        let (no, no2, ts) = (self.no, self.no2, self.terrain_scale);

        match self.style {
            0 => {
                // Rocky: FBM elevation map drives 4-stop color gradient
                let h = fbm(lon * ts + no, lat * ts + no, 4);
                let detail = (noise2(lon * ts * 7.0 + no2, lat * ts * 7.0 + no2) - 0.5) * 0.07;
                let elev = (h + detail).clamp(0.0, 1.0);
                let c = if elev < 0.28 {
                    self.base_color.scale(0.55 + elev * 1.6)
                } else if elev < 0.56 {
                    self.base_color.lerp(self.alt_color, (elev - 0.28) / 0.28)
                } else if elev < 0.78 {
                    self.alt_color.lerp(self.high_color, (elev - 0.56) / 0.22)
                } else {
                    self.high_color.scale(0.90 + elev * 0.15)
                };
                c.lerp(self.pole_color, t_abs.powf(2.2) * self.polar_strength)
            }

            1 => {
                // Smooth/atmospheric: swirling domain-warped noise over a pole-to-equator gradient
                let wx = (noise2(lon * 1.3 + no, lat * 1.9 + no) - 0.5) * 0.50;
                let wy = (noise2(lon * 1.3 + no2, lat * 1.9 + no2) - 0.5) * 0.35;
                let swirl = fbm(lon * 2.2 + wx * 0.7 + no, lat * 2.2 + wy * 0.7 + no, 3);
                let detail = noise2(lon * 8.0 + no2, lat * 8.0 + no2) * 0.12;

                // Equator-to-pole base gradient
                let pole_t = t_abs.powf(1.4);
                let base = self.base_color.lerp(self.alt_color, pole_t * 0.60);

                // Swirl pattern layered on top
                let swirl_t = (swirl * 0.75 + detail).clamp(0.0, 1.0);
                let c = base.lerp(self.high_color, swirl_t * 0.45);

                c.lerp(self.pole_color, pole_t * self.polar_strength)
            }

            2 => {
                // Terrestrial: FBM height map for ocean/coast/land/mountains
                let h = fbm(lon * ts + no, lat * ts + no, 4);
                let detail = noise2(lon * ts * 5.5 + no2, lat * ts * 5.5 + no2) * 0.12;
                let elev = (h + detail * 0.5).clamp(0.0, 1.0);

                let sea = 0.44;
                let c = if elev < sea - 0.06 {
                    // Deep ocean: darker base
                    self.base_color.scale(0.50 + elev * 1.0)
                } else if elev < sea + 0.04 {
                    // Coastal / shallow water transition
                    let t = (elev - (sea - 0.06)) / 0.10;
                    let shallow = self.base_color.lerp(self.alt_color, 0.35);
                    shallow.lerp(self.alt_color, t)
                } else if elev < 0.72 {
                    // Lowland to mid terrain
                    self.alt_color.lerp(self.high_color, (elev - sea) / (0.72 - sea) * 0.65)
                } else {
                    // Mountain peaks → snow
                    self.high_color.lerp(self.pole_color, (elev - 0.72) / 0.28)
                };

                c.lerp(self.pole_color, t_abs.powf(2.4) * self.polar_strength * 1.20)
            }

            _ => { // 3: volcanic
                // dark crust driven by FBM, lava in low areas, calderas at peaks
                let h = fbm(lon * ts + no, lat * ts + no, 4);
                let detail = noise2(lon * ts * 4.5 + no2, lat * ts * 4.5 + no2) * 0.14;
                let elev = (h + detail * 0.5).clamp(0.0, 1.0);

                // Dark crust with surface texture
                let crust = self.base_color.scale(0.22 + elev * 0.45 + detail * 0.3);

                // Magma wells up in the lowest terrain
                let magma = ((0.30 - elev).max(0.0) / 0.30).powf(1.4);
                let c = crust.lerp(self.alt_color, magma * 0.88);

                // Bright caldera spikes at highest noise peaks
                let caldera_n = noise2(lon * ts * 3.8 + no2, lat * ts * 3.8 + no2);
                let caldera = ((caldera_n - 0.77) * 4.3).clamp(0.0, 1.0);
                let c = c.lerp(self.high_color, caldera * 0.72);

                c.lerp(self.pole_color, t_abs.powf(3.0) * self.polar_strength * 0.70)
            }
        }
    }

    fn ring_config(&self) -> Option<RingConfig> {
        self.ring
    }

    fn moons(&self) -> &[Moon] {
        &self.stored_moons
    }
}
