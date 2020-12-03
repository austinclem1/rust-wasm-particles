extern crate libc;
extern crate nalgebra_glm as glm;
mod color;
mod gravity_well;
mod particle;
mod renderer;
mod utils;
mod webgl_helpers;
use color::Color;
use gravity_well::GravityWell;
use particle::Particle;
use rand::Rng;
use renderer::Renderer;
use std::collections::VecDeque;
use vecmath;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{ console, HtmlImageElement, WebGlRenderingContext };

// A timer that calls console.time(`name`) on creation and
// calls console.time.end(`name`) when it is dropped.
// Useful for timing how long a function takes when debugging
// in the browser.
struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(name: &'a str) -> Timer<'a> {
        // console::time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        // console::time_end_with_label(self.name);
    }
}

// Rust portion of the main app, owns all state for the particle simulation
// and decides how to handle user input (sent from JavaScript front end)
// Has a Renderer struct that handles rendering to the WebGl context
// that it gets from the DOM
#[wasm_bindgen]
pub struct WasmApp {
    width: u32,
    height: u32,
    renderer: Option<Renderer>,
    particles: VecDeque<Particle>,
    particle_trail_scale: f64,
    particle_vertex_array: Vec<f32>,
    gravity_wells: Vec<GravityWell>,
    gravity_well_mass: f64,
    borders_are_active: bool,
    should_clear_screen: bool,
    rng: rand::rngs::ThreadRng,
}

#[wasm_bindgen]
impl WasmApp {
    pub fn new() -> WasmApp {
        utils::set_panic_hook();
        let particles: VecDeque<Particle> = VecDeque::new();
        let particle_vertex_array: Vec<f32> = Vec::new();
        let rng = rand::thread_rng();
        let mut rust_canvas = WasmApp {
            width: 0,
            height: 0,
            renderer: None,
            particles,
            particle_trail_scale: 0.1,
            particle_vertex_array,
            gravity_wells: Vec::new(),
            gravity_well_mass: 90.0,
            borders_are_active: false,
            should_clear_screen: true,
            rng,
        };
        rust_canvas
    }

    pub fn initialize(&mut self) -> Result<(), JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

        self.width = canvas.width();
        self.height = canvas.height();

        self.renderer = Some(Renderer::new(&canvas));

        Ok(())
    }

    pub fn initialize_particles(&mut self, num_particles: u32) {
        // self.particles.reserve(num_particles as usize);
        // self.particle_vertex_array
        //     .reserve(num_particles as usize * 12);
        let min_vel = -80.0;
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
        let _timer = Timer::new("WasmApp::update()");
        delta /= 1000.0;

        for well in &mut self.gravity_wells {
            // rotate gravity well
            well.rotation_deg += GravityWell::ROTATION_SPEED;
            well.rotation_deg %= 360.0;

            // calculate and apply gravity force and velocity for each particle
            for p in &mut self.particles {
                let p_to_well = vecmath::vec2_sub(well.pos, p.pos);
                // let distance_squared = f64::max(1.0, f64::powi(vecmath::vec2_len(p_to_well), 2));
                // let distance_squared = f64::max(1.0, f64::sqrt(vecmath::vec2_len(p_to_well)));
                let distance_squared = f64::max(1.0, vecmath::vec2_len(p_to_well) / 30.0);
                let grav_force = self.gravity_well_mass / (distance_squared);
                // let grav_force = self.gravity_well_mass
                //     / (distance_squared * f64::sqrt(distance_squared + Self::SOFTENING_CONSTANT));
                let force_dir = vecmath::vec2_normalized(p_to_well);
                let acc = vecmath::vec2_scale(force_dir, grav_force);
                p.vel = vecmath::vec2_add(p.vel, acc);
                // if vecmath::vec2_len(p.vel) > Particle::MAX_VELOCITY {
                //     p.vel = vecmath::vec2_scale(
                //         p.vel,
                //         Particle::MAX_VELOCITY / vecmath::vec2_len(p.vel),
                //     );
                // }
            }
        }

        for p in &mut self.particles {
            p.pos[0] += p.vel[0] * delta;
            p.pos[1] += p.vel[1] * delta;

            // apply 'drag' to particles
            // p.vel = vecmath::vec2_scale(p.vel, 0.9995);
            p.vel = vecmath::vec2_scale(p.vel, 0.99);
            // p.vel = vecmath::vec2_scale(p.vel, 1.001);

            if self.borders_are_active {
                if p.pos[0] < 0.0 || p.pos[0] >= self.width as f64 {
                    p.vel[0] *= -1.0;
                    // p.pos[0] = p.pos[0].max(0.0);
                    p.pos[0] = f64::max(p.pos[0], 0.0);
                    // p.pos[0] = p.pos[0].min((self.width - p.size) as f64);
                    p.pos[0] = f64::min(p.pos[0], (self.width - 1) as f64);
                }
                if p.pos[1] < 0.0 || p.pos[1] >= self.height as f64 {
                    p.vel[1] *= -1.0;
                    // p.pos[1] = p.pos[1].max(0.0);
                    p.pos[1] = f64::max(p.pos[1], 0.0);
                    // p.pos[1] = p.pos[1].min((self.height - p.size) as f64);
                    p.pos[1] = f64::min(p.pos[1], (self.height - 1) as f64);
                }
            }

            // p.color.r = p.color.r.wrapping_add(1);
            // p.color.g = p.color.g.wrapping_add(1);
            // p.color.b = p.color.b.wrapping_add(1);
        }
    }

    pub fn render(&mut self) {
        let _timer = Timer::new("WasmApp::render");

        match &mut self.renderer {
            None => {
                console::log_1(&"Error: No renderer".into());
                return;
            }
            Some(renderer) => {
                renderer.clear_screen();

                renderer.render_particles(&self.particles, self.particle_trail_scale);

                renderer.render_gravity_wells(&self.gravity_wells);
            }
        }
    }

    pub fn spawn_particle(&mut self, x: f64, y: f64, vel_x: f64, vel_y: f64) {
        // let _timer = Timer::new("WasmApp::spawn_particle");
        let color = Color {
            r: self.rng.gen::<u8>(),
            g: self.rng.gen::<u8>(),
            b: self.rng.gen::<u8>(),
            a: 0xff,
        };
        self.particles
            .push_back(Particle::new(x, y, vel_x, vel_y, color));
        let from_x = x;
        let from_y = y;
        let to_x = from_x - (vel_x * self.particle_trail_scale);
        let to_y = from_y - (vel_y * self.particle_trail_scale);
        if let Some(renderer) = &mut self.renderer {
            renderer.particle_vertex_array.append(&mut vec![
                from_x as f32,
                from_y as f32,
                to_x as f32,
                to_y as f32,
            ]);
            renderer.particle_color_array.append(&mut vec![
                color.r, color.g, color.b, 255, color.r, color.g, color.b, 0,
            ]);
        }
    }

    pub fn spawn_gravity_well(&mut self, x: f64, y: f64) {
        self.gravity_wells.push(GravityWell::new([x, y], 200.0));
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

    pub fn move_selection_to(&mut self, new_x: f64, new_y: f64) {
        for well in &mut self.gravity_wells {
            if well.is_selected {
                well.pos[0] = new_x;
                well.pos[1] = new_y;
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

    pub fn clear_particles(&mut self) {
        self.particles.clear();
        if let Some(renderer) = &mut self.renderer {
            renderer.particle_color_array.clear();
            renderer.particle_vertex_array.clear();
        }
    }

    pub fn remove_particles(&mut self, num_to_remove: usize) {
        let num_to_remove = usize::min(self.particles.len(), num_to_remove);
        drop(self.particles.drain(0..num_to_remove));
    }

    pub fn set_gravity_well_mass(&mut self, new_mass: f64) {
        self.gravity_well_mass = new_mass;
    }

    pub fn get_gravity_well_mass(&self) -> f64 {
        self.gravity_well_mass
    }

    pub fn get_particle_count(&self) -> usize {
        self.particles.len()
    }

    pub fn set_particle_trail_scale(&mut self, scale: f64) {
        self.particle_trail_scale = scale;
    }

    pub fn get_particle_trail_scale(&self) -> f64 {
        self.particle_trail_scale
    }

    pub fn set_borders_active(&mut self, new_state: bool) {
        if new_state == true {
            // let mut particle_indices_to_delete = Vec::new();
            // for i in 0..self.particles.len() {
            //     let p = &self.particles[i];
            //     if p.pos[0] < 0.0
            //         || p.pos[1] < 0.0
            //         || p.pos[1] >= (self.height - p.size) as f64
            //     {
            //         particle_indices_to_delete.push(i);
            //     }
            // }
            // for idx in particle_indices_to_delete.iter().rev().copied() {
            //     self.particles.remove(idx);
            // }
            // let width = self.width;
            // let height = self.height;
            // self.particles.retain(|p| {
            //     p.pos[0] >= 0.0
            //         && p.pos[0] < (width - p.size) as f64
            //         && p.pos[1] >= 0.0
            //         && p.pos[1] < (height - p.size) as f64
            // });
            for i in (0..self.particles.len()).rev() {
                let p = &self.particles[i];
                if p.pos[0] < 0.0
                    || p.pos[0] >= (self.width - 1) as f64
                    || p.pos[1] < 0.0
                    || p.pos[1] >= (self.height - 1) as f64
                {
                    self.particles.swap_remove_back(i);
                }
            }
        }

        self.borders_are_active = new_state;
    }

    pub fn set_should_clear_screen(&mut self, new_state: bool) {
        self.should_clear_screen = new_state;
    }

    pub fn add_texture_from_image(&mut self, name: String, image: &HtmlImageElement) {
        if let Some(renderer) = &mut self.renderer {
            let texture = renderer.context.create_texture();
            renderer
                .context
                .bind_texture(WebGlRenderingContext::TEXTURE_2D, texture.as_ref());
            renderer
                .context
                .tex_image_2d_with_u32_and_u32_and_image(
                    WebGlRenderingContext::TEXTURE_2D,
                    0,
                    WebGlRenderingContext::RGBA as i32,
                    WebGlRenderingContext::RGBA,
                    WebGlRenderingContext::UNSIGNED_BYTE,
                    image,
                )
                .expect("failed to buffer image data to gravity well texture");
            if is_power_of_2(image.width()) && is_power_of_2(image.height()) {
                renderer
                    .context
                    .generate_mipmap(WebGlRenderingContext::TEXTURE_2D);
            } else {
                renderer.context.tex_parameteri(
                    WebGlRenderingContext::TEXTURE_2D,
                    WebGlRenderingContext::TEXTURE_WRAP_S,
                    WebGlRenderingContext::CLAMP_TO_EDGE as i32,
                );
                renderer.context.tex_parameteri(
                    WebGlRenderingContext::TEXTURE_2D,
                    WebGlRenderingContext::TEXTURE_WRAP_T,
                    WebGlRenderingContext::CLAMP_TO_EDGE as i32,
                );
                renderer.context.tex_parameteri(
                    WebGlRenderingContext::TEXTURE_2D,
                    WebGlRenderingContext::TEXTURE_MIN_FILTER,
                    WebGlRenderingContext::LINEAR as i32,
                );
            }
            renderer.textures.insert(name, texture);
        }
    }
}

impl WasmApp {
    const SOFTENING_CONSTANT: f64 = 600.0;
}

fn is_power_of_2(n: u32) -> bool {
    if (n & (n - 1)) == 0 {
        true
    } else {
        false
    }
}
