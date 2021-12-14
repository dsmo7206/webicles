use crate::gl::{
    setup_array_buffer_vao, AttribInfo, Buffer, BufferInfo, Colour, Context, Program, Shader,
    VertexArrayObject,
};
use crate::linalg::{mat_add_scalar, polar_decomp, square_vec, svd, Mat2, Vec2};
use rand::distributions::{Distribution, Uniform};
use wasm_bindgen::{prelude::*, JsCast};

// Snow material properties
const PARTICLE_MASS: f32 = 1.0;
const VOL: f32 = 1.0; // Particle Volume
const HARDENING: f32 = 10.0; // Snow hardening factor
const E: f32 = 10000.0; // Young's Modulus
const NU: f32 = 0.2; // Poisson ratio
const PLASTIC: bool = true;

// Initial Lamé parameters
const MU_0: f32 = E / (2.0 * (1.0 + NU));
const LAMBDA_0: f32 = E * NU / ((1.0 + NU) * (1.0 - 2.0 * NU));

macro_rules! console {
    // ($($arg:tt)*) => {{
    //     let res = std::fmt::format(format_args!($($arg)*));
    //     web_sys::console::log_1(&res.into());
    // }}
    ($($arg:tt)*) => {{}};
}

struct Particle {
    position: Vec2,
    velocity: Vec2,
    deformation_gradient: Mat2,
    apic_affine_momentum: Mat2,
    deformation_gradient_det: f32,
    colour: u32,
}

impl Particle {
    fn new(pos: Vec2, colour: u32) -> Self {
        Self {
            position: pos,
            velocity: Vec2::ZERO,
            deformation_gradient: Mat2::IDENTITY,
            apic_affine_momentum: Mat2::ZERO,
            deformation_gradient_det: 1.0,
            colour,
        }
    }
}

#[derive(Clone, Debug)]
struct Cell {
    velocity: Vec2,
    mass: f32,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            velocity: Vec2::ZERO,
            mass: 0.0,
        }
    }
}

#[wasm_bindgen]
pub struct RustMlsMpm {
    ctx: Context,
    draw_program: DrawProgram,
    particles: Vec<Particle>,
    grid_size: usize,
    buffer: Buffer,
    vao: VertexArrayObject,
    frame_number: usize,
}

#[wasm_bindgen]
impl RustMlsMpm {
    pub fn new(
        canvas: Option<web_sys::Element>,
        num_particles: usize, // per oject
        grid_size: usize,
    ) -> Result<RustMlsMpm, JsValue> {
        let mut particles = vec![];
        add_particles(
            &mut particles,
            num_particles,
            Vec2::new(0.55, 0.45),
            0xed553b,
        );
        add_particles(
            &mut particles,
            num_particles,
            Vec2::new(0.45, 0.65),
            0xf2b134,
        );
        add_particles(
            &mut particles,
            num_particles,
            Vec2::new(0.55, 0.85),
            0x068587,
        );
        //particles.push(Particle::new(Vec2::new(0.55, 0.45), 0));

        let canvas = match canvas {
            Some(element) => element.dyn_into::<web_sys::HtmlCanvasElement>()?,
            None => return Err("Canvas element does not exist".into()),
        };

        let ctx = Context::new(&canvas)?;

        let draw_program = DrawProgram::new(&ctx)?;
        let buffer = Buffer::new(&ctx)?;
        let vao = VertexArrayObject::new(&ctx)?;

        setup_array_buffer_vao(
            &ctx,
            &vao,
            &BufferInfo {
                obj: &buffer,
                stride: 4 * 3,
                attribs: &[&draw_program.attrib_info_position],
            },
        );

        Ok(Self {
            ctx,
            draw_program,
            particles,
            grid_size,
            buffer,
            vao,
            frame_number: 0,
        })
    }

    pub fn advance(&mut self, dt: f32) {
        let mut grid = vec![vec![Cell::default(); self.grid_size + 1]; self.grid_size + 1];
        let dx = 1.0 / self.grid_size as f32;
        let inv_dx = self.grid_size as f32;

        // Particles to grid
        for particle in self.particles.iter() {
            // Elementwise floor
            let base_coord = (particle.position * inv_dx - Vec2::splat(0.5))
                .floor()
                .as_ivec2();

            let fx = particle.position * inv_dx - base_coord.as_vec2();

            // Quadratic kernels
            let w = [
                Vec2::splat(0.5) * square_vec(Vec2::splat(1.5) - fx),
                Vec2::splat(0.75) - square_vec(fx - Vec2::ONE),
                Vec2::splat(0.5) * square_vec(fx - Vec2::splat(0.5)),
            ];

            // Lamé parameters
            let e = (HARDENING * (1.0 - particle.deformation_gradient_det)).exp();
            let mu = MU_0 * e;
            let lambda = LAMBDA_0 * e;

            // Current volume
            let J = particle.deformation_gradient.determinant();

            // Polar decomposition for fixed corotated model
            let (r, _) = polar_decomp(particle.deformation_gradient);

            let Dinv = 4.0 * inv_dx * inv_dx;

            let PF = mat_add_scalar(
                2.0 * mu
                    * (particle.deformation_gradient - r)
                    * particle.deformation_gradient.transpose(),
                lambda * (J - 1.0) * J,
            );
            let stress = -(dt * VOL) * (Dinv * PF);
            let affine = stress + (PARTICLE_MASS * particle.apic_affine_momentum);

            // Translational momentum

            for i in 0..3 {
                for j in 0..3 {
                    let dpos = (Vec2::new(i as f32, j as f32) - fx) * dx;

                    let factor = w[i].x * w[j].y;
                    let affine_times_dpos = affine * dpos;

                    let cell = &mut grid[base_coord.x as usize + i][base_coord.y as usize + j];

                    cell.velocity +=
                        (particle.velocity * PARTICLE_MASS + affine_times_dpos) * factor;
                    cell.mass += PARTICLE_MASS * factor;
                }
            }
        }

        // For all grid nodes
        for i in 0..=self.grid_size {
            for j in 0..=self.grid_size {
                let cell = &mut grid[i][j];
                if cell.mass <= 0.0 {
                    continue;
                }

                // Normalise by mass
                cell.velocity /= cell.mass;
                cell.mass = 1.0;

                // Gravity
                cell.velocity += Vec2::new(0.0, -200.0 * dt);

                // Boundary thickness
                let boundary = 0.05;
                let x = i as f32 / self.grid_size as f32;
                let y = j as f32 / self.grid_size as f32;

                // Sticky boundary
                if x < boundary || x > 1.0 - boundary || y > 1.0 - boundary {
                    cell.velocity = Vec2::ZERO;
                    cell.mass = 0.0;
                }
                // Separate boundary
                if y < boundary {
                    cell.velocity.y = cell.velocity.y.max(0.0);
                }
            }
        }

        // Grid to particles
        for particle in self.particles.iter_mut() {
            // Elementwise floor
            let base_coord = (particle.position * inv_dx - Vec2::splat(0.5))
                .floor()
                .as_ivec2();
            let fx = particle.position * inv_dx - base_coord.as_vec2();

            // Quadratic kernels
            let w = [
                Vec2::splat(0.5) * square_vec(Vec2::splat(1.5) - fx),
                Vec2::splat(0.75) - square_vec(fx - Vec2::ONE),
                Vec2::splat(0.5) * square_vec(fx - Vec2::splat(0.5)),
            ];

            particle.apic_affine_momentum = Mat2::ZERO;
            particle.velocity = Vec2::ZERO;

            for i in 0..3 {
                for j in 0..3 {
                    let dpos = Vec2::new(i as f32, j as f32) - fx;
                    let grid_v =
                        grid[base_coord.x as usize + i][base_coord.y as usize + j].velocity;
                    let weight = w[i].x * w[j].y;
                    //console!("weight is {}, grid_v is: {}", weight, grid_v);

                    // Velocity
                    particle.velocity += weight * grid_v;
                    // APIC C
                    let hmm = weight * grid_v;

                    let outer = Mat2::from_cols_array(&[
                        hmm.x * dpos.x,
                        hmm.y * dpos.x,
                        hmm.x * dpos.y,
                        hmm.y * dpos.y,
                    ]);
                    particle.apic_affine_momentum += 4.0 * inv_dx * outer;
                }
            }

            // Advection
            particle.position += dt * particle.velocity;

            // MLS-MPM F-update
            let F = (Mat2::IDENTITY + particle.apic_affine_momentum * dt)
                * particle.deformation_gradient;

            let (svd_u, mut sig, svd_v) = svd(F);

            // Snow plasticity
            if PLASTIC {
                sig.col_mut(0).x = sig.col_mut(0).x.clamp(1.0 - 2.5e-2, 1.0 + 7.5e-3);
                sig.col_mut(1).y = sig.col_mut(1).y.clamp(1.0 - 2.5e-2, 1.0 + 7.5e-3);
            }

            let old_j = F.determinant();
            let F = svd_u * sig * svd_v.transpose();

            particle.deformation_gradient_det =
                (particle.deformation_gradient_det * old_j / F.determinant()).clamp(0.6, 20.0);

            particle.deformation_gradient = F;
        }
    }

    pub fn draw(&mut self, mut dt: f32) -> Result<(), JsValue> {
        self.ctx.clear_colour_buffer(Colour {
            red: 0.9,
            green: 0.8,
            blue: 0.8,
            alpha: 1.0,
        });

        self.ctx.use_program(&self.draw_program.program);

        self.advance(dt);

        let mut data = vec![];
        for p in self.particles.iter() {
            data.push(p.position.x);
            data.push(p.position.y);
            data.push(unsafe { std::mem::transmute::<u32, f32>(p.colour) });
        }

        upload_array_buffer(&self.ctx, &data, &self.buffer);

        // /* Now, we draw the particle system. Note that we're actually
        // drawing the data from the "read" buffer, not the "write" buffer
        // that we've written the updated data to. */
        self.ctx.bind_vertex_array(&self.vao);
        self.ctx.0.draw_arrays(
            web_sys::WebGl2RenderingContext::POINTS,
            0,
            self.particles.len() as i32,
        );

        self.frame_number += 1;

        Ok(())
    }
}

fn upload_array_buffer(ctx: &Context, data: &[f32], buffer: &Buffer) {
    let src_data = unsafe { js_sys::Float32Array::view(data) };
    ctx.0.bind_buffer(
        web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
        Some(&buffer.0),
    );
    ctx.buffer_data_with_array_buffer_view(
        web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
        &src_data,
        web_sys::WebGl2RenderingContext::STREAM_DRAW,
    );
}

struct DrawProgram {
    program: Program,
    attrib_info_position: AttribInfo,
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
        };

        Ok(Self {
            program,
            attrib_info_position,
        })
    }
}

fn add_particles(v: &mut Vec<Particle>, num_particles: usize, center: Vec2, c: u32) {
    let mut rng = rand::thread_rng();
    let range = Uniform::from(-1.0..=1.0);

    (0..num_particles).for_each(|_| {
        let pos = Vec2::new(range.sample(&mut rng), range.sample(&mut rng));
        let pos = pos * 0.08 + center;

        v.push(Particle::new(pos, c));
    });
}
