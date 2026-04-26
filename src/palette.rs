#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn lerp(self, other: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb(
            (self.0 as f64 + (other.0 as f64 - self.0 as f64) * t) as u8,
            (self.1 as f64 + (other.1 as f64 - self.1 as f64) * t) as u8,
            (self.2 as f64 + (other.2 as f64 - self.2 as f64) * t) as u8,
        )
    }

    pub fn scale(self, factor: f64) -> Rgb {
        Rgb(
            (self.0 as f64 * factor).clamp(0.0, 255.0) as u8,
            (self.1 as f64 * factor).clamp(0.0, 255.0) as u8,
            (self.2 as f64 * factor).clamp(0.0, 255.0) as u8,
        )
    }

    pub fn fg_seq(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.0, self.1, self.2)
    }

    pub fn bg_seq(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.0, self.1, self.2)
    }
}
