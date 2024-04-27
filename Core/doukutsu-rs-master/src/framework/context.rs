use crate::framework::backend::{BackendRenderer, init_backend};
use crate::framework::error::GameResult;
use crate::framework::filesystem::Filesystem;
use crate::framework::gamepad::GamepadContext;
use crate::framework::graphics::VSyncMode;
use crate::framework::keyboard::KeyboardContext;
use crate::game::Game;
use std::ffi::c_void;

use super::backend::Backend;
use super::backend::BackendEventLoop;

#[cfg(feature = "backend-libretro")]
use crate::framework::backend_libretro::{LibretroBackend, LibretroEventLoop};

pub struct Context {
    pub headless: bool,
    pub size_hint: (u16, u16),
    pub(crate) filesystem: Filesystem,
    pub(crate) renderer: Option<Box<dyn BackendRenderer>>,
    //pub renderer: Option<Box<dyn BackendRenderer>>,
    pub(crate) gamepad_context: GamepadContext,
    pub(crate) keyboard_context: KeyboardContext,
    pub(crate) real_screen_size: (u32, u32),
    pub(crate) screen_size: (f32, f32),
    pub(crate) screen_insets: (f32, f32, f32, f32),
    pub(crate) vsync_mode: VSyncMode,
}

impl Context {
    pub fn new() -> Context {
        Context {
            headless: false,
            size_hint: (640, 480),
            filesystem: Filesystem::new(),
            renderer: None,
            gamepad_context: GamepadContext::new(),
            keyboard_context: KeyboardContext::new(),
            real_screen_size: (320, 240),
            screen_size: (320.0, 240.0),
            screen_insets: (0.0, 0.0, 0.0, 0.0),
            vsync_mode: VSyncMode::Uncapped,
        }
    }

    pub fn run(&mut self, game: &mut Game) -> GameResult {
        let backend = init_backend(self.headless, self.size_hint)?; //don't need, backend just used for creating event loop
        let mut event_loop = backend.create_event_loop(self)?; //don't need, event loop is already created in a higher layer
        self.renderer = Some(event_loop.new_renderer(self as *mut Context)?); //do need, is used for imgui rendering

        event_loop.run(game, self);

        Ok(())
    }

    #[cfg(feature = "backend-libretro")]
    pub fn create_backend(&mut self, game: &mut Game,
        get_current_framebuffer: fn() -> usize,
        get_proc_address: fn(&str) -> *const c_void,
    ) -> GameResult<(Box<LibretroBackend>, Box<LibretroEventLoop>)> {
        // let backend = init_backend(self.headless, self.size_hint)?; //don't need, backend just used for creating event loop
        // let mut event_loop = backend.create_event_loop(self)?; //don't need, event loop is already created in a higher layer
        // self.renderer = Some(event_loop.new_renderer(self as *mut Context)?); //do need, is used for imgui rendering

        //force libretro type (no dyns)
        let backend = LibretroBackend::new_nd()?;
        let mut event_loop = backend.create_event_loop_nd(self, get_current_framebuffer, get_proc_address)?;
        self.renderer = Some(event_loop.new_renderer_nd(self as *mut Context)?);


        Ok((backend, event_loop))
    }

}
