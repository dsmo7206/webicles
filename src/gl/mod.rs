use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader, WebGlUniformLocation};

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Colour {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

#[wasm_bindgen]
impl Colour {
    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

pub struct AttribInfo {
    pub location: i32,
    pub num_components: usize,
    pub type_: u32,
    pub normalised: bool,
}

pub struct Context(pub web_sys::WebGl2RenderingContext);

impl Context {
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> Result<Self, JsValue> {
        Ok(Self(
            canvas
                .get_context("webgl2")?
                .unwrap()
                .dyn_into::<web_sys::WebGl2RenderingContext>()?,
        ))
    }

    pub fn get_attrib_location(&self, program: &Program, name: &str) -> i32 {
        self.0.get_attrib_location(&program.0, name)
    }

    pub fn get_uniform_location(
        &self,
        program: &Program,
        name: &str,
    ) -> Result<web_sys::WebGlUniformLocation, JsValue> {
        match self.0.get_uniform_location(&program.0, name) {
            Some(location) => Ok(location),
            None => Err("Uniform location not found".into()),
        }
    }

    pub fn set_uniform_colour(&self, location: &WebGlUniformLocation, colour: &Colour) {
        self.0.uniform4f(
            Some(location),
            colour.red,
            colour.green,
            colour.blue,
            colour.alpha,
        );
    }

    // pub fn bind_buffer(&self, target: u32, buffer: &Buffer) {
    //     self.0.bind_buffer(target, Some(&buffer.0))
    // }

    // pub fn clear_buffer(&self, target: u32) {
    //     self.0.bind_buffer(target, None)
    // }

    // pub fn bind_texture(&self, target: u32, texture: &Texture) {
    //     self.0.bind_texture(target, Some(&texture.0));
    // }

    pub fn tex_image_2d(&self, width: usize, height: usize, pixels: &[u8]) -> Result<(), JsValue> {
        self.0
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                web_sys::WebGl2RenderingContext::TEXTURE_2D,
                0,
                web_sys::WebGl2RenderingContext::RG8 as i32,
                width as i32,
                height as i32,
                0,
                web_sys::WebGl2RenderingContext::RG,
                web_sys::WebGl2RenderingContext::UNSIGNED_BYTE,
                Some(pixels),
            )
    }

    pub fn buffer_data_with_array_buffer_view(
        &self,
        target: u32,
        src_data: &js_sys::Object,
        usage: u32,
    ) {
        self.0
            .buffer_data_with_array_buffer_view(target, src_data, usage);
    }

    pub fn bind_vertex_array(&self, vao: &VertexArrayObject) {
        self.0.bind_vertex_array(Some(&vao.0));
    }

    pub fn clear_vertex_array(&self) {
        self.0.bind_vertex_array(None);
    }

    pub fn vertex_attrib_pointer_with_i32(
        &self,
        indx: u32,
        size: i32,
        type_: u32,
        normalised: bool,
        stride: i32,
        offset: i32,
    ) {
        self.0
            .vertex_attrib_pointer_with_i32(indx, size, type_, normalised, stride, offset);
    }

    pub fn enable_vertex_attrib_array(&self, index: i32) {
        self.0.enable_vertex_attrib_array(index as u32)
    }

    pub fn use_program(&self, program: &Program) {
        self.0.use_program(Some(&program.0));
    }

    pub fn clear_colour_buffer(&self, colour: Colour) {
        self.clear_colour(colour);
        self.0
            .clear(web_sys::WebGl2RenderingContext::COLOR_BUFFER_BIT);
    }

    pub fn clear_colour(&self, colour: Colour) {
        self.0
            .clear_color(colour.red, colour.green, colour.blue, colour.alpha);
    }

    pub fn draw_triangles(&self, count: u32) {
        self.0
            .draw_arrays(web_sys::WebGl2RenderingContext::TRIANGLES, 0, count as i32);
    }
}

pub struct Shader(WebGlShader);

impl Shader {
    pub fn new_vert(ctx: &Context, source: &str) -> Result<Self, String> {
        Self::new(ctx, web_sys::WebGl2RenderingContext::VERTEX_SHADER, source)
    }

    pub fn new_frag(ctx: &Context, source: &str) -> Result<Self, String> {
        Self::new(
            ctx,
            web_sys::WebGl2RenderingContext::FRAGMENT_SHADER,
            source,
        )
    }

    fn new(ctx: &Context, shader_type: u32, source: &str) -> Result<Self, String> {
        let shader = ctx
            .0
            .create_shader(shader_type)
            .ok_or_else(|| String::from("Unable to create shader object"))?;

        ctx.0.shader_source(&shader, source);
        ctx.0.compile_shader(&shader);

        if ctx
            .0
            .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(Shader(shader))
        } else {
            Err(ctx
                .0
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| String::from("Unknown error creating shader")))
        }
    }
}

pub struct TransformFeedbackVaryings<'a> {
    pub names: &'a [&'a str],
    pub buffer_mode: u32,
}

pub struct Program(WebGlProgram);

impl Program {
    pub fn new(
        ctx: &Context,
        shaders: &[&Shader],
        transform_feedback_varyings: Option<TransformFeedbackVaryings>,
    ) -> Result<Self, String> {
        let program = ctx
            .0
            .create_program()
            .ok_or_else(|| String::from("Unable to create shader object"))?;

        shaders
            .into_iter()
            .for_each(|shader| ctx.0.attach_shader(&program, &shader.0));

        if let Some(varyings) = transform_feedback_varyings {
            let names = varyings
                .names
                .iter()
                .map(|&name| JsValue::from(name))
                .collect::<js_sys::Array>();

            ctx.0
                .transform_feedback_varyings(&program, &names.into(), varyings.buffer_mode)
        }

        ctx.0.link_program(&program);

        if ctx
            .0
            .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(Program(program))
        } else {
            Err(ctx
                .0
                .get_program_info_log(&program)
                .unwrap_or_else(|| String::from("Unknown error creating program object")))
        }
    }
}

pub struct Buffer(pub web_sys::WebGlBuffer);

impl Buffer {
    pub fn new(ctx: &Context) -> Result<Self, JsValue> {
        match ctx.0.create_buffer() {
            Some(buffer) => Ok(Self(buffer)),
            None => Err("Failed to create buffer".into()),
        }
    }
}

pub struct VertexArrayObject(web_sys::WebGlVertexArrayObject);

impl VertexArrayObject {
    pub fn new(ctx: &Context) -> Result<Self, JsValue> {
        match ctx.0.create_vertex_array() {
            Some(vao) => Ok(VertexArrayObject(vao)),
            None => Err("Could not create vertex array object".into()),
        }
    }
}

pub struct Texture(pub web_sys::WebGlTexture);

impl Texture {
    pub fn new(ctx: &Context) -> Result<Self, JsValue> {
        match ctx.0.create_texture() {
            Some(texture) => Ok(Texture(texture)),
            None => Err("Could not create texture".into()),
        }
    }
}

pub struct BufferInfo<'a> {
    pub obj: &'a Buffer,
    pub stride: usize,
    pub attribs: &'a [&'a AttribInfo],
}

pub fn setup_array_buffer_vao(ctx: &Context, vao: &VertexArrayObject, buffer_info: &BufferInfo) {
    ctx.bind_vertex_array(vao);

    ctx.0.bind_buffer(
        web_sys::WebGl2RenderingContext::ARRAY_BUFFER,
        Some(&buffer_info.obj.0),
    );

    let mut offset = 0;

    buffer_info.attribs.iter().for_each(|&attrib| {
        ctx.enable_vertex_attrib_array(attrib.location);
        ctx.vertex_attrib_pointer_with_i32(
            attrib.location as u32,
            attrib.num_components as i32,
            attrib.type_,
            attrib.normalised,
            buffer_info.stride as i32,
            offset,
        );

        /* Note that we're cheating a little bit here: if the buffer has some irrelevant data
            between the attributes that we're interested in, calculating the offset this way
            would not work. However, in this demo, buffers are laid out in such a way that this code works :) */
        offset += attrib.num_components as i32 * match attrib.type_ {
            web_sys::WebGl2RenderingContext::FLOAT => 4,
            web_sys::WebGl2RenderingContext::UNSIGNED_BYTE => 1,
            _ => unimplemented!()
        };

        // TODO: ADD DIVISOR
        /*
        if (attrib_desc.hasOwnProperty("divisor")) { /* we'll need this later */
          gl.vertexAttribDivisor(attrib_desc.location, attrib_desc.divisor);
        }
         */
    });

    ctx.clear_vertex_array();
    ctx.0
        .bind_buffer(web_sys::WebGl2RenderingContext::ARRAY_BUFFER, None);
}
