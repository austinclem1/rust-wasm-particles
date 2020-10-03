mod utils;
use rand::Rng;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, WebGlBuffer, WebGlProgram, WebGlRenderingContext, WebGlShader};
extern crate libc;
extern crate nalgebra_glm as glm;
use glm::TMat4;
use std::collections::VecDeque;
use vecmath;

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

pub fn compile_shader(
    context: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGlRenderingContext,
    vertex_shader: &WebGlShader,
    fragment_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.attach_shader(&program, vertex_shader);
    context.attach_shader(&program, fragment_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creaing program object")))
    }
}

#[wasm_bindgen]
pub struct RustCanvas {
    width: u32,
    height: u32,
    gl_context: Option<WebGlRenderingContext>,
    projection_mat: TMat4<f32>,
    shader_program: Option<WebGlProgram>,
    vbo: Option<WebGlBuffer>,
    pixel_buffer: PixelBuffer,
    particles: VecDeque<Particle>,
    particle_trail_length: usize,
    vertex_buffer: Vec<f32>,
    gravity_wells: Vec<GravityWell>,
    gravity_well_mass: f64,
    borders_are_active: bool,
    should_clear_screen: bool,
    rng: rand::rngs::ThreadRng,
}

#[wasm_bindgen]
impl RustCanvas {
    pub fn new() -> RustCanvas {
        utils::set_panic_hook();
        let particles: VecDeque<Particle> = VecDeque::new();
        let vertex_buffer: Vec<f32> = Vec::new();
        let rng = rand::thread_rng();
        let mut rust_canvas = RustCanvas {
            width: 0,
            height: 0,
            gl_context: None,
            projection_mat: glm::zero(),
            shader_program: None,
            vbo: None,
            pixel_buffer: PixelBuffer::new(0, 0),
            particles,
            particle_trail_length: 5,
            vertex_buffer,
            gravity_wells: Vec::new(),
            gravity_well_mass: 200.0,
            borders_are_active: false,
            should_clear_screen: true,
            rng,
        };
        rust_canvas.initialize().unwrap();
        rust_canvas
    }

    pub fn initialize(&mut self) -> Result<(), JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

        self.width = canvas.width();
        self.height = canvas.height();

        self.projection_mat =
            nalgebra_glm::ortho(0.0, self.width as f32, self.height as f32, 0.0, -1.0, 1.0);

        let context = canvas
            .get_context("webgl")?
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()?;

        let vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
            attribute vec2 a_Position;
            attribute vec4 a_Color;

            // uniform float u_TrailScale;
            uniform mat4 u_Proj;

            varying vec4 v_Color;

            void main() {
                gl_Position = u_Proj * vec4(a_Position, 0.0, 1.0);
                v_Color = a_Color;
            }
        "#,
        )?;
        let fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec4 v_Color;

            void main() {
                // gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
                gl_FragColor = v_Color;
            }
        "#,
        )?;
        let program = link_program(&context, &vertex_shader, &fragment_shader)?;
        context.use_program(Some(&program));
        context.enable(WebGlRenderingContext::BLEND);
        context.blend_func(
            WebGlRenderingContext::SRC_ALPHA,
            WebGlRenderingContext::ONE_MINUS_SRC_ALPHA,
        );
        let vbo = context
            .create_buffer()
            .ok_or("failed to create buffer")
            .unwrap();

        self.gl_context = Some(context);
        self.shader_program = Some(program);
        self.vbo = Some(vbo);

        Ok(())
    }

    pub fn initialize_particles(&mut self, num_particles: u32) {
        self.gravity_wells.push(GravityWell {
            pos: [self.width as f64 / 2.0, self.height as f64 / 2.0],
            mass: 200.0,
            is_selected: false,
        });
        self.particles.reserve(num_particles as usize);
        self.vertex_buffer.reserve(num_particles as usize * 12);
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
        let _timer = Timer::new("RustCanvas::update()");
        delta /= 1000.0;

        for well in &self.gravity_wells {
            let mass = self.gravity_well_mass;
            for p in &mut self.particles {
                let p_to_well = vecmath::vec2_sub(well.pos, p.pos);
                let distance_from_gravity = vecmath::vec2_len(p_to_well);
                let grav_force = {
                    if distance_from_gravity <= 30.0 {
                        // mass / 5.0
                        mass / 60.0
                    } else {
                        well.mass / (distance_from_gravity * 2.0)
                        // mass / (distance_from_gravity / 2.0)
                    }
                };
                let force_dir = vecmath::vec2_normalized(p_to_well);
                let acc = vecmath::vec2_scale(force_dir, grav_force);
                p.vel = vecmath::vec2_add(p.vel, acc);
            }
        }

        for p in &mut self.particles {
            // if p.prev_positions.len() >= Particle::MAX_TRAIL_LENGTH {
            //     p.prev_positions.pop_back();
            // }
            p.prev_positions.truncate(self.particle_trail_length - 1);
            p.prev_positions.push_front(p.pos);
            p.pos[0] += p.vel[0] * delta;
            p.pos[1] += p.vel[1] * delta;

            // p.vel = vecmath::vec2_scale(p.vel, 0.9999);
            p.vel = vecmath::vec2_scale(p.vel, 0.999);
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
        }
    }

    pub fn render(&mut self) {
        let _timer = Timer::new("RustCanvas::render");

        let gl_context = self.gl_context.as_ref().unwrap();

        gl_context.clear_color(0.0, 0.0, 0.0, 1.0);
        gl_context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl_context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, self.vbo.as_ref());

        for (i, p) in self.particles.iter().enumerate() {
            let idx = i * 12;
            let from_x = p.pos[0];
            let from_y = p.pos[1];
            let to_x = from_x - (p.vel[0] * 0.1);
            let to_y = from_y - (p.vel[1] * 0.1);
            self.vertex_buffer[idx + 0] = from_x as f32;
            self.vertex_buffer[idx + 1] = from_y as f32;
            self.vertex_buffer[idx + 6] = to_x as f32;
            self.vertex_buffer[idx + 7] = to_y as f32;
        }
        unsafe {
            let vertex_array = js_sys::Float32Array::view(&self.vertex_buffer);
            gl_context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vertex_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }

        let shader_program = self.shader_program.as_ref().unwrap();

        let u_proj_location = gl_context.get_uniform_location(&shader_program, "u_Proj");
        gl_context.uniform_matrix4fv_with_f32_array(
            u_proj_location.as_ref(),
            false,
            self.projection_mat.as_slice(),
        );

        let position_attrib_location =
            gl_context.get_attrib_location(&shader_program, "a_Position"); // as u32;
        let color_attrib_location = gl_context.get_attrib_location(&shader_program, "a_Color"); // as u32;
        if position_attrib_location < 0 || color_attrib_location < 0 {
            console::log_1(&"Invalid attribute location".into());
        }
        let stride = 6 * std::mem::size_of::<f32>() as i32;
        gl_context.vertex_attrib_pointer_with_i32(
            position_attrib_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            stride,
            0,
        );
        gl_context.enable_vertex_attrib_array(position_attrib_location as u32);
        gl_context.vertex_attrib_pointer_with_i32(
            color_attrib_location as u32,
            4,
            WebGlRenderingContext::FLOAT,
            false,
            stride,
            2 * std::mem::size_of::<f32>() as i32,
        );
        gl_context.enable_vertex_attrib_array(color_attrib_location as u32);

        gl_context.draw_arrays(
            WebGlRenderingContext::LINES,
            0,
            self.particles.len() as i32 * 2,
        );

        // for gravity_well in &self.gravity_wells {
        //     gravity_well.render(&mut self.pixel_buffer);
        // }
    }

    pub fn spawn_particle(&mut self, x: f64, y: f64, vel_x: f64, vel_y: f64) {
        // let _timer = Timer::new("RustCanvas::spawn_particle");
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
        let to_x = from_x - (vel_x * 0.1);
        let to_y = from_y - (vel_y * 0.1);
        self.vertex_buffer.append(&mut vec![
            from_x as f32,
            from_y as f32,
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            1.0,
            to_x as f32,
            to_y as f32,
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            0.0,
        ]);
    }

    pub fn spawn_gravity_well(&mut self, x: f64, y: f64) {
        self.gravity_wells.push(GravityWell {
            pos: [x, y],
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

    pub fn clear_particles(&mut self) {
        self.particles.clear();
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

    pub fn set_particle_trail_length(&mut self, length: usize) {
        self.particle_trail_length = length;
    }

    pub fn get_particle_trail_length(&self) -> usize {
        self.particle_trail_length
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

    fn draw_background(&mut self) {
        self.pixel_buffer
            .draw_rect_rgb(0, 0, self.width, self.height, Color::from_u32(0x000000ff));
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

pub struct Particle {
    pos: [f64; 2],
    vel: [f64; 2],
    color: Color,
    prev_positions: VecDeque<[f64; 2]>,
}

impl Particle {
    const MAX_TRAIL_LENGTH: usize = 5;
    const TRAIL_SCALE: f64 = 0.1;

    fn new(pos_x: f64, pos_y: f64, vel_x: f64, vel_y: f64, color: Color) -> Particle {
        Particle {
            pos: [pos_x, pos_y],
            vel: [vel_x, vel_y],
            color,
            prev_positions: VecDeque::with_capacity(Particle::MAX_TRAIL_LENGTH as usize),
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let mut color = self.color;
        color.a = 0xff;
        let from_x = self.pos[0] as i32;
        let from_y = self.pos[1] as i32;
        let to_x = (self.pos[0] + (self.vel[0] * Particle::TRAIL_SCALE)) as i32;
        let to_y = (self.pos[1] + (self.vel[1] * Particle::TRAIL_SCALE)) as i32;
        buffer.draw_line_rgba(from_x, from_y, to_x, to_y, 1, self.color);
    }
}

pub struct GravityWell {
    pos: [f64; 2],
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
        let left = self.pos[0] - (GravityWell::SIZE as f64 / 2.0);
        let top = self.pos[1] - (GravityWell::SIZE as f64 / 2.0);
        let right = self.pos[0] + (GravityWell::SIZE as f64 / 2.0);
        let bottom = self.pos[1] + (GravityWell::SIZE as f64 / 2.0);
        (left, top, right, bottom)
    }

    fn move_by(&mut self, delta_x: f64, delta_y: f64) {
        self.pos[0] += delta_x;
        self.pos[1] += delta_y;
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let size = GravityWell::SIZE;
        let x = self.pos[0] - (size as f64 / 2.0);
        let y = self.pos[1] - (size as f64 / 2.0);
        let mut color = Color::from_u32(0xffffff99);
        if self.is_selected {
            color.tint(Color::from_u32(0x0000ffff));
        }
        buffer.draw_rect_rgba(x as i32, y as i32, size, size, color);
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
