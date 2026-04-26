use crate::palette::Rgb;
use super::{Planet, noise2};

pub struct Venus;

impl Planet for Venus {
    fn surface_color(&self, lat: f64, lon: f64) -> Rgb {
        // Venus cloud deck: strong horizontal banding with domain-warped turbulence.
        // Latitude-dependent shear makes bands stretch east-west.
        let shear = (lat * 2.5).cos() * 5.0 + 1.0;
        let warp = noise2(lon * 1.5 + 20.0, lat * 2.0) * 1.8
                 - noise2(lon * 3.0 + 7.0, lat * 3.0) * 0.9;
        let lon_w = lon * shear * 0.12 + warp;
        let lat_w = lat * 5.0 + noise2(lon * 2.0, lat * 2.5) * 0.6;

        let n1 = noise2(lon_w, lat_w) * 0.50;
        let n2 = noise2(lon_w * 2.8 + 3.0, lat_w * 2.8) * 0.30;
        let n3 = noise2(lon_w * 7.5 + 1.0, lat_w * 7.5) * 0.20;
        let n = (n1 + n2 + n3).clamp(0.0, 1.0);

        // Prominent horizontal bands via latitude.
        let band = ((lat * 8.0).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        let detail = n * 0.6 + band * 0.4;

        let dark_band = Rgb(120, 82, 28);
        let mid_band = Rgb(195, 162, 95);
        let bright = Rgb(248, 228, 168);
        let haze = Rgb(235, 210, 148);

        let c = if detail > 0.72 {
            mid_band.lerp(bright, (detail - 0.72) / 0.28)
        } else if detail > 0.42 {
            dark_band.lerp(mid_band, (detail - 0.42) / 0.30)
        } else {
            Rgb(90, 58, 18).lerp(dark_band, detail / 0.42)
        };

        // Polar hood: bright whitish haze.
        let abs_lat = lat.abs();
        let polar = ((abs_lat - 1.1) / 0.45).clamp(0.0, 1.0).powf(1.5);
        let c = c.lerp(haze, polar * 0.55);

        // Y-shaped feature hint near equator (characteristic of Venus UV).
        let y_lat = (lat * 4.0).cos();
        let y_lon = (lon * 2.0).sin().abs();
        let y_feat = (y_lat * y_lon - 0.4).max(0.0) * 0.5;
        c.lerp(Rgb(105, 72, 22), y_feat * noise2(lon * 1.8, lat * 3.0))
    }
}
