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

#[wasm_bindgen]
pub struct RustCanvas {
    width: u32,
    height: u32,
    pixel_buffer: PixelBuffer,
    particles: Vec<TrailParticle>,
    gravity_wells: Vec<GravityWell>,
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
            pixel_buffer: PixelBuffer::new(width, height),
            particles,
            gravity_wells: Vec::new(),
            rng,
        }
    }

    pub fn initialize_particles(&mut self, num_particles: u32) {
        self.gravity_wells.push(GravityWell {
            pos: (self.width as f64 / 2.0, self.height as f64 / 2.0),
            mass: 200.0,
            is_selected: false,
        });
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

    pub fn update(&mut self, mut delta: f64) {
        let _timer = Timer::new("RustCanvas::update()");
        delta /= 1000.0;

        for gravity_well in &self.gravity_wells {
            for particle in &mut self.particles {
                let (px, py) = particle.pos;
                let (grav_x, grav_y) = gravity_well.pos;
                let delta_x = grav_x - px;
                let delta_y = grav_y - py;
                let distance_from_gravity = vecmath::vec2_len([delta_x, delta_y]);
                let grav_force = {
                    if distance_from_gravity <= 5.0 {
                        gravity_well.mass
                    } else {
                        // gravity_well.mass / (distance_from_gravity * 2.0)
                        gravity_well.mass / distance_from_gravity
                    }
                };
                let force_dir = vecmath::vec2_normalized([delta_x, delta_y]);
                let acc = vecmath::vec2_scale(force_dir, grav_force);
                particle.vel.0 += acc[0];
                particle.vel.1 += acc[1];
            }
        }

        for particle in &mut self.particles {
            if particle.prev_positions.len() >= TrailParticle::MAX_TRAIL_LENGTH {
                particle.prev_positions.pop_back();
            }
            particle.prev_positions.push_front(particle.pos);
            particle.pos.0 += particle.vel.0 * delta;
            particle.pos.1 += particle.vel.1 * delta;

            // particle.vel.0 *= 0.995;
            // particle.vel.1 *= 0.995;
            // if particle.pos.0 < 0.0 || particle.pos.0 >= self.width as f64 {
            //     particle.vel.0 *= -0.8;
            //     particle.pos.0 = particle.pos.0.max(0.0);
            //     particle.pos.0 = particle.pos.0.min((self.width - 1) as f64);
            // }
            // if particle.pos.1 < 0.0 || particle.pos.1 >= self.height as f64 {
            //     particle.vel.1 *= -0.8;
            //     particle.pos.1 = particle.pos.1.max(0.0);
            //     particle.pos.1 = particle.pos.1.min((self.height - 1) as f64);
            // }
        }
    }

    pub fn render(&mut self) {
        let _timer = Timer::new("RustCanvas::render");
        {
            let _timer = Timer::new("draw background");
            self.pixel_buffer.draw_rect_rgb(
                0,
                0,
                self.width,
                self.height,
                Color::from_u32(0x000000ff),
            );
        }
        {
            let _timer = Timer::new("draw particles");
            // for i in 0..self.particles.len() {
            //     let p = &self.particles[i];
            //     let rect_size = p.size;
            //     let mut rect_color = p.color;
            //     let mut pos_vec = p.prev_positions.clone();
            //     pos_vec.push_front(p.pos);
            //     for pos in pos_vec {
            //         rect_color.a = (rect_color.a as f64 * TrailParticle::TRAIL_FADE_RATIO) as u8;
            //         self.pixel_buffer.draw_rect_rgba(
            //             pos.0 as i32,
            //             pos.1 as i32,
            //             rect_size,
            //             rect_size,
            //             rect_color,
            //         );
            //     }
            // }
            for particle in &self.particles {
                let mut color = particle.color;
                color.a = 0xff;
                let alpha_reduction = 0xff as i32 / particle.prev_positions.len() as i32;
                let alpha_reduction = if alpha_reduction < 1 {
                    1
                } else {
                    (alpha_reduction as u8).saturating_mul(1)
                };
                let mut from_x = particle.pos.0 as i32;
                let mut from_y = particle.pos.1 as i32;
                for to_pos in &particle.prev_positions {
                    let to_x = to_pos.0 as i32;
                    let to_y = to_pos.1 as i32;
                    self.pixel_buffer
                        .draw_line_rgba(from_x, from_y, to_x, to_y, 1, color);
                    from_x = to_x;
                    from_y = to_y;
                    color.a -= alpha_reduction;
                }
            }
        }

        for gravity_well in &self.gravity_wells {
            let size = GravityWell::SIZE;
            let x = gravity_well.pos.0 - (size as f64 / 2.0);
            let y = gravity_well.pos.1 - (size as f64 / 2.0);
            let mut color = Color::from_u32(0xffffff99);
            if gravity_well.is_selected {
                color.tint(Color::from_u32(0x0000ffff));
            }
            self.pixel_buffer
                .draw_rect_rgba(x as i32, y as i32, size, size, color);
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

    pub fn spawn_gravity_well(&mut self, x: f64, y: f64) {
        self.gravity_wells.push(GravityWell {
            pos: (x, y),
            mass: 200.0,
            is_selected: false,
        });
    }

    pub fn get_pixel_buffer_ptr(&self) -> *const u32 {
        self.pixel_buffer.get_ptr()
    }

    pub fn try_selecting(&mut self, x: i32, y: i32) -> bool {
        for well in &mut self.gravity_wells {
            if well.is_point_inside(x, y) {
                well.is_selected = true;
                return true;
            }
        }
        false
    }

    pub fn release_selection(&mut self) {
        for well in &mut self.gravity_wells {
            well.is_selected = false;
        }
    }

    pub fn drag_selection(&mut self, delta_x: f64, delta_y: f64) {
        for well in &mut self.gravity_wells {
            if well.is_selected {
                well.move_by(delta_x, delta_y);
            }
        }
    }

    pub fn try_removing(&mut self, x: f64, y: f64) {
        for i in 0..self.gravity_wells.len() as usize {
            if self.gravity_wells[i].is_point_inside(x as i32, y as i32) {
                self.gravity_wells.remove(i);
                return;
            }
        }
    }
}

pub struct PixelBuffer {
    width: u32,
    height: u32,
    data: Vec<u32>,
}

impl PixelBuffer {
    pub fn new(width: u32, height: u32) -> PixelBuffer {
        PixelBuffer {
            width,
            height,
            data: vec![0x00000000; (width * height) as usize],
        }
    }

    fn get_pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            Some((y * self.width as i32 + x) as usize)
        } else {
            None
        }
    }

    fn get_pixel_color(&self, x: i32, y: i32) -> Option<Color> {
        if let Some(idx) = self.get_pixel_index(x, y) {
            let pixel_val = self.data[idx];
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

    fn set_pixel_rgba(&mut self, x: i32, y: i32, color: Color) {
        if let Some(idx) = self.get_pixel_index(x, y) {
            let old_pixel_color = self.get_pixel_color(x, y).unwrap();
            let blend_ratio = color.a as f64 / 255.0;
            let mut blended_color = Color::from_u32(0x000000ff);
            blended_color.r = (color.r as f64 * blend_ratio
                + (old_pixel_color.r as f64 * (1.0 - blend_ratio)))
                as u8;
            blended_color.g = (color.g as f64 * blend_ratio
                + (old_pixel_color.g as f64 * (1.0 - blend_ratio)))
                as u8;
            blended_color.b = (color.b as f64 * blend_ratio
                + (old_pixel_color.b as f64 * (1.0 - blend_ratio)))
                as u8;
            let blended_pixel_val: u32 = ((0xff & 0xff) as u32) << 24
                | ((blended_color.b & 0xff) as u32) << 16
                | ((blended_color.g & 0xff) as u32) << 8
                | ((blended_color.r & 0xff) as u32);
            self.data[idx] = blended_pixel_val;
        }
    }

    fn set_pixel_rgb(&mut self, x: i32, y: i32, color: Color) {
        if let Some(idx) = self.get_pixel_index(x, y) {
            let pixel_val: u32 = ((0xff & 0xff) as u32) << 24
                | ((color.b & 0xff) as u32) << 16
                | ((color.g & 0xff) as u32) << 8
                | ((color.r & 0xff) as u32);
            self.data[idx] = pixel_val;
        }
    }

    fn draw_line_rgba(
        &mut self,
        mut x0: i32,
        mut y0: i32,
        x1: i32,
        y1: i32,
        thickness: u32,
        color: Color,
    ) {
        let delta_x = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let delta_y = (y1 - y0).abs() * -1;
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = delta_x + delta_y;
        loop {
            self.set_pixel_rgba(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= delta_y {
                err += delta_y;
                x0 += sx;
            }
            if e2 <= delta_x {
                err += delta_x;
                y0 += sy;
            }
        }
    }

    fn draw_rect_rgba(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color) {
        for pixel_y in y..y + height as i32 {
            for pixel_x in x..x + width as i32 {
                self.set_pixel_rgba(pixel_x, pixel_y, color);
            }
        }
    }

    fn draw_rect_rgb(&mut self, x: i32, y: i32, width: u32, height: u32, color: Color) {
        for pixel_y in y..y + height as i32 {
            for pixel_x in x..x + width as i32 {
                self.set_pixel_rgb(pixel_x, pixel_y, color);
            }
        }
    }

    fn get_ptr(&self) -> *const u32 {
        self.data.as_ptr()
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
    const MAX_TRAIL_LENGTH: usize = 5;

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
            prev_positions: VecDeque::with_capacity(TrailParticle::MAX_TRAIL_LENGTH as usize),
        }
    }

    fn update(&mut self, delta: f64) {
        todo!();
    }

    fn render(&self, canvas: &RustCanvas) {
        todo!();
    }
}

pub struct GravityWell {
    pos: (f64, f64),
    mass: f64,
    is_selected: bool,
}

impl GravityWell {
    const SIZE: u32 = 20;

    fn is_point_inside(&self, x: i32, y: i32) -> bool {
        let (left, top, right, bottom) = self.get_rect();
        let x = x as f64;
        let y = y as f64;
        x >= left && y >= top && x <= right && y <= bottom
    }

    fn get_rect(&self) -> (f64, f64, f64, f64) {
        let (x, y) = self.pos;
        let left = x - (GravityWell::SIZE as f64 / 2.0);
        let top = y - (GravityWell::SIZE as f64 / 2.0);
        let right = x + (GravityWell::SIZE as f64 / 2.0);
        let bottom = y + (GravityWell::SIZE as f64 / 2.0);
        (left, top, right, bottom)
    }

    fn move_by(&mut self, delta_x: f64, delta_y: f64) {
        let (mut x, mut y) = self.pos;
        x += delta_x;
        y += delta_y;
        self.pos = (x, y);
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

    fn tint(&mut self, tint: Color) {
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
