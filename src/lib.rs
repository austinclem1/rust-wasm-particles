mod utils;
use rand::Rng;
use std::ptr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{console, CanvasRenderingContext2d, ImageData};
extern crate libc;
use std::mem;
use vecmath;
use vecmath::Vector2;

#[wasm_bindgen]
pub fn initialize() {
    utils::set_panic_hook();
}

pub struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(name: &'a str) -> Timer<'a> {
        console::time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        console::time_end_with_label(self.name);
    }
}

#[wasm_bindgen]
pub struct RustCanvas {
    width: u32,
    height: u32,
    pixel_data: Vec<u8>,
    particles: Vec<Particle>,
}

#[wasm_bindgen]
impl RustCanvas {
    pub fn new(width: u32, height: u32) -> RustCanvas {
        let particles: Vec<Particle> = Vec::new();
        RustCanvas {
            width,
            height,
            pixel_data: vec![0x00; (width * height * 4) as usize],
            particles,
        }
    }

    pub fn initialize_particles(&mut self, num_particles: u32) {
        self.particles.reserve(num_particles as usize);
        let mut rng = rand::thread_rng();
        let min_vel = 20.0;
        let max_vel = 80.0;
        for _ in 0..num_particles {
            let pos_x = rng.gen::<f64>() * self.width as f64;
            let pos_y = rng.gen::<f64>() * self.height as f64;
            let vel_x = rng.gen::<f64>() * (max_vel - min_vel) + min_vel;
            let vel_y = rng.gen::<f64>() * (max_vel - min_vel) + min_vel;
            let color = Color {
                r: rng.gen::<u8>(),
                g: rng.gen::<u8>(),
                b: rng.gen::<u8>(),
                a: 0xff,
            };
            let p = Particle::new(pos_x, pos_y, vel_x, vel_y, 3, color);
            self.particles.push(p);
        }
    }

    pub fn update(&mut self, delta: f64) {
        let _timer = Timer::new("RustCanvas::update()");
        let gravity_pos_x = (self.width / 2) as f64;
        let gravity_pos_y = (self.height / 2) as f64;
        let g_pos: Vector2<f64> = [(self.width / 2) as f64, (self.height / 2) as f64];
        let gravity_mass = 50.0;
        for particle in &mut self.particles {
            let (px, py) = particle.pos;
            let p_pos: Vector2<f64> = [px, py];
            let dx = gravity_pos_x - px;
            let dy = gravity_pos_y - py;
            let p_to_g = vecmath::vec2_sub(g_pos, p_pos);
            // let distance_from_gravity = (dx.powi(2) + dy.powi(2)).sqrt();
            // let distance_from_gravity =
            //     vecmath::vec2_len(vecmath::vec2_sub([gravity_pos_x, gravity_pos_y], [px, py]));
            let distance_from_gravity = vecmath::vec2_len(p_to_g);

            let gravity_force = {
                if distance_from_gravity <= 0.0 {
                    gravity_mass
                } else {
                    // gravity_mass / distance_from_gravity.powi(2)
                    gravity_mass / distance_from_gravity
                }
            };
            let gravity_dir = vecmath::vec2_normalized(p_to_g);
            let p_acc = vecmath::vec2_scale(gravity_dir, gravity_force);
            particle.vel.0 += p_acc[0];
            particle.vel.1 += p_acc[1];
            particle.pos.0 += particle.vel.0 * delta;
            particle.pos.1 += particle.vel.1 * delta;
            if particle.pos.0 < 0.0 || particle.pos.0 >= self.width as f64 {
                particle.vel.0 *= -1.0;
                particle.pos.0 = particle.pos.0.max(0.0);
                particle.pos.0 = particle.pos.0.min((self.width - 1) as f64);
            }
            if particle.pos.1 < 0.0 || particle.pos.1 >= self.height as f64 {
                particle.vel.1 *= -1.0;
                particle.pos.1 = particle.pos.1.max(0.0);
                particle.pos.1 = particle.pos.1.min((self.height - 1) as f64);
            }
        }
    }

    pub fn render(&mut self, ctx: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        let _timer = Timer::new("RustCanvas::render");
        {
            let _timer = Timer::new("draw background");
            // self.draw_rect(0, 0, self.width, self.height, Color::from_u32(0x000000ff));
            unsafe {
                ptr::write_bytes(
                    self.pixel_data.as_mut_ptr(),
                    0x00,
                    self.pixel_data.len() as usize,
                );
            }
            for pixel_idx in (0..self.pixel_data.len()).step_by(4) {
                self.pixel_data[pixel_idx + 3] = 0xff;
            }
        }
        {
            let _timer = Timer::new("draw particles");
            for i in 0..self.particles.len() {
                let p = self.particles[i];
                let (px, py) = p.pos;
                self.draw_rect(px as i32, py as i32, p.size, p.size, p.color);
            }
        }

        let _timer = Timer::new("ctx.put_image_data");
        let pixel_image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&mut self.pixel_data),
            self.width,
            self.height,
        )?;

        ctx.put_image_data(&pixel_image_data, 0.0, 0.0)
    }

    pub fn spawn_particle(&mut self, x: f64, y: f64, vel_x: f64, vel_y: f64) {
        let _timer = Timer::new("RustCanvas::spawn_particle");
        let mut rng = rand::thread_rng();
        let color = Color {
            r: rng.gen::<u8>(),
            g: rng.gen::<u8>(),
            b: rng.gen::<u8>(),
            a: 0xff,
        };
        self.particles
            .push(Particle::new(x, y, vel_x, vel_y, 3, color));
    }
}

impl RustCanvas {
    fn get_pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            Some(((y * self.width as i32 + x) * 4) as usize)
        } else {
            None
        }
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if let Some(idx) = self.get_pixel_index(x, y) {
            self.pixel_data[idx] = color.r;
            self.pixel_data[idx + 1] = color.g;
            self.pixel_data[idx + 2] = color.b;
            self.pixel_data[idx + 3] = color.a;
        }
    }

    fn draw_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color) {
        for pixel_y in y..y + height as i32 {
            for pixel_x in x..x + width as i32 {
                self.set_pixel(pixel_x, pixel_y, color);
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct Particle {
    pos: (f64, f64),
    vel: (f64, f64),
    size: u32,
    color: Color,
}

impl Particle {
    pub fn new(
        pos_x: f64,
        pos_y: f64,
        vel_x: f64,
        vel_y: f64,
        size: u32,
        color: Color,
    ) -> Particle {
        Particle {
            pos: (pos_x, pos_y),
            vel: (vel_x, vel_y),
            size,
            color,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    fn from_u32(num: u32) -> Color {
        let r = (num >> 24) as u8;
        let g = (num >> 16) as u8;
        let b = (num >> 8) as u8;
        let a = (num >> 0) as u8;

        Color { r, g, b, a }
    }
}
