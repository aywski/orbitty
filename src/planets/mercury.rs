use std::f64::consts::PI;
use crate::palette::Rgb;
use super::{Planet, noise2, wrap_lon};

pub struct Mercury;

impl Planet for Mercury {
    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        let lon_w = wrap_lon(lon);
        let n1 = noise2(lon * 2.0, lat * 2.0) * 0.45;
        let n2 = noise2(lon * 6.0, lat * 6.0) * 0.30;
        let n3 = noise2(lon * 16.0, lat * 16.0) * 0.15;
        let n4 = noise2(lon * 40.0, lat * 40.0) * 0.10;
        let n = (n1 + n2 + n3 + n4).clamp(0.0, 1.0);

        let dark = Rgb(62, 58, 54);
        let mid = Rgb(138, 128, 118);
        let bright = Rgb(205, 195, 180);

        let mut c = if n > 0.62 {
            mid.lerp(bright, (n - 0.62) / 0.38)
        } else {
            dark.lerp(mid, n / 0.62)
        };

        // Dense crater field: cellular dark pits.
        let crater_a = noise2(lon * 25.0 + 3.0, lat * 25.0 + 1.0);
        if crater_a < 0.16 {
            let t = (0.16 - crater_a) / 0.16;
            c = c.lerp(Rgb(40, 36, 32), t * 0.75);
        }
        let crater_b = noise2(lon * 50.0 - 9.0, lat * 50.0 + 4.0);
        if crater_b < 0.10 {
            let t = (0.10 - crater_b) / 0.10;
            c = c.lerp(Rgb(220, 208, 190), t * 0.35);
        }

        // Caloris Basin - huge bright ringed impact (lat 30°, lon 163°).
        let cal_dlat = (lat - 0.52) / 0.38;
        let cal_dlon = angular_dist(lon_w, 2.85) / 0.45;
        let cal_d = (cal_dlat * cal_dlat + cal_dlon * cal_dlon).sqrt();
        if cal_d < 1.0 {
            let inside = (1.0 - cal_d).powf(0.6);
            c = c.lerp(Rgb(215, 205, 185), inside * 0.55);
        }
        if cal_d > 0.95 && cal_d < 1.10 {
            c = c.lerp(Rgb(90, 82, 74), (1.0 - (cal_d - 1.02).abs() / 0.08).clamp(0.0, 1.0) * 0.5);
        }

        c
    }
}

fn angular_dist(a: f64, b: f64) -> f64 {
    let mut d = (a - b).abs();
    if d > PI { d = 2.0 * PI - d; }
    d
}
