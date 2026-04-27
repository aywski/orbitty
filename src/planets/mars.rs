use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, Moon, noise2, wrap_lon};

pub struct Mars;

impl Planet for Mars {
    fn moons(&self) -> Vec<Moon> {
        vec![
            Moon { color: Rgb(105, 96, 85), radius: 0.04, orbital_radius: 2.2, inclination: 0.02, speed: 1.2, phase: 0.0 },
            Moon { color: Rgb(122, 112, 100), radius: 0.038, orbital_radius: 3.2, inclination: 0.03, speed: 0.4, phase: 2.0 },
        ]
    }

    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        surface(lat, lon)
    }
}

fn surface(lat: f64, lon: f64) -> Rgb {
    let lon_w = wrap_lon(lon);
    let n1 = noise2(lon * 2.5, lat * 2.5);
    let n2 = noise2(lon * 6.5, lat * 6.5);
    let n3 = noise2(lon * 16.0, lat * 16.0);
    let n = (n1 * 0.55 + n2 * 0.30 + n3 * 0.15).clamp(0.0, 1.0);

    // Dichotomy: northern hemisphere smoother / brighter basalt,
    // southern hemisphere darker, heavily cratered highlands.
    let hemisphere = if lat > 0.0 { 1.0 } else { 0.0 };

    let rust = Rgb(190, 80, 38);
    let dark_basalt = Rgb(115, 52, 30);
    let tan = Rgb(220, 140, 85);
    let shadow = Rgb(90, 40, 25);

    let mut c = if n > 0.68 {
        rust.lerp(tan, (n - 0.68) / 0.32)
    } else if n > 0.30 {
        dark_basalt.lerp(rust, (n - 0.30) / 0.38)
    } else {
        shadow.lerp(dark_basalt, n / 0.30)
    };

    // Southern highlands look darker overall.
    if hemisphere < 0.5 {
        c = c.scale(0.88);
    }

    // Crater pocks in south: dark spots from very high-freq noise.
    let crater = noise2(lon * 28.0, lat * 28.0);
    if hemisphere < 0.5 && crater < 0.22 {
        let t = (0.22 - crater) / 0.22;
        c = c.lerp(shadow, t * 0.55);
    }

    // Hellas Basin (southern, lat ~-40°, lon ~70°) - big dark depression.
    let hellas_dlat = (lat + 0.75) / 0.25;
    let hellas_dlon = angular_dist(lon_w, 1.22) / 0.30;
    let hellas_d2 = hellas_dlat * hellas_dlat + hellas_dlon * hellas_dlon;
    if hellas_d2 < 1.0 {
        c = c.lerp(Rgb(100, 55, 32), (1.0 - hellas_d2).sqrt() * 0.6);
    }

    // Syrtis Major (lat ~8°, lon ~70°) - classic dark triangle.
    let sm_dlat = (lat - 0.15) / 0.28;
    let sm_dlon = angular_dist(lon_w, 1.22) / 0.25;
    let sm_d2 = sm_dlat * sm_dlat + sm_dlon * sm_dlon;
    if sm_d2 < 1.0 {
        c = c.lerp(Rgb(80, 42, 25), (1.0 - sm_d2).sqrt() * 0.75);
    }

    // Valles Marineris - equatorial canyon east of Tharsis, lon ~-70°.
    let vm_dlat = (lat + 0.12) / 0.06;
    let vm_dlon = angular_dist(lon_w, -1.22) / 0.9;
    let vm_d2 = vm_dlat * vm_dlat + vm_dlon * vm_dlon;
    if vm_d2 < 1.0 {
        c = c.lerp(Rgb(60, 30, 18), (1.0 - vm_d2).sqrt() * 0.7);
    }

    // Tharsis + Olympus Mons - raised brighter region around lon ~-110°.
    let th_dlat = (lat - 0.15) / 0.30;
    let th_dlon = angular_dist(lon_w, -1.92) / 0.45;
    let th_d2 = th_dlat * th_dlat + th_dlon * th_dlon;
    if th_d2 < 1.0 {
        c = c.lerp(Rgb(210, 130, 80), (1.0 - th_d2).sqrt() * 0.45);
    }

    c
}

fn angular_dist(a: f64, b: f64) -> f64 {
    let mut d = (a - b).abs();
    if d > PI { d = 2.0 * PI - d; }
    d
}
