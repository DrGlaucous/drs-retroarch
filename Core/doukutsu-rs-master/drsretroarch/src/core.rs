
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::pin::Pin;

use libc::{c_char, c_uint};
use std::ffi::c_void;


//use doukutsu_rs::framework::backend::BackendEventLoop;
use doukutsu_rs::framework::backend_libretro::{LibretroEventLoop, LibretroBackend};
use doukutsu_rs::framework::backend::{BackendEventLoop, Backend};
use doukutsu_rs::framework::context::{self, Context};
use doukutsu_rs::game::Game;
use doukutsu_rs::game::shared_game_state::SharedGameState;

use crate::libretro::{self, gl_frame_done};

/// Static system information sent to the frontend on request
pub const SYSTEM_INFO: libretro::SystemInfo = libretro::SystemInfo {
    library_name: cstring!("d-rs"),
    library_version: "-20" as *const _ as *const c_char,
    valid_extensions: cstring!("exe"),
    need_fullpath: false,
    block_extract: false,
};



////////////////////////BACKEND CALLS

/// Called when a game is loaded and a new context must be built
pub fn load_game(disc: PathBuf) -> Option<Box<dyn libretro::Context>> {
    log::info!("Loading {:?}", disc); //info!

    //todo: get disk into there
    Core::new().ok()
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
    let max_width = (640 * upscaling) as c_uint;
    let max_height = (240 * upscaling) as c_uint;

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


struct Core<'a>  {
    //runner backend and other loop pointers are in here
	backend: Box<LibretroBackend>,
	event_loop: Box<LibretroEventLoop>,
    ////data_path: PathBuf,

    state_ref: &'a mut SharedGameState,
    pub game: Pin<Box<Game>>,
    pub context: Pin<Box<Context>>,	

    has_set_res: bool,
}

impl<'a>  Core<'a>  {

    fn new() -> Result<Core<'a>, ()>{

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

        //function to use in order to get the current framebuffer
        let get_current_framebuffer: fn() -> usize = libretro::hw_context::get_current_framebuffer;
        let get_proc_address: fn(&str) -> *const c_void = libretro::hw_context::get_proc_address;



        let options = doukutsu_rs::game::LaunchOptions { server_mode: false, editor: false, return_types: true };
		let (game, context) = doukutsu_rs::game::init(options).unwrap();

		let mut bor_context = context.unwrap();
		let mut borrowed = game.unwrap();
		let nuvis = borrowed.as_mut().get_mut();


		let (a, mut b) = bor_context.create_backend(nuvis, get_current_framebuffer, get_proc_address).unwrap();

        let state_ref = unsafe {&mut *borrowed.state.get()};

        Ok(Core {
            backend: a,
            event_loop: b,
            ////data_path: data.clone().to_path_buf(), 

            state_ref: state_ref,
            game: borrowed,
            context: bor_context,
            has_set_res: false
        
        
        })
        
    }



}


impl<'a>  libretro::Context  for Core<'a>  {

    fn render_frame(&mut self) {


        let mut benders_shiny_metal_ass = 0;
        let frys_face = benders_shiny_metal_ass + 1;
        if frys_face & 1 > 0 {
            benders_shiny_metal_ass = 3;
        }

        if !self.has_set_res {
            let geometry = libretro::GameGeometry {
                base_width: 640 as c_uint,
                base_height: 480 as c_uint,
                // Max parameters are ignored by this call
                max_width: 0,
                max_height: 0,
                // Is this accurate?
                aspect_ratio: 4./3.,
            };

            libretro::set_geometry(&geometry);
            self.has_set_res = true;

            self.event_loop.init(self.state_ref, self.game.as_mut().get_mut(), &mut self.context);
        }




        self.event_loop.update(self.state_ref, self.game.as_mut().get_mut(), &mut self.context);



        gl_frame_done(640, 480)

    }

    fn get_system_av_info(&self) -> libretro::SystemAvInfo {
        let upscaling = 2 as u32;

        get_av_info(VideoClock::Pal, upscaling)
    }

    fn refresh_variables(&mut self){

    }

    fn reset(&mut self) {

    }

    fn gl_context_reset(&mut self){

    }

    fn gl_context_destroy(&mut self){

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


}




struct MinRender {

}

impl MinRender {

}

