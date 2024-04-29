// XXX temporarily necessary to remove annoying warnings about the
// cstring! macro in rustc 1.10.0 which don't have a simple
// workaround. Will be removed once we find a better way to silence
// them.
#![allow(const_err)]

#[macro_use]
pub mod libretro;
#[macro_use]
mod retrogl;
mod retrolog;
mod renderer;
mod savestate;
mod debugger;
mod vcd;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;

use libc::{c_char, c_uint};

use rustc_serialize::{Encodable, Encoder, Decodable, Decoder};

use rustation::cdrom::disc::{Disc, Region};
use rustation::bios::{Bios, BIOS_SIZE};
use rustation::bios::db::Metadata;
use rustation::gpu::{Gpu, VideoClock};
use rustation::memory::Interconnect;
use rustation::cpu::Cpu;
use rustation::padmemcard::gamepad::{Button, ButtonState, DigitalProfile};
use rustation::shared::SharedState;
use rustation::parallel_io::exe_loader;
use rustation::tracer;

use cdimage::cue::Cue;

use debugger::Debugger;

use gl::types::{GLsizei, GLuint, GLfloat, GLsizeiptr, GLint, GLchar};
use retrogl::buffer::DrawBuffer;
use renderer::ImageLoadVertex;
use retrogl::shader::{Shader, ShaderType};
use retrogl::program::Program;



#[macro_use]
extern crate log;
extern crate libc;
extern crate gl;
extern crate rustation;
extern crate arrayvec;
extern crate cdimage;
extern crate rustc_serialize;
extern crate time;

/// Static system information sent to the frontend on request
const SYSTEM_INFO: libretro::SystemInfo = libretro::SystemInfo {
    library_name: cstring!("Rustation"),
    library_version: rustation::VERSION_CSTR as *const _ as *const c_char,
    valid_extensions: cstring!("cue|exe|psexe|psx"),
    need_fullpath: false,
    block_extract: false,
};

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

const WIN_WIDTH: u32 = 800;
const WIN_HEIGHT: u32 = 600;

const VERTICES: [f32; 9] = [
        -0.5, -0.5, 0.0,
         0.5, -0.5, 0.0,
         0.0,  0.5, 0.0
];

/// Emulator context
struct Context {
    //retrogl: retrogl::RetroGl,
    cpu: Cpu,
    shared_state: SharedState,
    debugger: Debugger,
    disc_path: PathBuf,
    video_clock: VideoClock,
    /// When true the internal FPS monitoring in enabled
    monitor_internal_fps: bool,
    /// Cached value for the maximum savestate size in bytes
    savestate_max_len: usize,
    /// If true we log the counters at the end of each frame
    log_frame_counters: bool,
    /// If true we trigger the debugger when Pause/Break is pressed
    debug_on_key: bool,



    //test
    has_init: bool,
    vert_shader: GLuint,
    frag_shader: GLuint,
    program_id: GLuint,
    vao: GLuint,
    vbo: GLuint,
    this_random_buffer: Option<DrawBuffer<ImageLoadVertex>>,




}

impl Context {
    fn new(disc: &Path) -> Result<Context, ()> {
        info!("Using Rustation {}", rustation::VERSION);

        let (mut cpu, video_clock) =
            match exe_loader::ExeLoader::load_file(disc) {
                Ok(l) => try!(Context::load_exe(l)),
                // Not an EXE, load as a disc
                Err(exe_loader::Error::UnknownFormat) =>
                    try!(Context::load_disc(disc)),
                Err(e) => {
                    error!("Couldn't load EXE file: {:?}", e);
                    return Err(())
                }
            };

        let shared_state = SharedState::new();
        let retrogl = try!(retrogl::RetroGl::new(video_clock));

        if CoreVariables::enable_debug_uart() {
            let result =
                cpu.interconnect_mut().bios_mut().enable_debug_uart();

            match result {
                Ok(_) => info!("BIOS patched to enable debug UART"),
                Err(_) => warn!("Couldn't patch BIOS to enable debug UART"),
            }
        }

        let mut context =
            Context {
                //retrogl: retrogl,
                cpu: cpu,
                shared_state: shared_state,
                debugger: Debugger::new(),
                disc_path: disc.to_path_buf(),
                video_clock: video_clock,
                monitor_internal_fps: false,
                savestate_max_len: 0,
                log_frame_counters: false,
                debug_on_key: false,


                has_init: false,
                frag_shader: 0,
                vert_shader: 0,
                program_id: 0,
                vao: 0,
                vbo: 0,
                this_random_buffer: None


            };

        libretro::Context::refresh_variables(&mut context);

        let max_len = try!(context.compute_savestate_max_length());

        context.savestate_max_len = max_len;

        context.setup_controllers();

        if CoreVariables::debug_on_reset() {
            context.trigger_break();
        }

        Ok(context)
    }

    /// Initialize the controllers connected to the emulated console
    fn setup_controllers(&mut self) {
        // XXX for now I only hardcode a digital pad in slot 1
        // (leaving slot 0 disconnected).
        self.cpu.interconnect_mut()
            .pad_memcard_mut()
            .gamepads_mut()[0]
            .set_profile(Box::new(DigitalProfile::new()));
    }

    fn compute_savestate_max_length(&mut self) -> Result<usize, ()> {
        // In order to get the full size we're just going to use a
        // dummy Write struct which will just count how many bytes are
        // being written
        struct WriteCounter(usize);

        impl ::std::io::Write for WriteCounter {
            fn write(&mut self, buf: &[u8]) -> ::std::io::Result<usize> {
                let len = buf.len();

                self.0 += len;

                Ok(len)
            }

            fn flush(&mut self) -> ::std::io::Result<()> {
                Ok(())
            }
        }

        let mut counter = WriteCounter(0);

        try!(self.save_state(&mut counter));

        let len = counter.0;

        // Our savestate format has variable length, in particular we
        // have the GPU's load_buffer which can grow to 1MB in the
        // worst case scenario (the entire VRAM). I'm going to be
        // optimistic here and give us 512KB of "headroom", that
        // should be enough 99% of the time, hopefully.
        let len = len + 512 * 1024;

        Ok(len)
    }

    fn save_state(&self, writer: &mut ::std::io::Write) -> Result<(), ()> {

        let mut encoder =
            match savestate::Encoder::new(writer) {
                Ok(encoder) => encoder,
                Err(e) => {
                    warn!("Couldn't create savestate encoder: {:?}", e);
                    return Err(())
                }
            };

        match self.encode(&mut encoder) {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Couldn't serialize emulator state: {:?}", e);
                Err(())
            }
        }
    }

    fn load_state(&mut self, reader: &mut ::std::io::Read) -> Result<(), ()> {
        let mut decoder =
            match savestate::Decoder::new(reader) {
                Ok(decoder) => decoder,
                Err(e) => {
                    warn!("Couldn't create savestate decoder: {:?}", e);
                    return Err(())
                }
            };

        // I don't implement Decodable for Context itself because I
        // don't want to create a brand new instance. Things like the
        // debugger or disc path don't need to be reset
        let decoded =
            decoder.read_struct("Context", 4, |d| {
                let cpu = try!(d.read_struct_field("cpu", 0,
                                                   Decodable::decode));

                let retrogl = try!(d.read_struct_field("retrogl", 1,
                                                       Decodable::decode));

                let video_clock = try!(d.read_struct_field("video_clock", 2,
                                                           Decodable::decode));

                let shared_state = try!(d.read_struct_field("shared_state", 3,
                                                            Decodable::decode));

                Ok((cpu, retrogl, video_clock, shared_state))
            });

        let (cpu, retrogl, video_clock, shared_state) =
            match decoded {
                Ok(d) => d,
                Err(e) => {
                    warn!("Couldn't decode savestate: {:?}", e);
                    return Err(())
                }
            };

        // The savestate doesn't contain the BIOS, only the metadata
        // describing which BIOS was used when the savestate was made
        // (in order to save space and not redistribute the BIOS with
        // savestate files). So let's find it back and reload it.
        let bios_md = self.cpu.interconnect().bios().metadata();

        // Convert sha256 to a hex string for pretty printing
        let sha256_hex: String =
            bios_md.sha256.iter()
            .fold(String::new(), |s, b| s + &format!("{:02x}", b));

        info!("Loading savestate BIOS: {:?} (SHA256: {})",
              bios_md, sha256_hex);

        let bios =
            match Context::find_bios(|md| { md.sha256 == bios_md.sha256 }) {
                Some(b) => b,
                None => {
                    error!("Couldn't find the savestate BIOS, bailing out");
                    return Err(());
                }
            };

        //let gl_is_valid = self.retrogl.is_valid();

        // Save the disc before we replace everything
        let disc = self.cpu.interconnect_mut().cdrom_mut().remove_disc();

        self.cpu = cpu;
        //self.retrogl = retrogl;
        self.video_clock = video_clock;
        self.shared_state = shared_state;

        self.cpu.interconnect_mut().set_bios(bios);
        self.cpu.interconnect_mut().cdrom_mut().set_disc(disc);

        self.setup_controllers();

        // If we had a valid GL context before the load we can
        // directly reload everything. Otherwise it'll be done when
        // the frontend calls context_reset
        //if gl_is_valid {
            //self.retrogl.context_reset();
        //}

        info!("Savestate load successful");

        Ok(())
    }

    fn load_exe(loader: exe_loader::ExeLoader)
                -> Result<(Cpu, VideoClock), ()> {
        let region =
            match loader.region() {
                Some(r) => {
                    info!("Detected EXE region: {:?}", r);
                    r
                }
                None => {
                    warn!("Couldn't establish EXE file region, \
                           defaulting to NorthAmerica");
                    Region::NorthAmerica
                }
            };

        // In order for the EXE loader to word correctly without any
        // disc we need to patch the BIOS, so let's make sure that the
        // animation_jump_hook is available
        let bios_predicate = |md: &Metadata| {
            md.region == region && md.animation_jump_hook.is_some()
        };

        let mut bios =
            match Context::find_bios(bios_predicate) {
                Some(b) => b,
                None => {
                    error!("Couldn't find a BIOS, bailing out");
                    return Err(());
                }
            };

        if let Err(_) = loader.patch_bios(&mut bios) {
             error!("EXE loader couldn't patch the BIOS, giving up");
             return Err(());
        }

        let video_clock =
            match region {
                Region::Europe => VideoClock::Pal,
                Region::NorthAmerica => VideoClock::Ntsc,
                Region::Japan => VideoClock::Ntsc,
            };

        let gpu = Gpu::new(video_clock);
        let mut inter = Interconnect::new(bios, gpu, None);

        // Plug the EXE loader in the Parallel I/O port
        inter.parallel_io_mut().set_module(Box::new(loader));

        Ok((Cpu::new(inter), video_clock))
    }

    fn load_disc(disc: &Path) -> Result<(Cpu, VideoClock), ()> {

        let image =
            match Cue::new(disc) {
                Ok(c) => c,
                Err(e) => {
                    error!("Couldn't load {}: {}", disc.to_string_lossy(), e);
                    return Err(());
                }
            };

        let disc =
            match Disc::new(Box::new(image)) {
                Ok(d) => d,
                Err(e) => {
                    error!("Couldn't load {}: {}", disc.to_string_lossy(), e);
                    return Err(());
                }
            };

        let serial = disc.serial_number();
        let region = disc.region();

        info!("Disc serial number: {}", serial);
        info!("Detected disc region: {:?}", region);

        let mut bios =
            match Context::find_bios(|md| { md.region == region }) {
                Some(b) => b,
                None => {
                    error!("Couldn't find a BIOS, bailing out");
                    return Err(());
                }
            };

        let bios_menu = CoreVariables::bios_menu();

        // Skipping BIOS animations seems to break the BIOS menu, so
        // we ignore this setting when the menu is requested.
        if CoreVariables::skip_bios_animation() && !bios_menu {
            match bios.patch_boot_animation() {
                Ok(_) => info!("Patched BIOS to skip boot animation"),
                Err(_) => warn!("Failed to patch BIOS to skip boot animations"),
            }
        }

        let video_clock =
            match region {
                Region::Europe => VideoClock::Pal,
                Region::NorthAmerica => VideoClock::Ntsc,
                Region::Japan => VideoClock::Ntsc,
            };

        // If we're asked to boot straight to the BIOS menu we pretend
        // no disc is present.
        let disc =
            if bios_menu {
                None
            } else {
                Some(disc)
            };

        let gpu = Gpu::new(video_clock);
        let inter = Interconnect::new(bios, gpu, disc);

        Ok((Cpu::new(inter), video_clock))
    }

    /// Attempt to find a BIOS for `region` in the system directory
    fn find_bios<F>(predicate: F) -> Option<Bios>
        where F: Fn(&Metadata) -> bool {
        let system_directory =
            match libretro::get_system_directory() {
                Some(dir) => dir,
                // libretro.h says that when the system directory is not
                // provided "it's up to the implementation to find a
                // suitable directory" but I'm not sure what to put
                // here. Maybe "."? I'd rather give an explicit error
                // message instead.
                None => {
                    error!("The frontend didn't give us a system directory, \
                            no BIOS can be loaded");
                    return None;
                }
            };

        info!("Looking for a suitable BIOS in {:?}", system_directory);

        let dir =
            match ::std::fs::read_dir(&system_directory) {
                Ok(d) => d,
                Err(e) => {
                    error!("Can't read directory {:?}: {}",
                           system_directory, e);
                    return None;
                }
            };

        for entry in dir {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    match entry.metadata() {
                        Ok(md) => {
                            if !md.is_file() {
                                debug!("Ignoring {:?}: not a file", path);
                            } else if md.len() != BIOS_SIZE as u64 {
                                debug!("Ignoring {:?}: bad size", path);
                            } else {
                                let bios = Context::try_bios(&predicate, &path);

                                if bios.is_some() {
                                    // Found a valid BIOS!
                                    return bios;
                                }
                            }
                        }
                        Err(e) =>
                            warn!("Ignoring {:?}: can't get file metadata: {}",
                                  path, e)
                    }
                }
                Err(e) => warn!("Error while reading directory: {}", e),
            }
        }

        None
    }

    /// Attempt to read and load the BIOS at `path`
    fn try_bios<F>(predicate: F, path: &Path) -> Option<Bios>
        where F: Fn(&Metadata) -> bool {

        let mut file =
            match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    warn!("Can't open {:?}: {}", path, e);
                    return None;
                }
            };

        // Load the BIOS
        let mut data = Box::new([0; BIOS_SIZE]);
        let mut nread = 0;

        while nread < BIOS_SIZE {
            nread +=
                match file.read(&mut data[nread..]) {
                    Ok(0) => {
                        warn!("Short read while loading {:?}", path);
                        return None;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        warn!("Error while reading {:?}: {}", path, e);
                        return None;
                    }
                };
        }

        match Bios::new(data) {
            Some(bios) => {
                let md = bios.metadata();

                info!("Found BIOS DB entry for {:?}: {:?}", path, md);

                if md.known_bad {
                    warn!("Ignoring {:?}: known bad dump", path);
                    None
                } else if !predicate(md) {
                    info!("Ignoring {:?}: rejected by predicate", path);
                    None
                } else {
                    info!("Using BIOS {:?} ({:?})", path, md);
                    Some(bios)
                }
            }
            None => {
                debug!("Ignoring {:?}: not a known PlayStation BIOS", path);
                None
            }
        }
    }

    fn poll_controllers(&mut self) {
        // XXX we only support pad 0 for now
        let pad = self.cpu.interconnect_mut()
            .pad_memcard_mut()
            .gamepads_mut()[0]
            .profile_mut();

        for &(retrobutton, psxbutton) in &BUTTON_MAP {
            let state =
                if libretro::button_pressed(0, retrobutton) {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };

            pad.set_button_state(psxbutton, state);
        }
    }

    /// Trigger a breakpoint in the debugger
    fn trigger_break(&mut self) {
        rustation::debugger::Debugger::trigger_break(&mut self.debugger);
    }




    pub fn make_dirty_shaders(&mut self) {

        let mut vert_shader = 0;
        let mut frag_shader = 0;
        unsafe{
            vert_shader = gl::CreateShader(gl::VERTEX_SHADER);
            frag_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        }
        let vert_sources = [VERTEX_SHADER_SOURCE.as_ptr() as *const GLchar];
        let frag_sources = [FRAGMENT_SHADER_SOURCE.as_ptr() as *const GLchar];
        let vert_sources_len = [VERTEX_SHADER_SOURCE.len() as GLint - 1];
        let frag_sources_len = [FRAGMENT_SHADER_SOURCE.len() as GLint - 1];
    
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
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(gl::ARRAY_BUFFER, (VERTICES.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr, VERTICES.as_ptr() as *const _, gl::STATIC_DRAW);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<GLfloat>() as GLsizei, std::ptr::null());
            gl::EnableVertexAttribArray(0);
        }

        self.vao = vao;
        self.vbo = vbo;
        self.vert_shader = vert_shader;
        self.program_id = program_id;
        self.frag_shader = frag_shader;


    }


}

impl Drop for Context {
    fn drop(&mut self) {
        if cfg!(feature = "trace") {
            // Dump the trace before destroying everything
            let path = VCD_TRACE_PATH;

            let trace = tracer::remove_trace();

            if trace.is_empty() {
                warn!("Empty trace, ignoring");
            } else {
                info!("Dumping VCD trace file to {}", path);

                let mut vcd_file = File::create(path).unwrap();

                let content = &*self.disc_path.to_string_lossy();

                let bios_md = self.cpu.interconnect().bios().metadata();
                let bios_desc = format!("{:?}", bios_md);

                vcd::dump_trace(&mut vcd_file, content, &bios_desc, trace);
            }
        }
    }
}

impl libretro::Context for Context {

    fn render_frame(&mut self) {
        //self.poll_controllers();

        // let debug_request =
        //     self.debug_on_key &&
        //     libretro::key_pressed(0, libretro::Key::Pause);

        // if debug_request {
        //     self.trigger_break();
        // }

        let cpu = &mut self.cpu;
        let shared_state = &mut self.shared_state;
        let debugger = &mut self.debugger;


        //bind fb
        {
            if !self.has_init {
                let geometry = libretro::GameGeometry {
                    base_width: WIN_WIDTH as c_uint,
                    base_height: WIN_HEIGHT as c_uint,
                    // Max parameters are ignored by this call
                    max_width: 0,
                    max_height: 0,
                    // Is this accurate?
                    aspect_ratio: WIN_WIDTH as f32/(WIN_HEIGHT) as f32,
                };
                libretro::set_geometry(&geometry);
                info!("Target framebuffer size: {}x{}", WIN_WIDTH, WIN_HEIGHT);
                self.has_init = true;
            }     
            // Bind the output framebuffer provided by the frontend
            let fbo = libretro::hw_context::get_current_framebuffer() as GLuint;
            unsafe {
                gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, fbo);
                //gl::Viewport(0, 0, w as GLsizei, h as GLsizei);
                gl::Viewport(0, 0, WIN_WIDTH as GLsizei, WIN_HEIGHT as GLsizei);
            }
        }

        //draw
        {
            // Draw pending commands
            //self.draw().unwrap();

            // We can now render to the frontend's buffer.
            //self.bind_libretro_framebuffer();

            // // // Bind `fb_out` to texture unit 1
            // self.fb_out.bind(gl::TEXTURE1);

            // // First we draw the visible part of fb_out
            unsafe {
                gl::Disable(gl::SCISSOR_TEST);
                gl::Disable(gl::DEPTH_TEST);
                gl::Disable(gl::BLEND);
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
            }

            // let (fb_x_start, fb_y_start) = self.config.display_top_left;
            // let (fb_width, fb_height) = self.config.display_resolution;

            // let fb_x_end = fb_x_start + fb_width;
            // let fb_y_end = fb_y_start + fb_height;

            //////////////////////

            // self.output_buffer.clear().unwrap();
            // self.output_buffer.push_slice(
            //     &[OutputVertex { position: [-1., -1.],
            //                      fb_coord: [fb_x_start, fb_y_end] },
            //       OutputVertex { position: [1., -1.],
            //                      fb_coord: [fb_x_end, fb_y_end] },
            //       OutputVertex { position: [-1., 1.],
            //                      fb_coord: [fb_x_start, fb_y_start] },
            //       OutputVertex { position: [1., 1.],
            //                      fb_coord: [fb_x_end, fb_y_start] }])
            //     .unwrap();

            // let depth_24bpp = self.config.display_24bpp as GLint;

            // self.output_buffer.program()
            //    .uniform1i("fb", 1).unwrap();
            // self.output_buffer.program()
            //    .uniform1i("depth_24bpp", depth_24bpp).unwrap();
            // self.output_buffer.program()
            //    .uniform1ui("internal_upscaling", self.internal_upscaling).unwrap();

            // self.output_buffer.draw(gl::TRIANGLE_STRIP).unwrap();


            //self.fill_rect([200,60,255], (0,0), (1,1));
            //self.fill_rect([30,244,80], (5,5), (1,1));
            unsafe {


                gl::ClearColor(0.5,
                            1.0,
                            1.0,
                            // XXX Not entirely sure what happens
                            // to the mask bit in fill_rect. No$
                            // seems to say that it's set to 0.
                            0.);
                gl::Clear(gl::COLOR_BUFFER_BIT);

                //self.fill_rect([200,60,255], (0,0), (1,1));


                gl::UseProgram(self.program_id);

                //gl::Disable(gl::BLEND);
                //gl::BlendColor(1.0, 0., 0., 1.0);


                //test minimal draw
                //let uv = (0.0, 0.0);
                //let color = (255, 255, 255, 0);
                // let vertices: [f32; 9] = [
                //     -0.5, -0.5, 0.0, //left
                //     0.5, -0.5, 0.0, //right
                //     0.0,  0.5, 0.0 //center
                // ];


                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                gl::DrawArrays(gl::TRIANGLES, 0, 3);
                gl::BindBuffer(gl::ARRAY_BUFFER, 0);

                //gl::BufferData(gl::ARRAY_BUFFER, (vertices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr, vertices.as_ptr() as *const _, gl::STATIC_DRAW);
                //gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 3 * std::mem::size_of::<GLfloat>() as GLsizei, std::ptr::null());
                //gl::DrawArrays(gl::TRIANGLES, 0, 3);



                // let vertices = [
                //     VertexData { position: (-1.0 as _, 1.0 as _), uv, color },
                //     VertexData { position: (-1.0 as _, -1.0 as _), uv, color },
                //     VertexData { position: (1.0 as _, -1.0 as _), uv, color },
                //     VertexData { position: (-1. as _, 1. as _), uv, color },
                //     VertexData { position: (-1. as _, -1. as _), uv, color },
                //     VertexData { position: (1. as _, 1. as _), uv, color },
                // ];
                // gl::BufferData(
                //     gl::ARRAY_BUFFER,
                //     (vertices.len() * mem::size_of::<VertexData>()) as _,
                //     vertices.as_ptr() as _,
                //     gl::STREAM_DRAW,
                // );

                // gl::DrawArrays(gl::TRIANGLES, 0, vertices.len() as _);

                //self.fill_rect([40,60,70], (5,5), (20,20));


                //end test

            }

            // Cleanup OpenGL context before returning to the frontend
            unsafe {

                gl::Disable(gl::BLEND);
                gl::BlendColor(0., 0., 0., 1.0);
                // gl::BlendEquationSeparate(gl::FUNC_ADD, gl::FUNC_ADD);
                // gl::BlendFuncSeparate(gl::ONE,
                //                       gl::ZERO,
                //                       gl::ONE,
                //                       gl::ZERO);
                // gl::ActiveTexture(gl::TEXTURE0);
                // gl::BindTexture(gl::TEXTURE_2D, 0);

                // gl::BindVertexArray(0);
                gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
                gl::LineWidth(1.);
            }

            // libretro::gl_frame_done(self.frontend_resolution.0,
            //                         self.frontend_resolution.1)
            libretro::gl_frame_done(WIN_WIDTH, WIN_HEIGHT)
        }


        //self.retrogl.render_frame(|renderer| {
        //    cpu.run_until_next_frame(debugger, shared_state, renderer);
        //});




    }

    fn get_system_av_info(&self) -> libretro::SystemAvInfo {
        let upscaling = CoreVariables::internal_upscale_factor();

        get_av_info(self.video_clock, upscaling)
    }

    fn refresh_variables(&mut self) {
        //self.monitor_internal_fps = CoreVariables::display_internal_fps();
        //self.log_frame_counters = CoreVariables::log_frame_counters();
        //self.debug_on_key = CoreVariables::debug_on_key();
        //self.cpu.set_debug_on_break(CoreVariables::debug_on_break());
        //self.debugger.set_log_bios_calls(CoreVariables::log_bios_calls());

        //self.retrogl.refresh_variables();
    }

    fn reset(&mut self) {
        match Context::load_disc(&self.disc_path) {
            Ok((cpu, video_clock)) => {
                info!("Game reset");
                self.cpu = cpu;
                self.video_clock = video_clock;
                self.shared_state = SharedState::new();

                if CoreVariables::debug_on_reset() {
                    self.trigger_break();
                }
            },
            Err(_) => warn!("Couldn't reset game"),
        }
    }

    fn gl_context_reset(&mut self) {
        //self.retrogl.context_reset();



        info!("OpenGL context reset");

        // Should I call this at every reset? Does it matter?
        gl::load_with(|s| {
            libretro::hw_context::get_proc_address(s) as *const _
        });

        if self.this_random_buffer.is_none() {

            // let image_load_buffer: DrawBuffer<ImageLoadVertex> =
            //     GlRenderer::build_buffer(
            //         include_str!("renderer/shaders/image_load_vertex.glsl"),
            //         include_str!("renderer/shaders/image_load_fragment.glsl"),
            //         4,
            //         false).unwrap();
            // self.this_random_buffer = Some(image_load_buffer);



            // let vertex_str = include_str!("renderer/shaders/image_load_vertex.glsl");
            // let fragmet_str = include_str!("renderer/shaders/image_load_fragment.glsl");
            // let vs = (Shader::new(vertex_str, ShaderType::Vertex)).unwrap(); //vertex shader (compile and get ID)
            // let fs = (Shader::new(fragmet_str, ShaderType::Fragment)).unwrap(); //compile fragment shader
            // let program = (Program::new(vs, fs)).unwrap(); //compile program using vertex and fragment shaders
            // let image_load_buffer = DrawBuffer::<ImageLoadVertex>::new(1, program, false).unwrap();
            // self.this_random_buffer = Some(image_load_buffer);









        }
        self.make_dirty_shaders();


    }

    fn gl_context_destroy(&mut self) {
        //self.retrogl.context_destroy();
    }

    fn serialize_size(&self) -> usize {
        self.savestate_max_len
    }

    fn serialize(&self, mut buf: &mut [u8]) -> Result<(), ()> {
        self.save_state(&mut buf)
    }

    fn unserialize(&mut self, mut buf: &[u8]) -> Result<(), ()> {
        self.load_state(&mut buf)
    }
}

impl Encodable for Context {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_struct("Context", 4, |s| {
            try!(s.emit_struct_field("cpu", 0,
                                     |s| self.cpu.encode(s)));
            //try!(s.emit_struct_field("retrogl", 1,
            //                         |s| self.retrogl.encode(s)));
            try!(s.emit_struct_field("video_clock", 2,
                                     |s| self.video_clock.encode(s)));
            try!(s.emit_struct_field("shared_state", 3,
                                     |s| self.shared_state.encode(s)));

            Ok(())
        })
    }
}

/// Init function, guaranteed called only once (unlike `retro_init`)
fn init() {
    retrolog::init();
}

/// Called when a game is loaded and a new context must be built
fn load_game(disc: PathBuf) -> Option<Box<libretro::Context>> {
    info!("Loading {:?}", disc);

    Context::new(&disc).ok()
        .map(|c| Box::new(c) as Box<libretro::Context>)
}

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

fn init_variables() {
    CoreVariables::register();
}

// Precise FPS values for the video output for the given
// VideoClock. It's actually possible to configure the PlayStation GPU
// to output with NTSC timings with the PAL clock (and vice-versa)
// which would make this code invalid but it wouldn't make a lot of
// sense for a game to do that.
fn video_output_framerate(std: VideoClock) -> f32 {
    match std {
        // 53.690MHz GPU clock frequency, 263 lines per field,
        // 3413 cycles per line
        VideoClock::Ntsc => 59.81,
        // 53.222MHz GPU clock frequency, 314 lines per field,
        // 3406 cycles per line
        VideoClock::Pal => 49.76,
    }
}

fn get_av_info(std: VideoClock, upscaling: u32) -> libretro::SystemAvInfo {

    // Maximum resolution supported by the PlayStation video
    // output is 640x480
    let max_width = (640 * upscaling) as c_uint;
    let max_height = (480 * upscaling) as c_uint;

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

/// Libretro to PlayStation button mapping. Libretro's mapping is
/// based on the SNES controller so libretro's A button matches the
/// PlayStation's Circle button.
const BUTTON_MAP: [(libretro::JoyPadButton, Button); 14] =
    [(libretro::JoyPadButton::Up, Button::DUp),
     (libretro::JoyPadButton::Down, Button::DDown),
     (libretro::JoyPadButton::Left, Button::DLeft),
     (libretro::JoyPadButton::Right, Button::DRight),
     (libretro::JoyPadButton::Start, Button::Start),
     (libretro::JoyPadButton::Select, Button::Select),
     (libretro::JoyPadButton::A, Button::Circle),
     (libretro::JoyPadButton::B, Button::Cross),
     (libretro::JoyPadButton::Y, Button::Square),
     (libretro::JoyPadButton::X, Button::Triangle),
     (libretro::JoyPadButton::L, Button::L1),
     (libretro::JoyPadButton::R, Button::R1),
     (libretro::JoyPadButton::L2, Button::L2),
     (libretro::JoyPadButton::R2, Button::R2)];

/// Number of output frames over which the internal FPS is averaged
const INTERNAL_FPS_SAMPLE_PERIOD: u32 = 32;

/// Hardcoded path for the generated VCD file when tracing is
/// enabled. XXX Should probably be changed for Windows, maybe made
/// configurable somehow?
const VCD_TRACE_PATH: &'static str = "/tmp/rustation-trace.vcd";
