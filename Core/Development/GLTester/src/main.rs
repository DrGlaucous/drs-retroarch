extern crate gl;
extern crate glutin;
extern crate log;

use glutin::dpi::LogicalSize;
use glutin::event_loop::EventLoop;
use glutin::window::WindowBuilder;
use glutin::event::{ElementState, Event, TouchPhase, VirtualKeyCode, WindowEvent};
use glutin::{Api, ContextBuilder, GlProfile, GlRequest, PossiblyCurrent, WindowedContext};
use gl::types::*;


const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;



//"old" opengl 3.3 shaders:
const VERTEX_SHADER_SOURCE: &str = r#"
    #version 330 core
    layout (location = 0) in vec3 aPos;
    void main() {
        gl_Position = vec4(aPos, 1.0);
    }
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
    #version 330 core
    out vec4 FragColor;
    void main() {
        FragColor = vec4(1.0, 0.2, 0.0, 1.0); // Red color
    }
"#;



//"new" opengl 2.1 shaders:
const VERTEX_SHADER_SOURCE_11: &str = r#"
    #version 110 //330 core
    attribute vec3 aPos;
    void main() {
        gl_Position = vec4(aPos, 1.0);
    }
"#;

const FRAGMENT_SHADER_SOURCE_11: &str = r#"
    #version 110
    void main() {
        gl_FragColor = vec4(1.0, 0.2, 0.0, 1.0); // Red color
    }
"#;


fn main() {

    let (mut width, mut height) = (WIDTH, HEIGHT);

    let mut flip = false;

    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("OpenGL Triangle")
        .with_inner_size(LogicalSize::new(width as f64, height as f64));




    let context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGl, (2, 1)))
        //.with_gl_profile(GlProfile::Core)
        //.with_gl_debug_flag(false)
        .with_pixel_format(24, 8)
        //.with_vsync(true)
        .build_windowed(window_builder, &event_loop)
        .unwrap();

    let context = unsafe { context.make_current().unwrap() };


    gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

    let vertices: [f32; 9] = [
        -0.5, -0.5, 0.0,
         0.5, -0.5, 0.0,
         0.0,  0.5, 0.0
    ];

    let mut texture: GLuint = 0;
    unsafe {
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as GLint, width as GLint, height as GLint, 0, gl::RGB, gl::UNSIGNED_BYTE, std::ptr::null());
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, texture, 0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }


    let mut vbo: GLuint = 0;
    let mut vao: GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(gl::ARRAY_BUFFER, (vertices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr, vertices.as_ptr() as *const _, gl::STATIC_DRAW);
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<GLfloat>() as GLsizei, std::ptr::null());
        gl::EnableVertexAttribArray(0);
    }

    let mut vert_shader = 0;
    let mut frag_shader = 0;
    unsafe{
        vert_shader = gl::CreateShader(gl::VERTEX_SHADER);
        frag_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
    }
    let vert_sources = [VERTEX_SHADER_SOURCE_11.as_ptr() as *const GLchar];
    let frag_sources = [FRAGMENT_SHADER_SOURCE_11.as_ptr() as *const GLchar];
    let vert_sources_len = [VERTEX_SHADER_SOURCE_11.len() as GLint - 1];
    let frag_sources_len = [FRAGMENT_SHADER_SOURCE_11.len() as GLint - 1];

    let mut program_id: GLuint = 0;
    unsafe{
        gl::ShaderSource(vert_shader, 1, vert_sources.as_ptr(), vert_sources_len.as_ptr());
        gl::ShaderSource(frag_shader, 1, frag_sources.as_ptr(), frag_sources_len.as_ptr());

        gl::CompileShader(vert_shader);
        gl::CompileShader(frag_shader);

        // //check status
        // {
        //     let mut max_length: GLint = 0;
        //     let mut msg_length: GLsizei = 0;
        //     gl::GetShaderiv(frag_shader, gl::INFO_LOG_LENGTH, (&mut max_length) as *mut _);
        //     let mut data: Vec<u8> = vec![0; max_length as usize];
        //     gl::GetShaderInfoLog(
        //         frag_shader,
        //         max_length as GLsizei,
        //         (&mut msg_length) as *mut _,
        //         data.as_mut_ptr() as *mut _,
        //     );
        //     let data = String::from_utf8_lossy(&data);
        //     log::error!("Failed to compile shader {}: {}", frag_shader, data);
        // }


        program_id = gl::CreateProgram();
        gl::AttachShader(program_id, vert_shader);
        gl::AttachShader(program_id, frag_shader);
        gl::LinkProgram(program_id);

        gl::UseProgram(program_id);

    }



    event_loop.run(move |event, _, control_flow| {

        //*control_flow = glutin::event_loop::ControlFlow::Poll;
        *control_flow = glutin::event_loop::ControlFlow::Wait;

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, window_id } => {
                if window_id == context.window().id()
                {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }
            },
            Event::WindowEvent {event: WindowEvent::KeyboardInput{input, ..}, window_id} => {
                if window_id == context.window().id() {
                    // if let Some(keycode) = input.virtual_keycode {
                    //     if keycode == VirtualKeyCode::A {
                    //         flip = match input.state {
                    //             ElementState::Pressed => true,
                    //             ElementState::Released => false,
                    //         };
                    //     }
                    // }
                    //log::info!("Pressed");
                    flip = !flip;
                }
            },
            Event::WindowEvent { event: WindowEvent::Resized(size), window_id } =>
            {
                if window_id == context.window().id() {
                    //flip = true;
                    width = size.width;
                    height = size.height;

                }
            }
            Event::LoopDestroyed => {
                unsafe {
                    //gl::DeleteFramebuffers(1, &framebuffer);
                    gl::DeleteTextures(1, &texture);
                    gl::DeleteBuffers(1, &vbo);
                }
            },
            Event::RedrawRequested(id) if id == context.window().id() => {
                context.window().request_redraw();
            }
            Event::MainEventsCleared => {

            },

            _ => (),
        }
        unsafe {
            gl::Disable(gl::BLEND);
            gl::BlendColor(1.0, 0., 0., 1.0);


            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, width as i32, height as i32);
            gl::ClearColor(0.2, 0.3, if flip {0.3} else {1.0}, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        context.swap_buffers().unwrap();

    });
}
