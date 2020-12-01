mod utils;
use rand::Rng;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    console, HtmlImageElement, WebGlBuffer, WebGlProgram, WebGlRenderingContext, WebGlShader,
    WebGlTexture,
};
extern crate libc;
extern crate nalgebra_glm as glm;
use glm::TMat4;
use std::collections::{HashMap, VecDeque};
use vecmath;

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

    pub fn update_1(&mut self, mut delta: f64) {
        let _timer = Timer::new("RustCanvas::update()");
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

    pub fn update_2(&mut self, mut delta: f64) {
        let _timer = Timer::new("RustCanvas::update()");
        delta /= 1000.0;

        for well in &mut self.gravity_wells {
            well.rotation_deg += GravityWell::ROTATION_SPEED;
            well.rotation_deg %= 360.0;
        }

        for p in &mut self.particles {
            for well in &self.gravity_wells {
                let p_to_well = vecmath::vec2_sub(well.pos, p.pos);
                let distance_squared = f64::max(1.0, f64::sqrt(vecmath::vec2_len(p_to_well)));
                let grav_force = self.gravity_well_mass / (distance_squared);
                let force_dir = vecmath::vec2_normalized(p_to_well);
                let acc = vecmath::vec2_scale(force_dir, grav_force);
                p.vel = vecmath::vec2_add(p.vel, acc);
            }
            p.pos[0] += p.vel[0] * delta;
            p.pos[1] += p.vel[1] * delta;

            // apply 'drag' to particles
            p.vel = vecmath::vec2_scale(p.vel, 0.99);

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
                    p.pos[1] = f64::max(p.pos[1], 0.0);
                    p.pos[1] = f64::min(p.pos[1], (self.height - 1) as f64);
                }
            }
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

impl RustCanvas {
    const SOFTENING_CONSTANT: f64 = 600.0;
}

struct Renderer {
    context: WebGlRenderingContext,
    textures: HashMap<String, Option<WebGlTexture>>,
    projection_mat: TMat4<f32>,
    particle_vertex_buffer: WebGlBuffer,
    particle_color_buffer: WebGlBuffer,
    gravity_well_vbo: WebGlBuffer,
    particle_shader: WebGlProgram,
    gravity_well_shader: WebGlProgram,
    particle_vertex_array: Vec<f32>,
    particle_color_array: Vec<u8>,
}

impl Renderer {
    fn new(canvas: &web_sys::HtmlCanvasElement) -> Self {
        let context = canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .unwrap();

        let projection_mat = glm::ortho(0.0, canvas.width() as f32, canvas.height() as f32, 0.0, 1.0, -1.0);

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
            attribute vec2 a_TexCoord;
            
            uniform bool u_IsSelected;
            uniform mat4 u_Model;
            uniform mat4 u_Proj;

            varying mediump vec2 v_TexCoord;
            
            void main() {
                gl_Position = u_Proj * u_Model * vec4(a_Position, 0.0, 1.0);
                v_TexCoord = a_TexCoord;
                // if(u_IsSelected) {
                //     v_Color = vec4(0.3, 0.5, 1.0, 1.0);
                // } else {
                //     v_Color = vec4(0.5, 0.5, 0.5, 1.0);
                // }
            }
            "#,
        )
        .unwrap();
        let gravity_well_fragment_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            r#"
            // precision mediump float;

            varying mediump vec2 v_TexCoord;

            uniform sampler2D u_Sampler;

            void main() {
                gl_FragColor = texture2D(u_Sampler, v_TexCoord);
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
        let particle_vertex_buffer = context
            .create_buffer()
            .ok_or("failed to create buffer")
            .unwrap();
        let particle_color_buffer = context
            .create_buffer()
            .ok_or("failed to create buffer")
            .unwrap();

        let gravity_well_vbo = context
            .create_buffer()
            .ok_or("failed to create buffer")
            .unwrap();

        let mut textures = HashMap::new();
        let not_found_texture = context.create_texture();
        context.bind_texture(
            WebGlRenderingContext::TEXTURE_2D,
            not_found_texture.as_ref(),
        );
        context
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                WebGlRenderingContext::TEXTURE_2D,
                0,
                WebGlRenderingContext::RGBA as i32,
                1,
                1,
                0,
                WebGlRenderingContext::RGBA,
                WebGlRenderingContext::UNSIGNED_BYTE,
                Some(&[0u8, 0u8, 255u8, 255u8]),
            )
            .expect("failed to create not_found texture");
        textures.insert("not_found".to_owned(), not_found_texture);

        Renderer {
            context,
            textures,
            projection_mat,
            particle_vertex_buffer,
            particle_color_buffer,
            gravity_well_vbo,
            particle_shader,
            gravity_well_shader,
            particle_vertex_array: Vec::new(),
            particle_color_array: Vec::new(),
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

        let position_attrib_location = self
            .context
            .get_attrib_location(&self.particle_shader, "a_Position");
        let color_attrib_location = self
            .context
            .get_attrib_location(&self.particle_shader, "a_Color");
        if position_attrib_location < 0 || color_attrib_location < 0 {
            console::log_1(&"Invalid attribute location".into());
        }

        for (i, p) in particles.iter().enumerate() {
            let pos_idx = i * 4;
            let color_idx = i * 8;
            let from_x = p.pos[0];
            let from_y = p.pos[1];
            let line_delta_x = match -1.0 * p.vel[0] * trail_scale {
                n if n.abs() >= 1.0 => n,
                _ => 1.0,
            };
            let line_delta_y = match -1.0 * p.vel[1] * trail_scale {
                n if n.abs() >= 1.0 => n,
                _ => 1.0,
            };
            let to_x = from_x + line_delta_x;
            let to_y = from_y + line_delta_y;
            self.particle_vertex_array[pos_idx + 0] = from_x as f32;
            self.particle_vertex_array[pos_idx + 1] = from_y as f32;
            self.particle_vertex_array[pos_idx + 2] = to_x as f32;
            self.particle_vertex_array[pos_idx + 3] = to_y as f32;

            self.particle_color_array[color_idx + 0] = p.color.r;
            self.particle_color_array[color_idx + 1] = p.color.g;
            self.particle_color_array[color_idx + 2] = p.color.b;
            self.particle_color_array[color_idx + 3] = 255;
            self.particle_color_array[color_idx + 4] = p.color.r;
            self.particle_color_array[color_idx + 5] = p.color.g;
            self.particle_color_array[color_idx + 6] = p.color.b;
            self.particle_color_array[color_idx + 7] = 0;
        }

        self.context.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.particle_vertex_buffer),
        );
        unsafe {
            let vertex_array = js_sys::Float32Array::view(&self.particle_vertex_array);
            self.context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vertex_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }
        let position_buffer_stride = 2 * std::mem::size_of::<f32>() as i32;
        self.context.vertex_attrib_pointer_with_i32(
            position_attrib_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            // position_buffer_stride,
            0,
            0,
        );
        self.context
            .enable_vertex_attrib_array(position_attrib_location as u32);

        self.context.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.particle_color_buffer),
        );
        unsafe {
            let color_array = js_sys::Uint8Array::view(&self.particle_color_array);
            self.context.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &color_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }

        let color_buffer_stride = 4 * std::mem::size_of::<u8>() as i32;
        self.context.vertex_attrib_pointer_with_i32(
            color_attrib_location as u32,
            4,
            WebGlRenderingContext::UNSIGNED_BYTE,
            true,
            // color_buffer_stride,
            0,
            0,
        );
        self.context
            .enable_vertex_attrib_array(color_attrib_location as u32);

        let u_proj_location = self
            .context
            .get_uniform_location(&self.particle_shader, "u_Proj")
            .expect("Failed to get u_Proj uniform location");
        self.context.uniform_matrix4fv_with_f32_array(
            Some(&u_proj_location),
            false,
            self.projection_mat.as_slice(),
        );

        self.context
            .draw_arrays(WebGlRenderingContext::LINES, 0, particles.len() as i32 * 2);
    }

    fn render_gravity_wells(&self, gravity_wells: &Vec<GravityWell>) {
        let vertex_array = vec![
            // top right
            GravityWell::RADIUS as f32,
            -(GravityWell::RADIUS as f32),
            1.0,
            1.0,
            // top left
            -(GravityWell::RADIUS as f32),
            -(GravityWell::RADIUS as f32),
            0.0,
            1.0,
            // bottom left
            -(GravityWell::RADIUS as f32),
            GravityWell::RADIUS as f32,
            0.0,
            0.0,
            // top right
            GravityWell::RADIUS as f32,
            -(GravityWell::RADIUS as f32),
            1.0,
            1.0,
            // bottom left
            -(GravityWell::RADIUS as f32),
            GravityWell::RADIUS as f32,
            0.0,
            0.0,
            // bottom right
            GravityWell::RADIUS as f32,
            GravityWell::RADIUS as f32,
            1.0,
            0.0,
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

        // TODO try just a 2D projection instead of 4D matrix
        let u_proj_location = self
            .context
            .get_uniform_location(&self.gravity_well_shader, "u_Proj")
            .expect("failed to get u_Proj uniform location");
        self.context.uniform_matrix4fv_with_f32_array(
            Some(&u_proj_location),
            false,
            self.projection_mat.as_slice(),
        );

        // let u_is_selected_location = match self
        //     .context
        //     .get_uniform_location(&self.gravity_well_shader, "u_IsSelected")
        // {
        //     Some(location) => location,
        //     None => {
        //         let error_code = self.context.get_error();
        //         console::log_1(
        //             &format!(
        //                 "failed to get u_IsSelected uniform location. error code: {}",
        //                 error_code
        //             )
        //             .into(),
        //         );
        //         panic!();
        //     }
        // };
        let u_model_location = self
            .context
            .get_uniform_location(&self.gravity_well_shader, "u_Model")
            .expect("failed to get u_Model uniform location");

        let u_sampler_location = self
            .context
            .get_uniform_location(&self.gravity_well_shader, "u_Sampler")
            .expect("failed to get u_Sampler uniform location");

        let position_attrib_location = self
            .context
            .get_attrib_location(&self.gravity_well_shader, "a_Position");

        let tex_coord_attrib_location = self
            .context
            .get_attrib_location(&self.gravity_well_shader, "a_TexCoord");

        let stride = (std::mem::size_of::<f32>() * 2) + (std::mem::size_of::<f32>() * 2);
        self.context.vertex_attrib_pointer_with_i32(
            position_attrib_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            stride as i32,
            0,
        );
        self.context.vertex_attrib_pointer_with_i32(
            tex_coord_attrib_location as u32,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            stride as i32,
            (std::mem::size_of::<f32>() * 2) as i32,
        );
        self.context
            .enable_vertex_attrib_array(position_attrib_location as u32);
        self.context
            .enable_vertex_attrib_array(tex_coord_attrib_location as u32);
        let gravity_well_tex = self
            .textures
            .get("gravity_well")
            .or(self.textures.get("not_found"))
            .expect("failed to load 'not_found' texture");

        self.context.active_texture(WebGlRenderingContext::TEXTURE0);
        self.context
            .bind_texture(WebGlRenderingContext::TEXTURE_2D, gravity_well_tex.as_ref());
        self.context.uniform1i(Some(&u_sampler_location), 0);

        for gravity_well in gravity_wells {
            let model_mat = glm::rotate_z(
                &glm::translation(&glm::vec3(
                    gravity_well.pos[0] as f32,
                    gravity_well.pos[1] as f32,
                    0.0,
                )),
                gravity_well.rotation_deg as f32 * 0.01745329252,
            );
            self.context.uniform_matrix4fv_with_f32_array(
                Some(&u_model_location),
                false,
                model_mat.as_slice(),
            );
            // self.context.uniform1i(
            //     Some(&u_is_selected_location),
            //     gravity_well.is_selected as i32,
            // );
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
    const MAX_VELOCITY: f64 = 2000.0;
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
    rotation_deg: f64,
    mass: f64,
    is_selected: bool,
}

impl GravityWell {
    const RADIUS: u32 = 20;
    const ROTATION_SPEED: f64 = 2.0;

    fn new(pos: [f64; 2], mass: f64) -> Self {
        GravityWell {
            pos,
            rotation_deg: 0.0,
            mass,
            is_selected: false,
        }
    }

    fn is_point_inside(&self, x: i32, y: i32) -> bool {
        // let (left, top, right, bottom) = self.get_rect();
        // let x = x as f64;
        // let y = y as f64;
        // x >= left && y >= top && x <= right && y <= bottom
        let delta_x = (x as f64 - self.pos[0]).abs();
        let delta_y = (y as f64 - self.pos[1]).abs();
        let distance_from_well = glm::length(&glm::vec2(delta_x, delta_y));
        if distance_from_well <= GravityWell::RADIUS as f64 {
            true
        } else {
            false
        }
    }

    fn get_rect(&self) -> (f64, f64, f64, f64) {
        let left = self.pos[0] - (GravityWell::RADIUS as f64);
        let top = self.pos[1] - (GravityWell::RADIUS as f64);
        let right = self.pos[0] + (GravityWell::RADIUS as f64);
        let bottom = self.pos[1] + (GravityWell::RADIUS as f64);
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

fn is_power_of_2(n: u32) -> bool {
    if (n & (n - 1)) == 0 {
        true
    } else {
        false
    }
}
