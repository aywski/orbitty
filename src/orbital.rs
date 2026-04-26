use std::f64::consts::PI;

// One full rotation every N seconds at speed level 1.0, same for all planets.
const BASE_SECONDS_PER_ROTATION: f64 = 30.0;

// Same rate and direction for all planets - direction is controlled by the speed level.
pub fn spin_rate() -> f64 {
    2.0 * PI / BASE_SECONDS_PER_ROTATION
}
