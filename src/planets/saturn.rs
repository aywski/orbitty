use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, Moon, noise2};

pub struct Saturn;

impl Planet for Saturn {
    fn ring_config(&self) -> Option<super::RingConfig> {
        Some(super::RingConfig {
            inner: 1.25,
            outer: 2.10,
            colors: [
                Rgb(215, 190, 145), Rgb(175, 145, 100), Rgb(210, 185, 140),
                Rgb(165, 135, 95), Rgb(205, 178, 132),
            ],
        })
    }

    fn moons(&self) -> Vec<Moon> {
        // All inner moons start beyond the rings (outer ring at 2.10 radii)
        vec![
            Moon { color: Rgb(210, 207, 202), radius: 0.038, orbital_radius: 2.35, inclination: 0.03, speed: 1.40, phase: 0.5 },
            Moon { color: Rgb(242, 242, 240), radius: 0.040, orbital_radius: 2.65, inclination: 0.02, speed: 0.95, phase: 2.5 },
            Moon { color: Rgb(222, 220, 215), radius: 0.042, orbital_radius: 2.90, inclination: 0.03, speed: 0.68, phase: 1.0 },
            Moon { color: Rgb(205, 202, 196), radius: 0.044, orbital_radius: 3.20, inclination: 0.04, speed: 0.48, phase: 4.0 },
            Moon { color: Rgb(200, 197, 191), radius: 0.048, orbital_radius: 3.60, inclination: 0.04, speed: 0.30, phase: 0.8 },
            Moon { color: Rgb(208, 158, 85), radius: 0.072, orbital_radius: 4.60, inclination: 0.07, speed: 0.11, phase: 2.2 },
            Moon { color: Rgb(158, 152, 142), radius: 0.048, orbital_radius: 5.90, inclination: 0.26, speed: 0.035, phase: 3.7 },
        ]
    }

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
