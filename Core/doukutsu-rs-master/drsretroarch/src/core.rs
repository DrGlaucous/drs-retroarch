
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
use doukutsu_rs::framework::keyboard::ScanCode;
use doukutsu_rs::framework::context::{self, Context};
use doukutsu_rs::game::Game;
use doukutsu_rs::game::shared_game_state::SharedGameState;

use crate::libretro::{self, gl_frame_done, Key, key_pressed};

/// Static system information sent to the frontend on request
pub const SYSTEM_INFO: libretro::SystemInfo = libretro::SystemInfo {
    library_name: cstring!("d-rs"),
    library_version: "-20" as *const _ as *const c_char,
    valid_extensions: cstring!("exe"),
    need_fullpath: false,
    block_extract: false,
};

pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 240; 

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


//get the current state of the backend's video settings (placeholder for now...)
fn get_av_info(fps: f32, upscaling: u32) -> libretro::SystemAvInfo {

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
            aspect_ratio: (max_width as f32)/(max_height as f32),
        },
        timing: libretro::SystemTiming {
            fps: fps as f64,
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


		let (a, b) = bor_context.create_backend(nuvis, get_current_framebuffer, get_proc_address).unwrap();

        let state_ref = unsafe {&mut *borrowed.state.get()};

        Ok(Core {
            backend: a,
            event_loop: b,
            context: bor_context,

            state_ref: state_ref,
            game: borrowed,
            has_set_res: false

            ////data_path: data.clone().to_path_buf(), 
        
        
        })
        
    }

    fn poll_keys(&mut self) {
        
        for (ret_key, drs_key) in BUTTON_MAP {
            self.event_loop.update_input(&mut self.context, drs_key, key_pressed(0, ret_key));
        }
    }

}


impl<'a>  libretro::Context  for Core<'a>  {

    fn render_frame(&mut self) {


        self.poll_keys();


        // let mut benders_shiny_metal_ass = 0;
        // let frys_face = benders_shiny_metal_ass + 1;
        // if frys_face & 1 > 0 {
        //     benders_shiny_metal_ass = 3;
        // }

        // if !self.has_set_res {
        //     let geometry = libretro::GameGeometry {
        //         base_width: WIDTH as c_uint,
        //         base_height: HEIGHT as c_uint,
        //         // Max parameters are ignored by this call
        //         max_width: 0,
        //         max_height: 0,
        //         // Is this accurate?
        //         aspect_ratio: (WIDTH as f32)/(HEIGHT as f32),
        //     };
        //     libretro::set_geometry(&geometry);
        //     self.has_set_res = true;
        //     //self.event_loop.init(self.state_ref, self.game.as_mut().get_mut(), &mut self.context);
        // }




        self.event_loop.update(self.state_ref, self.game.as_mut().get_mut(), &mut self.context);



        gl_frame_done(WIDTH, HEIGHT)

    }

    //tell frontend what audio and video parameters to use
    fn get_system_av_info(&self) -> libretro::SystemAvInfo {
        let upscaling = 2 as u32;

        get_av_info(60.0, upscaling)
    }

    //settings have been changed, update them inside the game
    fn refresh_variables(&mut self){

    }

    //soft-reset (gl is not re-initialized, send game back to top menu)
    fn reset(&mut self) {

    }

    //gl context was destroyed, now rebuild it (called when game is initialized).
    fn gl_context_reset(&mut self){
        let _ = self.event_loop.rebuild_renderer(self.state_ref, &mut self.context, WIDTH, HEIGHT);
    }

    //called when frontend window resolution is changed,
    //the gl context is about to be destroyed, remove anything from the back while you can
    fn gl_context_destroy(&mut self){
         let _ = self.event_loop.destroy_renderer(&mut self.state_ref, &mut self.context);
    }

    //todo: remove unused functions from Context
    fn serialize_size(&self) -> usize {
        0
    }
    fn serialize(&self, mut _buf: &mut [u8]) -> Result<(), ()> {
        Ok(())
    }
    fn unserialize(&mut self, mut _buf: &[u8]) -> Result<(), ()> {
        Ok(())
    }


}


/////////////////////UTILS

//need this static array to iterate over the enum:
const BUTTON_MAP: [(Key, ScanCode); 101] = [
    (Key::A, ScanCode::A),
    (Key::B, ScanCode::B),
    (Key::C, ScanCode::C),
    (Key::D, ScanCode::D),
    (Key::E, ScanCode::E),
    (Key::F, ScanCode::F),
    (Key::G, ScanCode::G),
    (Key::H, ScanCode::H),
    (Key::I, ScanCode::I),
    (Key::J, ScanCode::J),
    (Key::K, ScanCode::K),
    (Key::L, ScanCode::L),
    (Key::M, ScanCode::M),
    (Key::N, ScanCode::N),
    (Key::O, ScanCode::O),
    (Key::P, ScanCode::P),
    (Key::Q, ScanCode::Q),
    (Key::R, ScanCode::R),
    (Key::S, ScanCode::S),
    (Key::T, ScanCode::T),
    (Key::U, ScanCode::U),
    (Key::V, ScanCode::V),
    (Key::W, ScanCode::W),
    (Key::X, ScanCode::X),
    (Key::Y, ScanCode::Y),
    (Key::Z, ScanCode::Z),
    (Key::Num1, ScanCode::Key1),
    (Key::Num2, ScanCode::Key2),
    (Key::Num3, ScanCode::Key3),
    (Key::Num4, ScanCode::Key4),
    (Key::Num5, ScanCode::Key5),
    (Key::Num6, ScanCode::Key6),
    (Key::Num7, ScanCode::Key7),
    (Key::Num8, ScanCode::Key8),
    (Key::Num9, ScanCode::Key9),
    (Key::Num0, ScanCode::Key0),
    (Key::Return, ScanCode::Return),
    (Key::Escape, ScanCode::Escape),
    (Key::Backspace, ScanCode::Backspace),
    (Key::Tab, ScanCode::Tab),
    (Key::Space, ScanCode::Space),
    (Key::Minus, ScanCode::Minus),
    (Key::Equals, ScanCode::Equals),
    (Key::LeftBracket, ScanCode::LBracket),
    (Key::RightBracket, ScanCode::RBracket),
    (Key::Backslash, ScanCode::Backslash),
    (Key::Semicolon, ScanCode::Semicolon),
    (Key::Comma, ScanCode::Comma),
    (Key::Period, ScanCode::Period),
    (Key::Slash, ScanCode::Slash),
    (Key::CapsLock, ScanCode::Capslock),
    (Key::F1, ScanCode::F1),
    (Key::F2, ScanCode::F2),
    (Key::F3, ScanCode::F3),
    (Key::F4, ScanCode::F4),
    (Key::F5, ScanCode::F5),
    (Key::F6, ScanCode::F6),
    (Key::F7, ScanCode::F7),
    (Key::F8, ScanCode::F8),
    (Key::F9, ScanCode::F9),
    (Key::F10, ScanCode::F10),
    (Key::F11, ScanCode::F11),
    (Key::F12, ScanCode::F12),
    (Key::Pause, ScanCode::Pause),
    (Key::Insert, ScanCode::Insert),
    (Key::Home, ScanCode::Home),
    (Key::PageUp, ScanCode::PageUp),
    (Key::Delete, ScanCode::Delete),
    (Key::End, ScanCode::End),
    (Key::PageDown, ScanCode::PageDown),
    (Key::Right, ScanCode::Right),
    (Key::Left, ScanCode::Left),
    (Key::Down, ScanCode::Down),
    (Key::Up, ScanCode::Up),
    (Key::KpDivide, ScanCode::NumpadDivide),
    (Key::KpMultiply, ScanCode::NumpadMultiply),
    (Key::KpMinus, ScanCode::NumpadSubtract),
    (Key::KpPlus, ScanCode::NumpadAdd),
    (Key::KpEnter, ScanCode::NumpadEnter),
    (Key::Kp1, ScanCode::Numpad1),
    (Key::Kp2, ScanCode::Numpad2),
    (Key::Kp3, ScanCode::Numpad3),
    (Key::Kp4, ScanCode::Numpad4),
    (Key::Kp5, ScanCode::Numpad5),
    (Key::Kp6, ScanCode::Numpad6),
    (Key::Kp7, ScanCode::Numpad7),
    (Key::Kp8, ScanCode::Numpad8),
    (Key::Kp9, ScanCode::Numpad9),
    (Key::Kp0, ScanCode::Numpad0),
    (Key::Power, ScanCode::Power),
    (Key::KpEquals, ScanCode::NumpadEquals),
    (Key::F13, ScanCode::F13),
    (Key::F14, ScanCode::F14),
    (Key::F15, ScanCode::F15),
    (Key::SysReq, ScanCode::Sysrq),
    (Key::LCtrl, ScanCode::LControl),
    (Key::LShift, ScanCode::LShift),
    (Key::LAlt, ScanCode::LAlt),
    (Key::RCtrl, ScanCode::RControl),
    (Key::RShift, ScanCode::RShift),
    (Key::RAlt, ScanCode::RAlt),
];
