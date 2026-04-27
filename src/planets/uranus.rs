use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, Moon, noise2};

pub struct Uranus;

impl Planet for Uranus {
    fn moons(&self) -> Vec<Moon> {
        vec![
            Moon { color: Rgb(150, 146, 141), radius: 0.040, orbital_radius: 2.20, inclination: 0.06, speed: 0.90, phase: 1.0 },
            Moon { color: Rgb(186, 182, 176), radius: 0.045, orbital_radius: 2.80, inclination: 0.02, speed: 0.52, phase: 4.0 },
            Moon { color: Rgb(80, 78, 74), radius: 0.044, orbital_radius: 3.45, inclination: 0.01, speed: 0.32, phase: 2.0 },
            Moon { color: Rgb(170, 166, 160), radius: 0.050, orbital_radius: 4.35, inclination: 0.01, speed: 0.18, phase: 0.5 },
            Moon { color: Rgb(115, 112, 108), radius: 0.050, orbital_radius: 5.25, inclination: 0.01, speed: 0.12, phase: 3.0 },
        ]
    }

    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        let t_pole = (lat / (PI / 2.0)).abs().clamp(0.0, 1.0);

        // Base: characteristic pale cyan-green (methane absorbs red).
        // Poles significantly brighter/whiter due to haze.
        let equator = Rgb(82, 195, 208);
        let pole = Rgb(188, 242, 232);
        let base = equator.lerp(pole, t_pole.powf(1.2));

        // Domain-warp to get slightly wavy bands.
        let warp = noise2(lon * 1.8, lat * 2.2) * 0.5 - 0.25;
        let lon_w = lon + warp;

        // Primary banding - made more visible.
        let b1 = (lat * 7.5).sin();
        let b2 = (lat * 17.0).sin() * 0.45;
        let b3 = (lat * 32.0).sin() * 0.20;
        let band = b1 + b2 + b3;

        // Texture noise warped along bands.
        let n1 = noise2(lon_w * 4.0, lat * 6.5) * 0.50;
        let n2 = noise2(lon_w * 9.5, lat * 13.0) * 0.32;
        let n3 = noise2(lon_w * 21.0, lat * 24.0) * 0.18;
        let texture = n1 + n2 + n3; // [0, 1]

        // Combine: band drives hue, texture drives local contrast.
        let tone = band * 0.52 + (texture - 0.5) * 0.38;

        let dark = Rgb(60, 168, 185);
        let light = Rgb(158, 238, 228);
        let mut c = if tone >= 0.0 {
            base.lerp(light, tone.clamp(0.0, 1.0))
        } else {
            base.lerp(dark, (-tone).clamp(0.0, 1.0))
        };

        // Dark polar collar at ~55-70 deg lat where haze thins.
        let collar_inner = (t_pole - 0.58) / 0.08;
        let collar_outer = (0.75 - t_pole) / 0.08;
        let collar = collar_inner.clamp(0.0, 1.0).min(collar_outer.clamp(0.0, 1.0));
        c = c.lerp(Rgb(52, 155, 172), collar * 0.50);

        // Polar vortex - bright asymmetric spot near each pole.
        if t_pole > 0.82 {
            let vortex = (t_pole - 0.82) / 0.18;
            c = c.lerp(Rgb(215, 248, 240), vortex.powf(1.5) * 0.55);
        }

        // Cloud streaks - more frequent than before (every ~10 deg lat).
        let streak1 = noise2(lon_w * 7.5 + lat * 4.0 + 2.3, lat * 9.5);
        let streak2 = noise2(lon_w * 6.0 - lat * 3.5 + 8.1, lat * 11.0);
        if streak1 > 0.79 {
            c = c.lerp(Rgb(228, 250, 244), (streak1 - 0.79) / 0.21 * 0.68);
        }
        if streak2 > 0.82 {
            c = c.lerp(Rgb(210, 245, 238), (streak2 - 0.82) / 0.18 * 0.50);
        }

        c
    }
}
