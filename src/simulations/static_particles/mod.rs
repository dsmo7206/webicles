use crate::gl::{
    setup_array_buffer_vao, AttribInfo, Buffer, BufferInfo, Colour, Context, Program, Shader,
    Texture, TransformFeedbackVaryings, VertexArrayObject,
};
use rand::distributions::{Distribution, Uniform};
use wasm_bindgen::{prelude::*, JsCast};

#[wasm_bindgen]
pub struct StaticParticles {
    ctx: Context,
    read_index: usize,
    write_index: usize,
    num_particles: usize,
    born_particles: usize,
    particle_birth_rate: usize, // Num per second
    update_program: UpdateProgram,
    draw_program: DrawProgram,
    rg_noise_texture: Texture,
    total_time: f32,
    gravity: (f32, f32),
    buffers: [Buffer; 2],
    vaos: [VertexArrayObject; 4],
    stats: Stats,
}

#[wasm_bindgen]
impl StaticParticles {
    pub fn new(
        canvas: Option<web_sys::Element>,
        num_particles: usize,
        particle_birth_rate: usize,
        gravity_x: f32,
        gravity_y: f32,
    ) -> Result<StaticParticles, JsValue> {
        let canvas = match canvas {
            Some(element) => element.dyn_into::<web_sys::HtmlCanvasElement>()?,
            None => return Err("Canvas element does not exist".into()),
        };

        let ctx = Context::new(&canvas)?;

        let update_program = UpdateProgram::new(&ctx)?;
        let draw_program = DrawProgram::new(&ctx)?;

        let buffers = [Buffer::new(&ctx)?, Buffer::new(&ctx)?];
        let vaos = [
            VertexArrayObject::new(&ctx)?,
            VertexArrayObject::new(&ctx)?,
            VertexArrayObject::new(&ctx)?,
            VertexArrayObject::new(&ctx)?,
        ];

        let data = initial_particle_data(num_particles, 1.0, 2.0);
        {
            let src_data = unsafe { js_sys::Float32Array::view(&data) };

            ctx.0.bind_buffer(
                web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
                Some(&buffers[0].0),
            );
            ctx.buffer_data_with_array_buffer_view(
                web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
                &src_data,
                web_sys::WebGl2RenderingContext::STREAM_DRAW,
            );

            ctx.0.bind_buffer(
                web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
                Some(&buffers[1].0),
            );
            ctx.buffer_data_with_array_buffer_view(
                web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
                &src_data,
                web_sys::WebGl2RenderingContext::STREAM_DRAW,
            );
        }

        let update_program_attribs = [
            &update_program.attrib_info_position,
            &update_program.attrib_info_age,
            &update_program.attrib_info_life,
            &update_program.attrib_info_velocity,
        ];

        let render_program_attribs = [
            &draw_program.attrib_info_position,
            &draw_program.attrib_info_age,
            &draw_program.attrib_info_life,
        ];

        setup_array_buffer_vao(
            &ctx,
            &vaos[0],
            &BufferInfo {
                obj: &buffers[0],
                stride: 4 * 6,
                attribs: &update_program_attribs,
            },
        );

        setup_array_buffer_vao(
            &ctx,
            &vaos[1],
            &BufferInfo {
                obj: &buffers[1],
                stride: 4 * 6,
                attribs: &update_program_attribs,
            },
        );

        setup_array_buffer_vao(
            &ctx,
            &vaos[2],
            &BufferInfo {
                obj: &buffers[0],
                stride: 4 * 6,
                attribs: &render_program_attribs,
            },
        );

        setup_array_buffer_vao(
            &ctx,
            &vaos[3],
            &BufferInfo {
                obj: &buffers[1],
                stride: 4 * 6,
                attribs: &render_program_attribs,
            },
        );

        ctx.clear_colour(Colour {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 1.0,
        });

        // Create a texture for random values
        let rg_noise_texture = Texture::new(&ctx)?;
        ctx.0.bind_texture(
            web_sys::WebGl2RenderingContext::TEXTURE_2D,
            Some(&rg_noise_texture.0),
        );

        ctx.tex_image_2d(512, 512, &random_rg_data(512, 512))?;
        ctx.0.tex_parameteri(
            web_sys::WebGl2RenderingContext::TEXTURE_2D,
            web_sys::WebGl2RenderingContext::TEXTURE_WRAP_S,
            web_sys::WebGl2RenderingContext::MIRRORED_REPEAT as i32,
        );
        ctx.0.tex_parameteri(
            web_sys::WebGl2RenderingContext::TEXTURE_2D,
            web_sys::WebGl2RenderingContext::TEXTURE_WRAP_T,
            web_sys::WebGl2RenderingContext::MIRRORED_REPEAT as i32,
        );
        ctx.0.tex_parameteri(
            web_sys::WebGl2RenderingContext::TEXTURE_2D,
            web_sys::WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            web_sys::WebGl2RenderingContext::NEAREST as i32,
        );
        ctx.0.tex_parameteri(
            web_sys::WebGl2RenderingContext::TEXTURE_2D,
            web_sys::WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            web_sys::WebGl2RenderingContext::NEAREST as i32,
        );

        /* Set up blending */
        ctx.0.enable(web_sys::WebGl2RenderingContext::BLEND);
        ctx.0.blend_func(
            web_sys::WebGl2RenderingContext::SRC_ALPHA,
            web_sys::WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
        );

        Ok(StaticParticles {
            ctx,
            read_index: 0,
            write_index: 1,
            born_particles: 0,
            num_particles: data.len() / 6,
            particle_birth_rate,
            update_program,
            draw_program,
            rg_noise_texture,
            total_time: 0.0,
            gravity: (gravity_x, gravity_y),
            buffers,
            vaos,
            stats: Stats { x: 123 },
        })
    }

    pub fn draw(&mut self, mut dt: f32) -> Result<(), JsValue> {
        let num_particles_to_draw = self.born_particles;

        if dt > 0.5 {
            dt = 0.0; // This is in seconds. If too large, tab might be in background
        }

        if self.born_particles < self.num_particles {
            self.born_particles = self.num_particles.min(
                (self.born_particles as f32 + self.particle_birth_rate as f32 * dt).floor()
                    as usize,
            );
        }

        self.ctx.0.clear(
            web_sys::WebGl2RenderingContext::COLOR_BUFFER_BIT
                | web_sys::WebGl2RenderingContext::DEPTH_BUFFER_BIT,
        );
        self.ctx.use_program(&self.update_program.program);

        self.ctx.0.uniform1f(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_TimeDelta")?,
            ),
            dt,
        );
        // self.ctx.0.uniform1f(
        //     Some(
        //         &self
        //             .ctx
        //             .get_uniform_location(&self.update_program.program, "u_TotalTime")?,
        //     ),
        //     self.total_time,
        // );
        self.ctx.0.uniform2f(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_Gravity")?,
            ),
            self.gravity.0,
            self.gravity.1,
        );
        self.ctx.0.uniform2f(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_Origin")?,
            ),
            0.0,
            0.0,
        );
        self.ctx.0.uniform1f(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_MinTheta")?,
            ),
            -std::f32::consts::PI,
        );
        self.ctx.0.uniform1f(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_MaxTheta")?,
            ),
            std::f32::consts::PI,
        );
        self.ctx.0.uniform1f(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_MinSpeed")?,
            ),
            0.0,
        );
        self.ctx.0.uniform1f(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_MaxSpeed")?,
            ),
            1.0,
        );

        self.ctx
            .0
            .active_texture(web_sys::WebGl2RenderingContext::TEXTURE0);
        self.ctx.0.bind_texture(
            web_sys::WebGl2RenderingContext::TEXTURE_2D,
            Some(&self.rg_noise_texture.0),
        );
        self.ctx.0.uniform1i(
            Some(
                &self
                    .ctx
                    .get_uniform_location(&self.update_program.program, "u_RgNoise")?,
            ),
            0,
        );

        self.total_time += dt;

        /* Bind the "read" buffer - it contains the state of the particle system
        "as of now".*/
        self.ctx.bind_vertex_array(&self.vaos[self.read_index]);

        /* Bind the "write" buffer as transform feedback - the varyings of the
        update shader will be written here. */
        self.ctx.0.bind_buffer_base(
            web_sys::WebGl2RenderingContext::TRANSFORM_FEEDBACK_BUFFER,
            0,
            Some(&self.buffers[self.write_index].0),
        );

        /* Since we're not actually rendering anything when updating the particle
        state, disable rasterization.*/
        self.ctx
            .0
            .enable(web_sys::WebGl2RenderingContext::RASTERIZER_DISCARD);

        /* Begin transform feedback! */
        self.ctx
            .0
            .begin_transform_feedback(web_sys::WebGl2RenderingContext::POINTS);
        self.ctx.0.draw_arrays(
            web_sys::WebGl2RenderingContext::POINTS,
            0,
            num_particles_to_draw as i32,
        );
        self.ctx.0.end_transform_feedback();
        self.ctx
            .0
            .disable(web_sys::WebGl2RenderingContext::RASTERIZER_DISCARD);
        /* Don't forget to unbind the transform feedback buffer! */
        self.ctx.0.bind_buffer_base(
            web_sys::WebGl2RenderingContext::TRANSFORM_FEEDBACK_BUFFER,
            0,
            None,
        );

        /* Now, we draw the particle system. Note that we're actually
        drawing the data from the "read" buffer, not the "write" buffer
        that we've written the updated data to. */
        self.ctx.bind_vertex_array(&self.vaos[self.read_index + 2]);
        self.ctx.use_program(&self.draw_program.program);
        self.ctx.0.draw_arrays(
            web_sys::WebGl2RenderingContext::POINTS,
            0,
            num_particles_to_draw as i32,
        );

        /* Finally, we swap read and write buffers. The updated state will be
        rendered on the next frame. */
        std::mem::swap(&mut self.read_index, &mut self.write_index);

        Ok(())
    }

    pub fn get_stats(&self) -> JsValue {
        serde_json::to_string(&self.stats).unwrap().into()
    }
}

#[derive(serde::Serialize)]
struct Stats {
    x: usize,
}

pub struct UpdateProgram {
    program: Program,
    attrib_info_position: AttribInfo,
    attrib_info_age: AttribInfo,
    attrib_info_life: AttribInfo,
    attrib_info_velocity: AttribInfo,
}

impl UpdateProgram {
    fn new(ctx: &Context) -> Result<Self, JsValue> {
        let program = {
            let vert_shader = Shader::new_vert(&ctx, include_str!("update_vert.glsl"))?;
            let frag_shader = Shader::new_frag(&ctx, include_str!("update_frag.glsl"))?;

            Program::new(
                &ctx,
                &[&vert_shader, &frag_shader],
                Some(TransformFeedbackVaryings {
                    names: &["v_Position", "v_Age", "v_Life", "v_Velocity"],
                    buffer_mode: web_sys::WebGl2RenderingContext::INTERLEAVED_ATTRIBS,
                }),
            )?
        };

        let attrib_info_position = AttribInfo {
            location: ctx.get_attrib_location(&program, "i_Position"),
            num_components: 2,
            type_: web_sys::WebGl2RenderingContext::FLOAT,
            normalised: false,
        };
        let attrib_info_age = AttribInfo {
            location: ctx.get_attrib_location(&program, "i_Age"),
            num_components: 1,
            type_: web_sys::WebGl2RenderingContext::FLOAT,
            normalised: false,
        };
        let attrib_info_life = AttribInfo {
            location: ctx.get_attrib_location(&program, "i_Life"),
            num_components: 1,
            type_: web_sys::WebGl2RenderingContext::FLOAT,
            normalised: false,
        };
        let attrib_info_velocity = AttribInfo {
            location: ctx.get_attrib_location(&program, "i_Velocity"),
            num_components: 2,
            type_: web_sys::WebGl2RenderingContext::FLOAT,
            normalised: false,
        };

        Ok(Self {
            program,
            attrib_info_position,
            attrib_info_age,
            attrib_info_life,
            attrib_info_velocity,
        })
    }
}

struct DrawProgram {
    program: Program,
    attrib_info_position: AttribInfo,
    attrib_info_age: AttribInfo,
    attrib_info_life: AttribInfo,
}

impl DrawProgram {
    fn new(ctx: &Context) -> Result<Self, JsValue> {
        let program = {
            let vert_shader = Shader::new_vert(&ctx, include_str!("draw_vert.glsl"))?;
            let frag_shader = Shader::new_frag(&ctx, include_str!("draw_frag.glsl"))?;

            Program::new(&ctx, &[&vert_shader, &frag_shader], None)?
        };

        let attrib_info_position = AttribInfo {
            location: ctx.get_attrib_location(&program, "i_Position"),
            num_components: 2,
            type_: web_sys::WebGl2RenderingContext::FLOAT,
            normalised: false,
        };

        let attrib_info_age = AttribInfo {
            location: ctx.get_attrib_location(&program, "i_Age"),
            num_components: 1,
            type_: web_sys::WebGl2RenderingContext::FLOAT,
            normalised: false,
        };

        let attrib_info_life = AttribInfo {
            location: ctx.get_attrib_location(&program, "i_Life"),
            num_components: 1,
            type_: web_sys::WebGl2RenderingContext::FLOAT,
            normalised: false,
        };

        Ok(Self {
            program,
            attrib_info_position,
            attrib_info_age,
            attrib_info_life,
        })
    }
}

fn initial_particle_data(num_parts: usize, min_age: f32, max_age: f32) -> Vec<f32> {
    let mut data = vec![];

    let mut rng = rand::thread_rng();
    let die = Uniform::from(min_age..=max_age);

    (0..num_parts).for_each(|_| {
        // Position
        data.push(0.0);
        data.push(0.0);

        let life = die.sample(&mut rng) as f32;
        data.push(life + 1.0);
        data.push(life);

        // Velocity
        data.push(0.0);
        data.push(0.0);
    });

    data
}

fn random_rg_data(size_x: usize, size_y: usize) -> Vec<u8> {
    let mut data = vec![];

    let mut rng = rand::thread_rng();
    let die = Uniform::from(0u8..=255);

    (0..(size_x * size_y)).for_each(|_| {
        data.push(die.sample(&mut rng));
        data.push(die.sample(&mut rng));
    });

    data
}

fn fixed_rg_data(size_x: usize, size_y: usize) -> Vec<u8> {
    let mut data = vec![];

    (0..(size_x * size_y)).for_each(|_| {
        data.push(0);
        data.push(0);
    });

    data
}
