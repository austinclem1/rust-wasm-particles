// Simple particle struct to keep track of individual position, velocity, and color

use crate::color::Color;

pub struct Particle {
    pub pos: [f64; 2],
    pub vel: [f64; 2],
    pub color: Color,
}

impl Particle {
    const MAX_VELOCITY: f64 = 2000.0;
    pub fn new(pos_x: f64, pos_y: f64, vel_x: f64, vel_y: f64, color: Color) -> Particle {
        Particle {
            pos: [pos_x, pos_y],
            vel: [vel_x, vel_y],
            color,
        }
    }
}

