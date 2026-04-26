use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, noise2};

pub struct Jupiter;

impl Planet for Jupiter {
    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        let bands = [
            (0.00, Rgb(205, 175, 125)),
            (0.12, Rgb(155, 110, 80)),
            (0.22, Rgb(220, 190, 140)),
            (0.32, Rgb(145, 105, 75)),
            (0.42, Rgb(215, 185, 135)),
            (0.52, Rgb(170, 130, 95)),
            (0.60, Rgb(225, 195, 145)),
            (0.70, Rgb(150, 110, 80)),
            (0.82, Rgb(210, 180, 130)),
            (1.00, Rgb(190, 155, 110)),
        ];

        let t = (lat / (PI / 2.0)).abs();

        // Turbulent band edges - latitudinal ripples.
        let wave = noise2(lon * 1.8, lat * 10.0) * 0.05
                 + noise2(lon * 5.0, lat * 20.0) * 0.02;
        let t_noisy = (t + wave).clamp(0.0, 1.0);

        let mut c = band_color(t_noisy, &bands);

        // Zonal swirls within bands.
        let swirl = noise2(lon * 4.0 + lat * 8.0, lat * 14.0);
        if swirl > 0.66 {
            let hot = Rgb(220, 155, 100);
            c = c.lerp(hot, (swirl - 0.66) / 0.34 * 0.3);
        }

        // Great Red Spot: lat ~-0.39, lon ~PI.
        let dlat = (lat - (-0.39_f64)) / 0.11;
        let dlon = wrap_dlon(lon, PI) / 0.22;
        let d2 = dlat * dlat + dlon * dlon;
        if d2 < 1.0 {
            let spot_core = Rgb(195, 75, 45);
            let spot_edge = Rgb(225, 140, 90);
            let t = (1.0 - d2).sqrt();
            return c.lerp(spot_edge.lerp(spot_core, t), t * 0.60);
        }

        // Secondary oval (Oval BA).
        let dlat = (lat - (-0.55_f64)) / 0.08;
        let dlon = wrap_dlon(lon, -1.2) / 0.13;
        let d2 = dlat * dlat + dlon * dlon;
        if d2 < 1.0 {
            c = c.lerp(Rgb(230, 200, 160), (1.0 - d2).sqrt() * 0.35);
        }

        c
    }
}

// Signed angular distance in [-PI, PI], works for any accumulated lon.
fn wrap_dlon(lon: f64, target: f64) -> f64 {
    let mut d = (lon - target) % (2.0 * std::f64::consts::PI);
    if d > std::f64::consts::PI { d -= 2.0 * std::f64::consts::PI; }
    if d < -std::f64::consts::PI { d += 2.0 * std::f64::consts::PI; }
    d.abs()
}

fn band_color(t: f64, bands: &[(f64, Rgb)]) -> Rgb {
    for i in 0..bands.len() - 1 {
        let (t0, c0) = bands[i];
        let (t1, c1) = bands[i + 1];
        if t <= t1 {
            let f = (t - t0) / (t1 - t0);
            return c0.lerp(c1, f);
        }
    }
    bands.last().unwrap().1
}
