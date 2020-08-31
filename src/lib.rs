mod utils;
use rand::Rng;
use std::ptr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::{console, CanvasRenderingContext2d, ImageData};
extern crate libc;
use std::collections::VecDeque;
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

pub struct PixelBuffer {
    data: Vec<u32>,
}

#[wasm_bindgen]
pub struct RustCanvas {
    width: u32,
    height: u32,
    pixel_data: Vec<u32>,
    particles: Vec<TrailParticle>,
    rng: rand::rngs::ThreadRng,
}

#[wasm_bindgen]
impl RustCanvas {
    pub fn new(width: u32, height: u32) -> RustCanvas {
        let particles: Vec<TrailParticle> = Vec::new();
        let rng = rand::thread_rng();
        RustCanvas {
            width,
            height,
            pixel_data: vec![0x00; (width * height) as usize],
            particles,
            rng,
        }
    }

    pub fn initialize_particles(&mut self, num_particles: u32) {
        self.particles.reserve(num_particles as usize);
        let min_vel = 20.0;
        let max_vel = 80.0;
        for _ in 0..num_particles {
            let pos_x = self.rng.gen::<f64>() * self.width as f64;
            let pos_y = self.rng.gen::<f64>() * self.height as f64;
            let vel_x = self.rng.gen::<f64>() * (max_vel - min_vel) + min_vel;
            let vel_y = self.rng.gen::<f64>() * (max_vel - min_vel) + min_vel;
            self.spawn_particle(pos_x, pos_y, vel_x, vel_y);
        }
    }

    pub fn update(&mut self, delta: f64) {
        let _timer = Timer::new("RustCanvas::update()");
        let g_pos: Vector2<f64> = [(self.width / 2) as f64, (self.height / 2) as f64];
        let gravity_mass = 50.0;
        for particle in &mut self.particles {
            if particle.prev_positions.len() >= 5 {
                particle.prev_positions.pop_front();
            }
            particle.prev_positions.push_back(particle.pos);
            let (px, py) = particle.pos;
            let p_pos: Vector2<f64> = [px, py];
            let p_to_g = vecmath::vec2_sub(g_pos, p_pos);
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

    pub fn render(&mut self) {
        let _timer = Timer::new("RustCanvas::render");
        {
            let _timer = Timer::new("draw background");
            self.draw_rect(0, 0, self.width, self.height, Color::from_u32(0x000000ff));
        }
        {
            let _timer = Timer::new("draw particles");
            for i in 0..self.particles.len() {
                let p = &self.particles[i];
                let rect_size = p.size;
                let rect_color = p.color;
                let mut pos_vec = p.prev_positions.clone();
                pos_vec.push_back(p.pos);
                for pos in pos_vec {
                    self.draw_rect(pos.0 as i32, pos.1 as i32, rect_size, rect_size, rect_color);
                }
            }
        }
    }

    pub fn spawn_particle(&mut self, x: f64, y: f64, vel_x: f64, vel_y: f64) {
        let _timer = Timer::new("RustCanvas::spawn_particle");
        let color = Color {
            r: self.rng.gen::<u8>(),
            g: self.rng.gen::<u8>(),
            b: self.rng.gen::<u8>(),
            a: 0xff,
        };
        self.particles
            .push(TrailParticle::new(x, y, vel_x, vel_y, 2, color));
    }

    pub fn get_pixel_data_ptr(&self) -> *const u32 {
        self.pixel_data.as_ptr()
    }
}

impl RustCanvas {
    fn get_pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            Some((y * self.width as i32 + x) as usize)
        } else {
            None
        }
    }

    fn get_pixel_color(&self, x: i32, y: i32) -> Option<Color> {
        if let Some(idx) = self.get_pixel_index(x, y) {
            let pixel_val = self.pixel_data[idx];
            Some(Color {
                r: (pixel_val & 0xff) as u8,
                g: ((pixel_val >> 8) & 0xff) as u8,
                b: ((pixel_val >> 16) & 0xff) as u8,
                a: ((pixel_val >> 24) & 0xff) as u8,
            })
        } else {
            None
        }
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if let Some(idx) = self.get_pixel_index(x, y) {
            let pixel_val: u32 = ((0xff & 0xff) as u32) << 24
                | ((color.b & 0xff) as u32) << 16
                | ((color.g & 0xff) as u32) << 8
                | ((color.r & 0xff) as u32);
            self.pixel_data[idx] = pixel_val;
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

pub struct TrailParticle {
    pos: (f64, f64),
    vel: (f64, f64),
    size: u32,
    color: Color,
    prev_positions: VecDeque<(f64, f64)>,
}

impl TrailParticle {
    fn new(
        pos_x: f64,
        pos_y: f64,
        vel_x: f64,
        vel_y: f64,
        size: u32,
        color: Color,
    ) -> TrailParticle {
        TrailParticle {
            pos: (pos_x, pos_y),
            vel: (vel_x, vel_y),
            size,
            color,
            prev_positions: VecDeque::new(),
        }
    }

    fn update(&mut self, delta: f64) {
        todo!();
    }

    fn render(&self, canvas: &RustCanvas) {
        todo!();
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
