use crate::gl::{Buffer, Colour, Context, Program, Shader, VertexArrayObject};
use wasm_bindgen::{prelude::*, JsCast};

#[wasm_bindgen]
pub struct Triangle {
    ctx: Context,
    vert_count: u32,
    bg_colour: Colour,
}

#[wasm_bindgen]
impl Triangle {
    pub fn new(
        canvas: Option<web_sys::Element>,
        fg_colour: Colour,
        bg_colour: Colour,
    ) -> Result<Triangle, JsValue> {
        let canvas = match canvas {
            Some(element) => element.dyn_into::<web_sys::HtmlCanvasElement>()?,
            None => return Err("Canvas element does not exist".into()),
        };

        let ctx = Context::new(&canvas)?;

        let vert_shader = Shader::new_vert(&ctx, include_str!("vert.glsl"))?;
        let frag_shader = Shader::new_frag(&ctx, include_str!("frag.glsl"))?;
        let program = Program::new(&ctx, &[&vert_shader, &frag_shader], None)?;

        ctx.use_program(&program);

        let vertices: [f32; 9] = [-0.7, -0.7, 0.0, 0.7, -0.7, 0.0, 0.0, 0.7, 0.0];

        let position_attribute_location = ctx.get_attrib_location(&program, "position");

        let fg_colour_uniform_location = ctx.get_uniform_location(&program, "fg_colour")?;
        ctx.set_uniform_colour(&fg_colour_uniform_location, &fg_colour);

        let buffer = Buffer::new(&ctx)?;
        ctx.0.bind_buffer(
            web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
            Some(&buffer.0),
        );

        // Note that `Float32Array::view` is somewhat dangerous (hence the
        // `unsafe`!). This is creating a raw view into our module's
        // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
        // (aka do a memory allocation in Rust) it'll cause the buffer to change,
        // causing the `Float32Array` to be invalid.
        //
        // As a result, after `Float32Array::view` we have to be very careful not to
        // do any memory allocations before it's dropped.
        {
            let src_data = unsafe { js_sys::Float32Array::view(&vertices) };

            ctx.buffer_data_with_array_buffer_view(
                web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
                &src_data,
                web_sys::WebGl2RenderingContext::STATIC_DRAW,
            );
        }

        let vao = VertexArrayObject::new(&ctx)?;
        ctx.bind_vertex_array(&vao);

        ctx.vertex_attrib_pointer_with_i32(
            0,
            3,
            web_sys::WebGl2RenderingContext::FLOAT,
            false,
            0,
            0,
        );
        ctx.enable_vertex_attrib_array(position_attribute_location);

        ctx.bind_vertex_array(&vao);

        let vert_count = (vertices.len() / 3) as u32;

        Ok(Triangle {
            ctx,
            vert_count,
            bg_colour,
        })
    }

    pub fn draw(&mut self) {
        self.ctx.clear_colour_buffer(self.bg_colour);
        self.ctx.draw_triangles(self.vert_count);
    }
}
