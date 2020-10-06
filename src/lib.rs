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

struct Timer<'a> {
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

fn compile_shader(
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

fn link_program(
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
impl RustCanvas {
    pub fn new() -> RustCanvas {
        utils::set_panic_hook();
        let particles: VecDeque<Particle> = VecDeque::new();
        let particle_vertex_array: Vec<f32> = Vec::new();
        let rng = rand::thread_rng();
        let mut rust_canvas = RustCanvas {
            width: 0,
            height: 0,
            renderer: None,
            particles,
            particle_trail_scale: 0.1,
            particle_vertex_array,
            gravity_wells: Vec::new(),
            gravity_well_mass: 200.0,
            borders_are_active: false,
            should_clear_screen: true,
            rng,
        };
        rust_canvas.initialize().unwrap();
        rust_canvas
    }

    fn initialize(&mut self) -> Result<(), JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

        self.width = canvas.width();
        self.height = canvas.height();

        self.renderer = Some(Renderer::new(&canvas));

        Ok(())
    }

    pub fn initialize_particles(&mut self, num_particles: u32) {
        self.particles.reserve(num_particles as usize);
        self.particle_vertex_array
            .reserve(num_particles as usize * 12);
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
            // let mass = self.gravity_well_mass;
            for p in &mut self.particles {
                let p_to_well = vecmath::vec2_sub(well.pos, p.pos);
                // let mut distance_from_gravity = vecmath::vec2_len(p_to_well);
                let distance_from_gravity = (15.0f64).max(vecmath::vec2_len(p_to_well));
                let grav_force = self.gravity_well_mass / (distance_from_gravity * 2.0);
                let force_dir = vecmath::vec2_normalized(p_to_well);
                let acc = vecmath::vec2_scale(force_dir, grav_force);
                p.vel = vecmath::vec2_add(p.vel, acc);
            }
        }

        for p in &mut self.particles {
            p.pos[0] += p.vel[0] * delta;
            p.pos[1] += p.vel[1] * delta;

            // p.vel = vecmath::vec2_scale(p.vel, 0.9999);
            p.vel = vecmath::vec2_scale(p.vel, 0.998);
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
        let _timer = Timer::new("RustCanvas::render");

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
        let to_x = from_x - (vel_x * self.particle_trail_scale);
        let to_y = from_y - (vel_y * self.particle_trail_scale);
        if let Some(renderer) = &mut self.renderer {
            renderer.particle_vertex_array.append(&mut vec![
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
    }

    pub fn spawn_gravity_well(&mut self, x: f64, y: f64) {
        self.gravity_wells.push(GravityWell {
            pos: [x, y],
            mass: 200.0,
            is_selected: false,
        });
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

    pub fn set_particle_trail_length(&mut self, scale: f64) {
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
}

struct Renderer {
    context: WebGlRenderingContext,
    projection_mat: TMat4<f32>,
    particle_vbo: WebGlBuffer,
    gravity_well_vbo: WebGlBuffer,
    particle_shader: WebGlProgram,
    gravity_well_shader: WebGlProgram,
    particle_vertex_array: Vec<f32>,
}

impl Renderer {
    fn new(canvas: &web_sys::HtmlCanvasElement) -> Self {
        let context = canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .unwrap();
        let particle_vertex_shader = compile_shader(
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
        )
        .unwrap();
        let particle_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec4 v_Color;

            void main() {
                gl_FragColor = v_Color;
            }
        "#,
        )
        .unwrap();
        let particle_shader =
            link_program(&context, &particle_vertex_shader, &particle_fragment_shader).unwrap();
        let gravity_well_vertex_shader = compile_shader(
            &context,
            WebGlRenderingContext::VERTEX_SHADER,
            r#"
            attribute vec2 a_Position;
            
            uniform mat4 u_Model;
            uniform mat4 u_Proj;
            uniform bool u_IsSelected;

            varying vec4 v_Color;
            
            void main() {
                gl_Position = u_Proj * u_Model * vec4(a_Position, 0.0, 1.0);
                if(u_IsSelected) {
                    v_Color = vec4(0.3, 0.5, 1.0, 1.0);
                } else {
                    v_Color = vec4(0.5, 0.5, 0.5, 1.0);
                }
            }
            "#,
        )
        .unwrap();
        let gravity_well_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec4 v_Color;

            void main() {
                gl_FragColor = v_Color;
            }
        "#,
        )
        .unwrap();
        let gravity_well_shader = link_program(
            &context,
            &gravity_well_vertex_shader,
            &gravity_well_fragment_shader,
        )
        .unwrap();
        context.enable(WebGlRenderingContext::BLEND);
        context.blend_func(
            WebGlRenderingContext::SRC_ALPHA,
            WebGlRenderingContext::ONE_MINUS_SRC_ALPHA,
        );
        // TODO Set position and color location explicitly (before or after linking?)
        let particle_vbo = context
            .create_buffer()
            .ok_or("failed to create buffer")
            .unwrap();
        let gravity_well_vbo = context
            .create_buffer()
            .ok_or("failed to create buffer")
            .unwrap();

        let projection_mat = nalgebra_glm::ortho(
            0.0,
            canvas.width() as f32,
            canvas.height() as f32,
            0.0,
            -1.0,
            1.0,
        );

        Renderer {
            context,
            projection_mat,
            particle_vbo,
            gravity_well_vbo,
            particle_shader,
            gravity_well_shader,
            particle_vertex_array: Vec::new(),
        }
    }

    fn clear_screen(&self) {
        self.context.clear_color(0.0, 0.0, 0.0, 1.0);
        self.context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
    }

    // TODO make particle system its own struct
    // give it its own trail scale, and particle vec
    fn render_particles(&mut self, particles: &VecDeque<Particle>, trail_scale: f64) {
        self.context.use_program(Some(&self.particle_shader));
        self.context.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.particle_vbo),
        );

        for (i, p) in particles.iter().enumerate() {
            let idx = i * 12;
            let from_x = p.pos[0];
            let from_y = p.pos[1];
            let to_x = from_x - (p.vel[0] * trail_scale);
            let to_y = from_y - (p.vel[1] * trail_scale);
            self.particle_vertex_array[idx + 0] = from_x as f32;
            self.particle_vertex_array[idx + 1] = from_y as f32;
            self.particle_vertex_array[idx + 2] = p.color.r as f32 / 255.0;
            self.particle_vertex_array[idx + 3] = p.color.g as f32 / 255.0;
            self.particle_vertex_array[idx + 4] = p.color.b as f32 / 255.0;
            self.particle_vertex_array[idx + 5] = 1.0;
            self.particle_vertex_array[idx + 6] = to_x as f32;
            self.particle_vertex_array[idx + 7] = to_y as f32;
            self.particle_vertex_array[idx + 8] = p.color.r as f32 / 255.0;
            self.particle_vertex_array[idx + 9] = p.color.g as f32 / 255.0;
            self.particle_vertex_array[idx + 10] = p.color.b as f32 / 255.0;
            self.particle_vertex_array[idx + 11] = 0.0;
        }
        unsafe {
            let vertex_array = js_sys::Float32Array::view(&self.particle_vertex_array);
            self.context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vertex_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }

        let u_proj_location = self
            .context
            .get_uniform_location(&self.particle_shader, "u_Proj");
        self.context.uniform_matrix4fv_with_f32_array(
            u_proj_location.as_ref(),
            false,
            self.projection_mat.as_slice(),
        );

        let position_attrib_location = self
            .context
            .get_attrib_location(&self.particle_shader, "a_Position"); // as u32;
        let color_attrib_location = self
            .context
            .get_attrib_location(&self.particle_shader, "a_Color"); // as u32;
        if position_attrib_location < 0 || color_attrib_location < 0 {
            console::log_1(&"Invalid attribute location".into());
        }
        let stride = 6 * std::mem::size_of::<f32>() as i32;
        self.context.vertex_attrib_pointer_with_i32(
            position_attrib_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            stride,
            0,
        );
        self.context
            .enable_vertex_attrib_array(position_attrib_location as u32);
        let color_attrib_offset = 2 * std::mem::size_of::<f32>() as i32;
        self.context.vertex_attrib_pointer_with_i32(
            color_attrib_location as u32,
            4,
            WebGlRenderingContext::FLOAT,
            false,
            stride,
            color_attrib_offset,
        );
        self.context
            .enable_vertex_attrib_array(color_attrib_location as u32);

        self.context
            .draw_arrays(WebGlRenderingContext::LINES, 0, particles.len() as i32 * 2);
    }

    fn render_gravity_wells(&self, gravity_wells: &Vec<GravityWell>) {
        let vertex_array = vec![
            GravityWell::SIZE as f32,
            -(GravityWell::SIZE as f32),
            -(GravityWell::SIZE as f32),
            -(GravityWell::SIZE as f32),
            -(GravityWell::SIZE as f32),
            GravityWell::SIZE as f32,
            GravityWell::SIZE as f32,
            -(GravityWell::SIZE as f32),
            -(GravityWell::SIZE as f32),
            GravityWell::SIZE as f32,
            GravityWell::SIZE as f32,
            GravityWell::SIZE as f32,
        ];
        self.context.use_program(Some(&self.gravity_well_shader));
        self.context.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.gravity_well_vbo),
        );
        unsafe {
            let vertex_array = js_sys::Float32Array::view(&vertex_array);
            self.context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vertex_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }

        let u_proj_location = self
            .context
            .get_uniform_location(&self.gravity_well_shader, "u_Proj");
        self.context.uniform_matrix4fv_with_f32_array(
            u_proj_location.as_ref(),
            false,
            self.projection_mat.as_slice(),
        );

        let u_model_location = self
            .context
            .get_uniform_location(&self.gravity_well_shader, "u_Model");

        let u_is_selected_location = self
            .context
            .get_uniform_location(&self.gravity_well_shader, "u_IsSelected");

        let position_attrib_location = self
            .context
            .get_attrib_location(&self.particle_shader, "a_Position");

        self.context.vertex_attrib_pointer_with_i32(
            position_attrib_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );
        self.context
            .enable_vertex_attrib_array(position_attrib_location as u32);

        for gravity_well in gravity_wells {
            self.context.uniform_matrix4fv_with_f32_array(
                u_model_location.as_ref(),
                false,
                glm::translation(&glm::vec3(
                    gravity_well.pos[0] as f32,
                    gravity_well.pos[1] as f32,
                    0.0,
                ))
                .as_slice(),
            );
            self.context.uniform1i(
                u_is_selected_location.as_ref(),
                gravity_well.is_selected as i32,
            );
            self.context
                .draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);
        }
    }
}

struct Particle {
    pos: [f64; 2],
    vel: [f64; 2],
    color: Color,
}

impl Particle {
    fn new(pos_x: f64, pos_y: f64, vel_x: f64, vel_y: f64, color: Color) -> Particle {
        Particle {
            pos: [pos_x, pos_y],
            vel: [vel_x, vel_y],
            color,
        }
    }
}

struct GravityWell {
    pos: [f64; 2],
    mass: f64,
    is_selected: bool,
}

impl GravityWell {
    const SIZE: u32 = 10;

    fn is_point_inside(&self, x: i32, y: i32) -> bool {
        let (left, top, right, bottom) = self.get_rect();
        let x = x as f64;
        let y = y as f64;
        x >= left && y >= top && x <= right && y <= bottom
    }

    fn get_rect(&self) -> (f64, f64, f64, f64) {
        let left = self.pos[0] - (GravityWell::SIZE as f64);
        let top = self.pos[1] - (GravityWell::SIZE as f64);
        let right = self.pos[0] + (GravityWell::SIZE as f64);
        let bottom = self.pos[1] + (GravityWell::SIZE as f64);
        (left, top, right, bottom)
    }

    fn move_by(&mut self, delta_x: f64, delta_y: f64) {
        self.pos[0] += delta_x;
        self.pos[1] += delta_y;
    }
}

#[derive(Copy, Clone)]
struct Color {
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
