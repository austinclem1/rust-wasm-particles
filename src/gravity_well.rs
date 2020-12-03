// Simple gravity well class, keeps track of position, rotation,
// and can determine if a click event lands within its borders

pub struct GravityWell {
    pub pos: [f64; 2],
    pub rotation_deg: f64,
    pub mass: f64,
    pub is_selected: bool,
}

impl GravityWell {
    pub const RADIUS: u32 = 20;
    pub const ROTATION_SPEED: f64 = 2.0;

    pub fn new(pos: [f64; 2], mass: f64) -> Self {
        GravityWell {
            pos,
            rotation_deg: 0.0,
            mass,
            is_selected: false,
        }
    }

    // Used for checking if the user's click is inside this gravity well
    // For clicking and dragging
    pub fn is_point_inside(&self, x: i32, y: i32) -> bool {
        let delta_x = (x as f64 - self.pos[0]).abs();
        let delta_y = (y as f64 - self.pos[1]).abs();
        let distance_from_well = glm::length(&glm::vec2(delta_x, delta_y));
        if distance_from_well <= GravityWell::RADIUS as f64 {
            true
        } else {
            false
        }
    }

    // Get canvas coordinates of each side of the gravity well
    pub fn get_rect(&self) -> (f64, f64, f64, f64) {
        let left = self.pos[0] - (GravityWell::RADIUS as f64);
        let top = self.pos[1] - (GravityWell::RADIUS as f64);
        let right = self.pos[0] + (GravityWell::RADIUS as f64);
        let bottom = self.pos[1] + (GravityWell::RADIUS as f64);
        (left, top, right, bottom)
    }

    pub fn move_by(&mut self, delta_x: f64, delta_y: f64) {
        self.pos[0] += delta_x;
        self.pos[1] += delta_y;
    }
}
