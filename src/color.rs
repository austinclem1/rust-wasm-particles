// Simple color struct, created from an unsigned 32 representing RRGGBBAA
#[derive(Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn from_u32(num: u32) -> Color {
        let r = (num >> 24) as u8;
        let g = (num >> 16) as u8;
        let b = (num >> 8) as u8;
        let a = (num >> 0) as u8;

        Color { r, g, b, a }
    }

    pub fn tint(&mut self, tint: Color) {
        let new_r = ((self.r as f64 + tint.r as f64) / 2.0) as u8;
        let new_g = ((self.g as f64 + tint.g as f64) / 2.0) as u8;
        let new_b = ((self.b as f64 + tint.b as f64) / 2.0) as u8;
        let new_a = ((self.a as f64 + tint.a as f64) / 2.0) as u8;

        self.r = new_r;
        self.g = new_g;
        self.b = new_b;
        self.a = new_a;
    }
}

