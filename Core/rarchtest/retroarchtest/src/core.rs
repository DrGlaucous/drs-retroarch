
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::pin::Pin;
use std::mem;

use libc::{c_char, c_uint};
use std::ffi::c_void;
use gl;
use gl::types::{GLsizei, GLuint, GLfloat, GLsizeiptr, GLint, GLchar};



use crate::libretro::{self, gl_frame_done, retro_filesystem_context, variables_need_update};
use crate::libretro::retro_filesystem_context::{FileHandle, DirHandle, FileAccessHint, FileAccessMode, FileSeekPos};

/// Static system information sent to the frontend on request
pub const SYSTEM_INFO: libretro::SystemInfo = libretro::SystemInfo {
    library_name: cstring!("AAAAAAAAAAAAAAAA"),
    library_version: "-20" as *const _ as *const c_char,
    valid_extensions: cstring!("exe"),
    need_fullpath: true,
    block_extract: false,
};

pub const VERTICES: [f32; 9] = [
    -3.5, -3.5, 0.0,
     3.5, -3.5, 0.0,
     0.0,  3.5, 0.0
];


//"new" opengl 2.1 shaders:
const VERTEX_SHADER_SOURCE_11: &str = r#"
    //#version 110 //330 core
    attribute vec3 aPos;
    void main() {
        gl_Position = vec4(aPos, 1.0);
    }
"#;

const FRAGMENT_SHADER_SOURCE_11: &str = r#"
    //#version 110
    void main() {
        gl_FragColor = vec4(1.0, 0.2, 0.0, 1.0); // Red color
    }
"#;



const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;


pub fn handle_err() {
    
    unsafe{
        let err = gl::GetError();

        if err != 0 {
            log::error!("OpenGL error: {}", err);
        }
    }

}


////////////////////////BACKEND CALLS

/// Called when a game is loaded and a new context must be built
pub fn load_game(disc: PathBuf) -> Option<Box<dyn libretro::Context>> {
    log::info!("Loading {:?}", disc); //info!

    //todo: get disk into there
    Core::new(disc).ok()
        .map(|c| Box::new(c) as Box<dyn libretro::Context>)
}

pub fn init() {

}

pub fn init_variables() {

}

// Precise FPS values for the video output for the given
// VideoClock. It's actually possible to configure the PlayStation GPU
// to output with NTSC timings with the PAL clock (and vice-versa)
// which would make this code invalid but it wouldn't make a lot of
// sense for a game to do that.
pub enum VideoClock {
    Ntsc,
    Pal,
}

fn video_output_framerate(std: VideoClock) -> f32 {
    match std {
        // 53.690MHz GPU clock frequency, 263 lines per field,
        // 3413 cycles per line
        VideoClock::Ntsc => 60.0,
        // 53.222MHz GPU clock frequency, 314 lines per field,
        // 3406 cycles per line
        VideoClock::Pal => 50.0,
    }
}

//get the current state of the backend's video settings (placeholder for now...)
fn get_av_info(std: VideoClock, upscaling: u32) -> libretro::SystemAvInfo {

    // Maximum resolution supported by the PlayStation video
    // output is 640x480
    let max_width = (WIDTH * upscaling) as c_uint;
    let max_height = (HEIGHT * upscaling) as c_uint;

    libretro::SystemAvInfo {
        geometry: libretro::GameGeometry {
            // The base resolution will be overriden using
            // ENVIRONMENT_SET_GEOMETRY before rendering a frame so
            // this base value is not really important
            base_width: max_width,
            base_height: max_height,
            max_width: max_width,
            max_height: max_height,
            aspect_ratio: 4./3.,
        },
        timing: libretro::SystemTiming {
            fps: video_output_framerate(std) as f64,
            sample_rate: 44_100.
        }
    }
}

////////////////////////SETTINGS

//helper for the settings macro below
fn parse_upscale(opt: &str) -> Result<u32, <u32 as FromStr>::Err> {
    let num = opt.trim_matches(|c: char| !c.is_numeric());

    num.parse()
}

fn parse_color_depth(opt: &str) -> Result<u8, <u8 as FromStr>::Err> {
    let num = opt.trim_matches(|c: char| !c.is_numeric());

    num.parse()
}

fn parse_bool(opt: &str) -> Result<bool, ()> {
    match opt {
        "true" | "enabled" | "on" => Ok(true),
        "false" | "disabled" | "off" => Ok(false),
        _ => Err(()),
    }
}

//macro to build settings for the setting menu
libretro_variables!(
    struct CoreVariables (prefix = "rustation") {
        internal_upscale_factor: u32, parse_upscale
            => "Internal upscaling factor; \
                1x (native)|2x|3x|4x|5x|6x|7x|8x|9x|10x",
        internal_color_depth: u8, parse_color_depth
            => "Internal color depth; dithered 16bpp (native)|32bpp",
        scale_dither: bool, parse_bool
            => "Scale dithering pattern with internal resolution; \
                enabled|disabled",
        wireframe: bool, parse_bool
            => "Wireframe mode; disabled|enabled",
        bios_menu: bool, parse_bool
            => "Boot to BIOS menu; disabled|enabled",
        skip_bios_animation: bool, parse_bool
            => "Skip BIOS boot animations; disabled|enabled",
        display_internal_fps: bool, parse_bool
            => "Display internal FPS; disabled|enabled",
        log_frame_counters: bool, parse_bool
            => "Log frame counters; disabled|enabled",
        enable_debug_uart: bool, parse_bool
            => "Enable debug UART in the BIOS; disabled|enabled",
        debug_on_break: bool, parse_bool
            => "Trigger debugger on BREAK instructions; disabled|enabled",
        debug_on_key: bool, parse_bool
            => "Trigger debugger when Pause/Break is pressed; disabled|enabled",
        debug_on_reset: bool, parse_bool
            => "Trigger debugger when starting or resetting the emulator; \
                disabled|enabled",
        log_bios_calls: bool, parse_bool
            => "Log BIOS calls; disabled|enabled",
    });

/////////////////////CORE IMPL


struct Core  {
    //runner backend and other loop pointers are in here
    elapsed_time: i64,
    async_audio_enabled: bool,

    has_set_res: bool,
    //get_current_framebuffer: fn() -> usize,
    //get_proc_address: fn(&str) -> *const c_void,

    texture: GLuint,
    vbo: GLuint,
    vao: GLuint,

    vert_shader: GLuint,
    frag_shader: GLuint,
    program_id: GLuint,


    //random unrealted testing stuff:
    personal_object: Option<ObjectToPass>

}

impl  Core  {

    fn new(game_path: PathBuf) -> Result<Core, ()>{

        //initialize the hardware backends
        if !libretro::set_pixel_format(libretro::PixelFormat::Xrgb8888) {
            log::warn!("Can't set pixel format");
            return Err(());
        }

        //todo! make ContextType dynamic so we can run with gles as well
        if !libretro::hw_context::init() {
            log::warn!("Failed to init hardware context");
            return Err(());
        }

        if !libretro::register_frame_time_callback(50) {
            log::warn!("Failed to init delta frame counter");
            return Err(());
        }

        let async_audio_enabled = if !libretro::async_audio_context::register_async_audio_callback() {
            log::warn!("Failed to init async audio, falling back to synchronous");
            false
        } else {true};

        if !libretro::retro_filesystem_context::register_vfs_interface(3) {
           log::warn!("Failed to init filesystem");
           return Err(());
        }

        let tt = retro_filesystem_context::fopen(game_path, 1, 0);


        //random unrealted testing stuff:
        let mut personal_object: Option<ObjectToPass> = None;
        let initializer = SubFuncObj{};

        initializer.make_obj(&mut personal_object);



        gl::load_with(|s| {libretro::hw_context::get_proc_address(s) as *const _});

        //function to use in order to get the current framebuffer
        //let get_current_framebuffer: fn() -> usize = libretro::hw_context::get_current_framebuffer;
        //let get_proc_address: fn(&str) -> *const c_void = libretro::hw_context::get_proc_address;


        Ok(Core {

            //get_current_framebuffer,
            //get_proc_address,
            elapsed_time: 0,
            async_audio_enabled,
            has_set_res: false,
            vbo: 0,
            vao: 0,
            texture: 0,

            vert_shader: 0,
            frag_shader: 0,
            program_id: 0,
            personal_object: None,
            
        })
        
    }


    fn bind_libretro_framebuffer(&mut self) {
        // let (f_w, f_h) = self.frontend_resolution;
        // let (w, h) = self.config.display_resolution;

        // let upscale = self.internal_upscaling;

        // // XXX scale w and h when implementing increased internal
        // // resolution
        // let w = (w as u32) * upscale;
        // let h = (h as u32) * upscale;

        // //if false {//w != f_w || h != f_h {
        // if w != f_w || h != f_h {
        //     // We need to change the frontend's resolution
        //     let geometry = libretro::GameGeometry {
        //         base_width: w as c_uint,
        //         base_height: h as c_uint,
        //         // Max parameters are ignored by this call
        //         max_width: 0,
        //         max_height: 0,
        //         // Is this accurate?
        //         aspect_ratio: 4./3.,
        //     };

        //     info!("Target framebuffer size: {}x{}", w, h);

        //     //libretro::set_geometry(&geometry);

        //     self.frontend_resolution = (w, h);
        // }

        // Bind the output framebuffer provided by the frontend
        let fbo = libretro::hw_context::get_current_framebuffer() as GLuint;

        unsafe {
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, fbo);
            //gl::Viewport(0, 0, w as GLsizei, h as GLsizei);
            gl::Viewport(0, 0, WIDTH as GLsizei, HEIGHT as GLsizei);
        }
    }


    pub fn make_dirty_shaders(&mut self) {

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
    
            program_id = gl::CreateProgram();
            gl::AttachShader(program_id, vert_shader);
            gl::AttachShader(program_id, frag_shader);
            gl::LinkProgram(program_id);
    
            //gl::UseProgram(program_id);

    
        }

        let mut vbo: GLuint = 0;
        let mut vao: GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);
            gl::EnableVertexAttribArray(0);
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(gl::ARRAY_BUFFER, (VERTICES.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr, VERTICES.as_ptr() as *const _, gl::STATIC_DRAW);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<GLfloat>() as GLsizei, std::ptr::null());
        }

        self.vao = vao;
        self.vbo = vbo;
        self.vert_shader = vert_shader;
        self.program_id = program_id;
        self.frag_shader = frag_shader;


    }


}


#[repr(C)]
#[derive(Copy, Clone)]
pub struct VertexData {
    pub position: (f32, f32),
    pub color: (u8, u8, u8, u8),
    pub uv: (f32, f32),
}

// pub fn load_gl(gl_context: &mut GLContext) -> &'static Gl {
//     unsafe {
//         if let Some(gl) = &GL_PROC {
//             return gl;
//         }
//         let gl = gl::Gles2::load_with(|ptr| (gl_context.get_proc_address)(&mut gl_context.user_data, ptr));
//         let version = {
//             let p = gl.GetString(gl::VERSION);
//             if p.is_null() {
//                 "unknown".to_owned()
//             } else {
//                 let data = CStr::from_ptr(p as *const _).to_bytes().to_vec();
//                 String::from_utf8(data).unwrap()
//             }
//         };
//         log::info!("OpenGL version {}", version);
//         GL_PROC = Some(Gl { gl });
//         GL_PROC.as_ref().unwrap()
//     }
// }

impl  libretro::Context  for Core  {

    fn render_frame(&mut self) {



        handle_err();


        if !self.has_set_res {
            let geometry = libretro::GameGeometry {
                base_width: WIDTH as c_uint,
                base_height: HEIGHT as c_uint,
                // Max parameters are ignored by this call
                max_width: 0,
                max_height: 0,
                // Is this accurate?
                aspect_ratio: WIDTH as f32/(HEIGHT) as f32,
            };

            libretro::set_geometry(&geometry);
            self.has_set_res = true;



            self.make_dirty_shaders();


            //let tt = retro_filesystem_context::fopen(PathBuf::from("./Maze.ch8"), 1, 0);





            return;



        }



        //let fbo = libretro::hw_context::get_current_framebuffer() as GLuint;
        self.bind_libretro_framebuffer();
        unsafe {
            handle_err();
            // gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, fbo);
            // //gl::Viewport(0, 0, w as GLsizei, h as GLsizei);
            // gl::Viewport(0, 0, WIDTH as GLsizei, HEIGHT as GLsizei);
            // gl::ClearColor(0.5,
            //     1.0,
            //     1.0,
            //     // XXX Not entirely sure what happens
            //     // to the mask bit in fill_rect. No$
            //     // seems to say that it's set to 0.
            //     0.);
            // gl::Clear(gl::COLOR_BUFFER_BIT);

            // //test minimal draw
            // let uv = (0.0, 0.0);
            // let color = (255, 0, 0, 0);

            // let vertices: [f32; 9] = [
            //     -0.5, -0.5, 0.0,
            //      0.5, -0.5, 0.0,
            //      0.0,  0.5, 0.0
            // ];


            // gl::BufferData(
            //     gl::ARRAY_BUFFER,
            //     (vertices.len() * mem::size_of::<VertexData>()) as _,
            //     vertices.as_ptr() as _,
            //     gl::STREAM_DRAW,
            // );

            // gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as _);

            //gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

            // gl::Viewport(0, 0, WIDTH as i32, HEIGHT as i32);
            // gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            // gl::Clear(gl::COLOR_BUFFER_BIT);

            // gl::UseProgram(self.program_id);
            // gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            // gl::DrawArrays(gl::TRIANGLES, 0, 3);
            // gl::BindBuffer(gl::ARRAY_BUFFER, 0);

            //open
            gl::Disable(gl::SCISSOR_TEST);
            gl::BindVertexArray(self.vao);

            //run
            gl::ClearColor(0.5,
                1.0,
                1.0,
                // XXX Not entirely sure what happens
                // to the mask bit in fill_rect. No$
                // seems to say that it's set to 0.
                0.);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.program_id);

            let vertices: [f32; 9] = [
                -0.5, -0.5, 0.0, //left
                 0.5, -0.5, 0.0, //right
                 0.0,  0.5, 0.0 //center
            ];
            gl::BufferData(gl::ARRAY_BUFFER, (vertices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr, vertices.as_ptr() as *const _, gl::STATIC_DRAW);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<GLfloat>() as GLsizei, std::ptr::null());
            gl::DrawArrays(gl::TRIANGLES, 0, 3);


            //close
            gl::Disable(gl::BLEND);
            gl::BlendColor(0., 0., 0., 1.0);
            // gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
            // gl::BlendFuncSeparate(gl::ONE,
            //                       gl::ZERO,
            //                       gl::ONE,
            //                       gl::ZERO);
            // gl::ActiveTexture(gl::TEXTURE0);
            // gl::BindTexture(gl::TEXTURE_2D, 0);

            gl::BindVertexArray(0);
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
            gl::LineWidth(1.);
            handle_err();


        }


        gl_frame_done(WIDTH, HEIGHT)

    }

    fn get_system_av_info(&self) -> libretro::SystemAvInfo {

        libretro::SystemAvInfo {
            geometry: libretro::GameGeometry {
                // The base resolution will be overriden using
                // ENVIRONMENT_SET_GEOMETRY before rendering a frame so
                // this base value is not really important
                base_width: WIDTH,
                base_height: HEIGHT,
                max_width: WIDTH,
                max_height: HEIGHT,
                aspect_ratio: (WIDTH) as f32/(HEIGHT as f32),
            },
            timing: libretro::SystemTiming {
                fps: 50. as f64,
                sample_rate: 44_100.
            }
        }
    }

    fn refresh_variables(&mut self){

    }

    fn reset(&mut self) {
    }

    fn gl_context_reset(&mut self){

        // Should I call this at every reset? Does it matter?
        info!("OpenGL context reset");
        gl::load_with(|s| {
            libretro::hw_context::get_proc_address(s) as *const _
        });
    }

    fn gl_context_destroy(&mut self){
        info!("OpenGL context destroy");

    }

    //todo: remove unused functions from Context
    fn serialize_size(&self) -> usize {
        0
    }

    fn serialize(&self, mut buf: &mut [u8]) -> Result<(), ()> {
        Ok(())
    }
    fn unserialize(&mut self, mut buf: &[u8]) -> Result<(), ()> {
        Ok(())
    }
    fn elapse_time(&mut self, delta_time: i64) {
        self.elapsed_time = delta_time;
    }

    fn async_audio_callback(&mut self) {
        
    }

    fn async_audio_state(&mut self, _: bool) {
        
    }



}



struct ObjectToPass {
    var1: i32,
    var2: i32,
}

struct SubFuncObj {}

impl SubFuncObj {
    pub fn make_obj(&self, passer: &mut Option<ObjectToPass>) -> bool{

        let new_obj = ObjectToPass{
            var1: 0,
            var2: 0,
        };

        *passer = Some(new_obj);

        true
    }
}






