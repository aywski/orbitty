use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, noise2};

pub struct Saturn;

impl Planet for Saturn {
    fn has_rings(&self) -> bool { true }

    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        let t_abs = (lat / (PI / 2.0)).abs().clamp(0.0, 1.0);

        // Realistic pale-gold Saturn palette.
        let equator = Rgb(228, 208, 158);
        let mid = Rgb(210, 185, 135);
        let high_lat = Rgb(192, 165, 118);
        let polar = Rgb(175, 148, 102);

        let base = if t_abs < 0.35 {
            equator.lerp(mid, t_abs / 0.35)
        } else if t_abs < 0.70 {
            mid.lerp(high_lat, (t_abs - 0.35) / 0.35)
        } else {
            high_lat.lerp(polar, (t_abs - 0.70) / 0.30)
        };

        // Horizontal banding - 6 distinct belts like the real planet.
        let band_t = (lat * 11.0).sin() * 0.5 + 0.5;
        let fine_t = (lat * 26.0).sin() * 0.18;
        let noise_t = noise2(lon * 1.5, lat * 10.0) * 0.12;
        let band = (band_t + fine_t + noise_t).clamp(0.0, 1.0);

        let belt_dark = Rgb(178, 148, 98);
        let belt_bright = Rgb(235, 218, 172);
        let banded = belt_dark.lerp(belt_bright, band);
        let mut c = base.lerp(banded, 0.60);

        // Equatorial bright zone.
        let eq_zone = (1.0 - (lat * 5.0).powi(2)).clamp(0.0, 1.0);
        c = c.lerp(Rgb(240, 225, 180), eq_zone * 0.15);

        // North polar hexagon (75 deg N ~ lat 1.31 rad).
        let hex_lat = lat - 1.31;
        if hex_lat.abs() < 0.18 && lat > 0.0 {
            // Six-fold symmetry approximated with cos(6*lon).
            let hex_r = hex_lat.abs() / 0.18;
            let hex_boundary = 0.5 + 0.5 * (lon * 6.0).cos();
            let inside_hex = (1.0 - hex_r) * hex_boundary;
            let hex_col = Rgb(148, 118, 75);
            c = c.lerp(hex_col, inside_hex.clamp(0.0, 1.0) * 0.55);
        }

        // Polar vortex core.
        if lat.abs() > 1.45 {
            let vortex = ((lat.abs() - 1.45) / 0.12).clamp(0.0, 1.0).powf(2.0);
            c = c.lerp(Rgb(118, 88, 52), vortex * 0.70);
        }

        // Occasional oval storm at mid-latitude (like Dragon Storm).
        let storm_lat = lat - 0.62;
        let storm_lon = {
            let mut l = lon % (2.0 * PI);
            if l > PI { l -= 2.0 * PI; }
            l - 1.8
        };
        let sd2 = (storm_lat / 0.08).powi(2) + (storm_lon / 0.14).powi(2);
        if sd2 < 1.0 {
            let blend = (1.0 - sd2).sqrt();
            c = c.lerp(Rgb(195, 158, 100), blend * 0.45);
        }

        c
    }
}
