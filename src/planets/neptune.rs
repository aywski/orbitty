use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, Moon, noise2};

pub struct Neptune;

fn wdlon(lon: f64, target: f64) -> f64 {
    let mut d = (lon - target) % (2.0 * PI);
    if d > PI { d -= 2.0 * PI; }
    if d < -PI { d += 2.0 * PI; }
    d.abs()
}

impl Planet for Neptune {
    fn moons(&self) -> Vec<Moon> {
        vec![
            // Triton: retrograde (speed < 0) and high inclination (~157 deg = 2.74 rad)
            Moon { color: Rgb(195, 182, 172), radius: 0.062, orbital_radius: 3.10, inclination: 2.74, speed: -0.15, phase: 1.5 },
        ]
    }

    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        let t_pole = (lat / (PI / 2.0)).abs().clamp(0.0, 1.0);

        // Neptune: vivid deep cobalt-blue, much more saturated than Uranus.
        let deep = Rgb(14, 32, 148);
        let mid = Rgb(32, 82, 198);
        let equator_bright = Rgb(52, 118, 225);

        // Strong banding - Neptune has the fastest winds in the solar system.
        let b1 = (lat * 9.0).sin();
        let b2 = (lat * 21.0).sin() * 0.45;
        let b3 = (lat * 38.0).sin() * 0.20;
        let band = ((b1 + b2 + b3) * 0.5 + 0.5).clamp(0.0, 1.0);

        // Polar brightening: poles are noticeably brighter in Voyager/HST images.
        let pole_glow = t_pole.powf(2.5);
        let base = deep
            .lerp(equator_bright, band * 0.55)
            .lerp(mid, t_pole * 0.35)
            .lerp(Rgb(78, 148, 232), pole_glow * 0.50);

        // Domain-warped turbulence to break up flat areas.
        let wx = noise2(lon * 2.5 + 7.1, lat * 3.5) * 1.8 - 0.9;
        let wy = noise2(lon * 2.5 + 14.3, lat * 3.5 + 5.2) * 1.8 - 0.9;
        let swirl = noise2(lon * 5.0 + wx, lat * 6.5 + wy);
        let fine = noise2(lon * 13.0, lat * 15.0) * 0.15;

        let mut c = if swirl > 0.62 {
            base.lerp(Rgb(62, 130, 228), (swirl - 0.62) / 0.38 * 0.65)
        } else if swirl < 0.30 {
            base.lerp(Rgb(8, 20, 88), (0.30 - swirl) / 0.30 * 0.40)
        } else {
            base
        };
        c = c.scale(0.88 + fine);

        // Great Dark Spot - large anticyclone ~22 deg S.
        let dlat = (lat + 0.38) / 0.18;
        let dlon = wdlon(lon, -1.05) / 0.32;
        let d2 = dlat * dlat + dlon * dlon;
        if d2 < 1.0 {
            let blend = (1.0 - d2).sqrt();
            c = c.lerp(Rgb(8, 18, 68), blend * 0.52);
            let ring = ((d2 - 0.38).abs() / 0.15).clamp(0.0, 1.0);
            c = c.lerp(Rgb(148, 195, 255), (1.0 - ring) * blend * 0.28);
        }

        // Small Dark Spot 2 (DS2).
        let d2lat = (lat + 0.96) / 0.09;
        let d2lon = wdlon(lon, -2.4) / 0.12;
        let ds2 = d2lat * d2lat + d2lon * d2lon;
        if ds2 < 1.0 {
            c = c.lerp(Rgb(10, 22, 80), (1.0 - ds2).sqrt() * 0.42);
        }

        // Scooter - bright cloud near equator.
        let slat = (lat - 0.10) / 0.065;
        let slon = wdlon(lon, 0.52) / 0.10;
        let sd2 = slat * slat + slon * slon;
        if sd2 < 1.0 {
            c = c.lerp(Rgb(200, 225, 255), (1.0 - sd2).sqrt() * 0.48);
        }

        c
    }
}
