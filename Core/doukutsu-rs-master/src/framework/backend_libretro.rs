use std::any::Any;
use std::cell::{RefCell, UnsafeCell};
use std::ffi::c_void;
use std::io::Read;
use std::mem;
use std::rc::Rc;
use std::sync::Arc;
use std::vec::Vec;


// //new libretro stuff (copied from example)
// use libretro_rs::c_utf8::c_utf8;
// use libretro_rs::retro::env::{Init, UnloadGame};
// use libretro_rs::retro::pixel::{Format, XRGB8888};
// //log conflicts, we need to explicitly include everything
// //use libretro_rs::retro::*;
// use libretro_rs::retro::{av, cores, device, env, error, fs, game, log as retro_log, mem as retro_mem, str};
// use libretro_rs::retro::av::*;
// use libretro_rs::retro::cores::*;
// use libretro_rs::{ext, libretro_core};

use imgui::{DrawData, TextureId, Ui};

use crate::common::{Color, Rect};
use crate::framework::backend::{
    Backend, BackendEventLoop, BackendRenderer, BackendGamepad, BackendShader, BackendTexture, SpriteBatchCommand, VertexData,
};
use crate::framework::context::Context;
use crate::framework::error::{GameResult, GameError};
use crate::framework::gamepad::GamepadType;
use crate::framework::graphics::BlendMode;


//gl stuff
use crate::framework::render_opengl::{GLContext, OpenGLRenderer};
use crate::framework::gl;

use crate::game::shared_game_state::SharedGameState;
use crate::game::Game;

use super::keyboard::ScanCode;
use super::gamepad::Button;

pub struct LibretroBackend;

impl LibretroBackend {
    pub fn new() -> GameResult<Box<dyn Backend>> {
        Ok(Box::new(LibretroBackend))
    }
    //special initializers without dynamic traits
    pub fn new_nd() -> GameResult<Box<LibretroBackend>> {
        Ok(Box::new(LibretroBackend))
    }

    pub fn create_event_loop_nd(&self, _ctx: &Context,
        get_current_framebuffer: fn() -> usize,
        get_proc_address: fn(&str) -> *const c_void,
    ) -> GameResult<Box<LibretroEventLoop>> {
        Ok(LibretroEventLoop::new(get_current_framebuffer, get_proc_address).unwrap())
    }

}

impl Backend for LibretroBackend {
    fn create_event_loop(&self, _ctx: &Context) -> GameResult<Box<dyn BackendEventLoop>> {
        Err(GameError::CommandLineError(("This function should not be called with this backend!".to_owned())))

        //Ok(LibretroEventLoop::new().unwrap())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}


pub struct LibretroEventLoop {
    refs: Rc<RefCell<LibretroContext>>
}

//holds things like openGL renderer, keystrokes, and audio? (maybe?)
//is basically a datapack struct to feed info to various functions in the form of a void()
struct LibretroContext {
    get_current_framebuffer: fn() -> usize,
    get_proc_address: fn(&str) -> *const c_void,
}

impl LibretroEventLoop {

    pub fn new(
        get_current_framebuffer: fn() -> usize,
        get_proc_address: fn(&str) -> *const c_void,
    ) -> GameResult<Box<LibretroEventLoop>>
    {
        let event_loop = LibretroEventLoop {
            refs: Rc::new(RefCell::new(LibretroContext{
                get_current_framebuffer,
                get_proc_address
            }))
        };

        Ok(Box::new(event_loop))
    }


    //destroy the context's renderer (because the frontend's environment has changed)
    pub fn destroy_renderer(&self, state_ref: &mut SharedGameState, ctx: &mut Context) -> GameResult {
        ctx.renderer = None;

        //wipe all old textures
        state_ref.texture_set.unload_all();

        Ok(())
    }

    //called on init and whenever the frontend's environment has changed (immediately after destroy_renderer)
    pub fn rebuild_renderer(&self, state_ref: &mut SharedGameState, ctx: &mut Context, width: u32, height: u32) -> GameResult {
        ctx.renderer = Some(self.new_renderer(ctx)?);
        self.handle_resize(state_ref, ctx, width, height)
    }

    pub fn handle_resize(&self, state_ref: &mut SharedGameState, ctx: &mut Context, width: u32, height: u32) -> GameResult {
        ctx.screen_size = (width as f32, height as f32);
        
        if let Some(renderer) = &ctx.renderer {
            if let Ok(imgui) = renderer.imgui() {
                imgui.io_mut().display_size = [ctx.screen_size.0, ctx.screen_size.1];
            }
        }
        state_ref.handle_resize(ctx);

        Ok(())
    }


    //like run(), but called repeatedly
    pub fn update(&mut self, state_ref: &mut SharedGameState, game: &mut Game, ctx: &mut Context, micros: u64)
    {
        //let state_ref = unsafe { &mut *game.state.get() };

        game.update(ctx, micros).unwrap();

        if state_ref.shutdown {
            log::info!("Shutting down...");
            //TODO: tell core to halt execution
            return;
        }

        if state_ref.next_scene.is_some() {
            mem::swap(&mut game.scene, &mut state_ref.next_scene);
            state_ref.next_scene = None;
            game.scene.as_mut().unwrap().init(state_ref, ctx).unwrap();
            game.loops = 0;
            state_ref.frame_time = 0.0;
        }
        //std::thread::sleep(std::time::Duration::from_millis(10));

        match game.draw(ctx)
        {
            Ok(_)=>{},
            Err(e)=>{log::error!("{}", e)}
        }


    }

    //takes input from libretro callbacks and pushes it into the engine
    pub fn update_keys(&mut self, ctx: &mut Context, key_id: ScanCode, key_state: bool)
    {
        ctx.keyboard_context.set_key(key_id, key_state);
    } 
    pub fn update_gamepad(&mut self, ctx: &mut Context, id: u16, button_id: Button, button_state: bool)
    {
        ctx.gamepad_context.set_button(id as u32, button_id, button_state);
    }

    pub fn add_gamepad(&mut self,
        state_ref: &mut SharedGameState,
        ctx: &mut Context,
        id: u16,
        rumble_fn: Option<fn (controller_port: u32, effect: u16, strengh: u16) -> bool>,
        ) {
        log::info!("Connected gamepad: {} (ID: {})", "Retropad", id);

        let axis_sensitivity = state_ref.settings.get_gamepad_axis_sensitivity(id as u32);
        ctx.gamepad_context.add_gamepad(LibretroGamepad::new(id, rumble_fn), axis_sensitivity);
        ctx.gamepad_context.set_gamepad_type(id as u32, GamepadType::Virtual);
    }


}

//not really used, since there are many special functions inside the libretroEventLoop
impl BackendEventLoop for LibretroEventLoop {

    //run is unused. See update() instead
    fn run(&mut self, _game: &mut Game, _ctx: &mut Context) { }

    //initialize the renderers for imgui and main
    fn new_renderer(&self, ctx: *mut Context) -> GameResult<Box<dyn BackendRenderer>> {


        let mut imgui = imgui::Context::create();
        imgui.io_mut().display_size = [640.0, 480.0];
        imgui.fonts().build_alpha8_texture();


        //test
        //let mut benders_shiny_metal_ass = (self.refs.borrow().get_current_framebuffer)();
        //let frys_face = benders_shiny_metal_ass + 1;
        //return Ok(Box::new(LibretroRenderer(RefCell::new(imgui))));

        //turn refs into a raw pointer
        let refs = self.refs.clone();
        let user_data = Rc::into_raw(refs) as *mut c_void;

        //load example:
        //let gl = gl::Gles2::load_with(|ptr| (gl_context.get_proc_address)(&mut gl_context.user_data, ptr));


        //function to use in order to refresh the buffer

        //these are responsible for turning a data dump over user_data into addresses avalable to the backend
        unsafe fn get_proc_address(user_data: &mut *mut c_void, name: &str) -> *const c_void {
            //pull a struct out of user_data pointer
            let refs = Rc::from_raw(*user_data as *mut RefCell<LibretroContext>);

            let result = {
                let refs = &mut *refs.as_ptr();//*refs.get();

                (refs.get_proc_address)(name)
            };
            *user_data = Rc::into_raw(refs) as *mut c_void;


            //return result
            result
        }

        unsafe fn swap_buffers(_user_data: &mut *mut c_void) {
            //libretro doesn't use this: do nothing
        }

        unsafe fn get_current_buffer(user_data: &mut *mut c_void) -> usize {
            let refs = Rc::from_raw(*user_data as *mut RefCell<LibretroContext>);

            let cur_fb: usize;
            {
                let refs = &mut *refs.as_ptr();//*refs.get();

                cur_fb = (refs.get_current_framebuffer)()
            }

            *user_data = Rc::into_raw(refs) as *mut c_void;
            cur_fb
        }


        let gl_context = GLContext { gles2_mode: false, is_sdl: false, get_proc_address, swap_buffers, get_current_buffer, user_data, ctx };
        //let gl_context = GLContext { gles2_mode: false, is_sdl: false, get_proc_address, swap_buffers, user_data, ctx };

        //Err(super::error::GameError::CommandLineError(("Not Done Yet!".to_owned())))//=>{log::error!("not done yet!")}
        Ok(Box::new(OpenGLRenderer::new(gl_context, UnsafeCell::new(imgui))))

    }

    fn as_any(&self) -> &dyn Any {
        self
    }

}


struct LibretroGamepad {
    id: u16,
    rumble_fn: Option<fn (controller_port: u32, effect: u16, strengh: u16) -> bool>,
}

impl LibretroGamepad {
    pub fn new(id: u16, rumble_fn: Option<fn (_: u32, _: u16, _: u16) -> bool>) -> Box<dyn BackendGamepad> {
        Box::new(LibretroGamepad {
            id,
            rumble_fn,
        })
    }
}

impl BackendGamepad for LibretroGamepad {

    fn set_rumble(&mut self, low_freq: u16, high_freq: u16, duration_ms: u32) -> GameResult {
        
        //todo: MAKE IT STOP!!!!!
        if let Some(rumble_fn) = self.rumble_fn{
            let _ = rumble_fn(self.id as u32, 0, low_freq);
            let _ = rumble_fn(self.id as u32, 1, high_freq);
        }

        Ok(())
    }

    fn instance_id(&self) -> u32 {
        self.id as u32
    }

}

//todo: fallback software renderer (not opengl)
//actually puts the stuff onto the screen, 
//render_opengl creates the textures beforehand
pub struct LibretroTexture(u16, u16);

impl BackendTexture for LibretroTexture {

    //get dimensions of texture
    fn dimensions(&self) -> (u16, u16) {
        (self.0, self.1)
    }

    //add a set of rects to be rendered?
    fn add(&mut self, _command: SpriteBatchCommand) {

        let (tex_scale_x, tex_scale_y) = (1.0 / self.0 as f32, 1.0 / self.1 as f32);






    }

    fn clear(&mut self) {}

    fn draw(&mut self) -> GameResult<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct LibretroRenderer(RefCell<imgui::Context>);


impl BackendRenderer for LibretroRenderer {
    fn renderer_name(&self) -> String {
        "Retroarch".to_owned()
    }

    fn clear(&mut self, _color: Color) {



    }

    fn present(&mut self) -> GameResult {
        Ok(())
    }

    fn create_texture_mutable(&mut self, width: u16, height: u16) -> GameResult<Box<dyn BackendTexture>> {
        Ok(Box::new(LibretroTexture(width, height)))
    }

    fn create_texture(&mut self, width: u16, height: u16, _data: &[u8]) -> GameResult<Box<dyn BackendTexture>> {
        Ok(Box::new(LibretroTexture(width, height)))
    }

    fn set_blend_mode(&mut self, _blend: BlendMode) -> GameResult {
        Ok(())
    }

    fn set_render_target(&mut self, _texture: Option<&Box<dyn BackendTexture>>) -> GameResult {
        Ok(())
    }

    fn draw_rect(&mut self, _rect: Rect<isize>, _color: Color) -> GameResult {
        Ok(())
    }

    fn draw_outline_rect(&mut self, _rect: Rect<isize>, _line_width: usize, _color: Color) -> GameResult {
        Ok(())
    }

    fn set_clip_rect(&mut self, _rect: Option<Rect>) -> GameResult {
        Ok(())
    }

    fn imgui(&self) -> GameResult<&mut imgui::Context> {
        unsafe { Ok(&mut *self.0.as_ptr()) }
    }

    fn imgui_texture_id(&self, _texture: &Box<dyn BackendTexture>) -> GameResult<TextureId> {
        Ok(TextureId::from(0))
    }

    fn prepare_imgui(&mut self, _ui: &Ui) -> GameResult {
        Ok(())
    }

    fn render_imgui(&mut self, _draw_data: &DrawData) -> GameResult {
        Ok(())
    }

    fn draw_triangle_list(
        &mut self,
        _vertices: &[VertexData],
        _texture: Option<&Box<dyn BackendTexture>>,
        _shader: BackendShader,
    ) -> GameResult<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
