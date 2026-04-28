mod earth;
mod jupiter;
mod mars;
mod mercury;
mod neptune;
mod saturn;
mod uranus;
mod venus;
pub mod random;

use crate::palette::Rgb;

#[derive(Clone, Copy)]
pub struct RingConfig {
    pub inner: f64,
    pub outer: f64,
    pub colors: [Rgb; 5],
}

#[derive(Clone)]
pub struct Moon {
    pub color: Rgb,
    pub radius: f64,         // display radius in planet-radius units
    pub orbital_radius: f64, // display orbit distance in planet-radius units
    pub inclination: f64,    // orbital plane tilt in radians (0 = equatorial plane)
    pub speed: f64,          // angular speed relative to spin_accum (negative = retrograde)
    pub phase: f64,          // initial orbital phase in radians
}

pub trait Planet: Send + Sync {
    // Returns base surface color at (lat, lon) in radians.
    // lat in [-PI/2, PI/2], lon in [-PI, PI] (may be outside, wrap as needed)
    fn surface_color(&self, lat: f64, lon: f64) -> Rgb;

    // Optional: render extra geometry (rings etc) around the sphere
    fn ring_config(&self) -> Option<RingConfig> {
        None
    }

    fn moons(&self) -> Vec<Moon> {
        vec![]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlanetId {
    Mercury,
    Venus,
    Earth,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

impl PlanetId {
    pub fn from_digit(d: u8) -> PlanetId {
        match d {
            1 => PlanetId::Mercury,
            2 => PlanetId::Venus,
            3 => PlanetId::Earth,
            4 => PlanetId::Mars,
            5 => PlanetId::Jupiter,
            6 => PlanetId::Saturn,
            7 => PlanetId::Uranus,
            _ => PlanetId::Neptune,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PlanetId::Mercury => "Mercury",
            PlanetId::Venus => "Venus",
            PlanetId::Earth => "Earth",
            PlanetId::Mars => "Mars",
            PlanetId::Jupiter => "Jupiter",
            PlanetId::Saturn => "Saturn",
            PlanetId::Uranus => "Uranus",
            PlanetId::Neptune => "Neptune",
        }
    }
}

pub fn make_random(seed: u64) -> (Box<dyn Planet>, String) {
    let rp = random::make(seed);
    let name = rp.name.clone();
    (Box::new(rp), name)
}

pub fn get(id: PlanetId) -> Box<dyn Planet> {
    match id {
        PlanetId::Mercury => Box::new(mercury::Mercury),
        PlanetId::Venus => Box::new(venus::Venus),
        PlanetId::Earth => Box::new(earth::Earth),
        PlanetId::Mars => Box::new(mars::Mars),
        PlanetId::Jupiter => Box::new(jupiter::Jupiter),
        PlanetId::Saturn => Box::new(saturn::Saturn),
        PlanetId::Uranus => Box::new(uranus::Uranus),
        PlanetId::Neptune => Box::new(neptune::Neptune),
    }
}

pub fn wrap_lon(lon: f64) -> f64 {
    use std::f64::consts::PI;
    (lon + PI).rem_euclid(2.0 * PI) - PI
}

// Simple value noise for procedural surfaces
pub fn noise2(x: f64, y: f64) -> f64 {
    let ix = x.floor() as i64;
    let iy = y.floor() as i64;
    let fx = x - ix as f64;
    let fy = y - iy as f64;
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    let a = hash2(ix, iy);
    let b = hash2(ix + 1, iy);
    let c = hash2(ix, iy + 1);
    let d = hash2(ix + 1, iy + 1);

    let ab = a + ux * (b - a);
    let cd = c + ux * (d - c);
    ab + uy * (cd - ab)
}

fn hash2(x: i64, y: i64) -> f64 {
    let mut h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h = h ^ (h >> 16);
    (h as u64 as f64) / (u64::MAX as f64)
}
