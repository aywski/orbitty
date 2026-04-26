pub fn compute(nx: f64, ny: f64, nz: f64, sun: [f64; 3]) -> f64 {
    let dot = (nx * sun[0] + ny * sun[1] + nz * sun[2]).clamp(-1.0, 1.0);

    // Night side is dimmed but not black; smooth gradient across the terminator.
    let ambient = 0.28;
    let diffuse = 0.72;

    // Wide smooth transition: map dot in [-0.35, 0.35] to [0, 1], smoothstep.
    let edge = 0.35;
    let t = ((dot + edge) / (2.0 * edge)).clamp(0.0, 1.0);
    let smooth = t * t * (3.0 - 2.0 * t);

    ambient + diffuse * smooth
}
