// Renderer struct that handles WebGl calls, and contains data for rendering,
// including textures, matrices for projecting into normalized screen coordinates,
// and shaders.

use wasm_bindgen::JsCast;
use crate::webgl_helpers;
use crate::particle::Particle;
use crate::gravity_well::GravityWell;
use web_sys::{ console, WebGlRenderingContext, WebGlBuffer, WebGlProgram, WebGlTexture };
use std::collections::{ HashMap, VecDeque };
extern crate nalgebra_glm as glm;
use glm::TMat4;

pub struct Renderer {
    pub context: WebGlRenderingContext,
    pub textures: HashMap<String, Option<WebGlTexture>>,
    pub projection_mat: TMat4<f32>,
    pub particle_vertex_buffer: WebGlBuffer,
    pub particle_color_buffer: WebGlBuffer,
    pub gravity_well_vbo: WebGlBuffer,
    pub particle_shader: WebGlProgram,
    pub gravity_well_shader: WebGlProgram,
    pub particle_vertex_array: Vec<f32>,
    pub particle_color_array: Vec<u8>,
}

impl Renderer {
    // On creation grabs reference to WebGl context from canvas on the DOM
    // Tries to compile shaders and link them into shader programs
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> Self {
        let context = canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .unwrap();

        let projection_mat = glm::ortho(0.0, canvas.width() as f32, canvas.height() as f32, 0.0, 1.0, -1.0);

        let particle_vertex_shader = webgl_helpers::compile_shader(
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
        let particle_fragment_shader = webgl_helpers::compile_shader(
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
            webgl_helpers::link_program(&context, &particle_vertex_shader, &particle_fragment_shader).unwrap();
        let gravity_well_vertex_shader = webgl_helpers::compile_shader(
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
        let gravity_well_fragment_shader = webgl_helpers::compile_shader(
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
        let gravity_well_shader = webgl_helpers::link_program(
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

        // Hashmap for storing named textures, creates a texture of one blue pixel
        // to use as a default when a requested texture isn't found in the hashmap
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

    pub fn clear_screen(&self) {
        self.context.clear_color(0.0, 0.0, 0.0, 1.0);
        self.context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
    }

    pub fn render_particles(&mut self, particles: &VecDeque<Particle>, trail_scale: f64) {
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

    pub fn render_gravity_wells(&self, gravity_wells: &Vec<GravityWell>) {
        // Coordinates are x, y, u, t
        let vertex_array = vec![
            // Triangle 1:
            // top right
            GravityWell::RADIUS as f32,
            -(GravityWell::RADIUS as f32),
            1.0,
            0.0,
            // top left
            -(GravityWell::RADIUS as f32),
            -(GravityWell::RADIUS as f32),
            0.0,
            0.0,
            // bottom left
            -(GravityWell::RADIUS as f32),
            GravityWell::RADIUS as f32,
            0.0,
            1.0,

            // Triangle 2:
            // top right
            GravityWell::RADIUS as f32,
            -(GravityWell::RADIUS as f32),
            1.0,
            0.0,
            // bottom left
            -(GravityWell::RADIUS as f32),
            GravityWell::RADIUS as f32,
            0.0,
            1.0,
            // bottom right
            GravityWell::RADIUS as f32,
            GravityWell::RADIUS as f32,
            1.0,
            1.0,
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
            .get_uniform_location(&self.gravity_well_shader, "u_Proj")
            .expect("failed to get u_Proj uniform location");
        self.context.uniform_matrix4fv_with_f32_array(
            Some(&u_proj_location),
            false,
            self.projection_mat.as_slice(),
        );

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

    // fn initialize_shaders(&mut self) {
    //     let particle_vertex_shader = webgl_helpers::compile_shader(
    //         &context,
    //         WebGlRenderingContext::VERTEX_SHADER,
    //         r#"
    //         attribute vec2 a_Position;
    //         attribute vec4 a_Color;

    //         // uniform float u_TrailScale;
    //         uniform mat4 u_Proj;

    //         varying vec4 v_Color;

    //         void main() {
    //             gl_Position = u_Proj * vec4(a_Position, 0.0, 1.0);
    //             v_Color = a_Color;
    //         }
    //     "#,
    //     )
    //     .unwrap();
    //     let particle_fragment_shader = webgl_helpers::compile_shader(
    //         &context,
    //         WebGlRenderingContext::FRAGMENT_SHADER,
    //         r#"
            
    //         precision mediump float;
    //         varying vec4 v_Color;

    //         void main() {
    //             gl_FragColor = v_Color;
    //         }
    //     "#,
    //     )
    //     .unwrap();
    //     let particle_shader =
    //         link_program(&context, &particle_vertex_shader, &particle_fragment_shader).unwrap();
    //     let gravity_well_vertex_shader = webgl_helpers::compile_shader(
    //         &context,
    //         WebGlRenderingContext::VERTEX_SHADER,
    //         r#"
    //         attribute vec2 a_Position;
    //         attribute vec2 a_TexCoord;
            
    //         uniform bool u_IsSelected;
    //         uniform mat4 u_Model;
    //         uniform mat4 u_Proj;

    //         varying mediump vec2 v_TexCoord;
            
    //         void main() {
    //             gl_Position = u_Proj * u_Model * vec4(a_Position, 0.0, 1.0);
    //             v_TexCoord = a_TexCoord;
    //             // if(u_IsSelected) {
    //             //     v_Color = vec4(0.3, 0.5, 1.0, 1.0);
    //             // } else {
    //             //     v_Color = vec4(0.5, 0.5, 0.5, 1.0);
    //             // }
    //         }
    //         "#,
    //     )
    //     .unwrap();
    //     let gravity_well_fragment_shader = webgl_helpers::compile_shader(
    //         &context,
    //         WebGlRenderingContext::FRAGMENT_SHADER,
    //         r#"
    //         // precision mediump float;

    //         varying mediump vec2 v_TexCoord;

    //         uniform sampler2D u_Sampler;

    //         void main() {
    //             gl_FragColor = texture2D(u_Sampler, v_TexCoord);
    //         }
    //     "#,
    //     )
    //     .unwrap();
    //     let gravity_well_shader = link_program(
    //         &context,
    //         &gravity_well_vertex_shader,
    //         &gravity_well_fragment_shader,
    //     )
    //     .unwrap();
    // }
}

