use std::any::Any;
use std::cell::RefCell;
use std::mem;
use std::pin::Pin;
use std::rc::Rc;


//new libretro stuff (copied from example)
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
    Backend, BackendEventLoop, BackendRenderer, BackendShader, BackendTexture, SpriteBatchCommand, VertexData,
};
use crate::framework::context::Context;
use crate::framework::error::GameResult;
use crate::framework::graphics::BlendMode;
use crate::framework::filesystem;
//we will try to use the opengl renderer...
use crate::framework::gl;
use crate::framework::keyboard::ScanCode;
use crate::framework::render_opengl::{GLContext, OpenGLRenderer};

use crate::game::shared_game_state::SharedGameState;
use crate::game::GAME_SUSPENDED;
use crate::game::Game;

pub struct LibretroBackend;

impl LibretroBackend {
    pub fn new() -> GameResult<Box<dyn Backend>> {
        Ok(Box::new(LibretroBackend))
    }
    //special initializers without dynamic traits
    pub fn new_nd() -> GameResult<Box<LibretroBackend>> {
        Ok(Box::new(LibretroBackend))
    }
    pub fn create_event_loop_nd(&self, _ctx: &Context) -> GameResult<Box<LibretroEventLoop>> {
        Ok(LibretroEventLoop::new().unwrap())
    }

}

impl Backend for LibretroBackend {
    fn create_event_loop(&self, _ctx: &Context) -> GameResult<Box<dyn BackendEventLoop>> {
        Ok(LibretroEventLoop::new().unwrap())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}


//keyboard and gamepad inputs also go in here...
pub struct LibretroEventLoop {
    refs: Rc<RefCell<LibretroContext>>,
    opengl_available: RefCell<bool>,
}

//holds things like openGL renderer, keystrokes, and audio? (maybe?)
struct LibretroContext {
    //gl_context: Option<sdl2::video::GLContext>,
}

impl LibretroEventLoop {

    pub fn new() -> GameResult<Box<LibretroEventLoop>>
    {
        //subject to change!
        let opengl_available = if let Ok(v) = std::env::var("CAVESTORY_NO_OPENGL") { v != "1" } else { true };
        let event_loop = LibretroEventLoop {
            refs: Rc::new(RefCell::new(LibretroContext{
                //gl_context: None,

            })),
            opengl_available: RefCell::new(opengl_available),
        };

        Ok(Box::new(event_loop))
    }

    //the stuff in run() before the loop
    pub fn init(&mut self, state_ref: &mut SharedGameState, game: &mut Game, ctx: &mut Context)
    {
        ctx.screen_size = (640.0, 480.0);
        state_ref.handle_resize(ctx).unwrap();
    }

    //like run(), but called repeatedly
    /*
    pub fn update(&mut self, state_ref: &mut SharedGameState, game: &mut Game, ctx: &mut Context, callbacks: &mut impl Callbacks)
    {
        //let state_ref = unsafe { &mut *game.state.get() };

        game.update(ctx).unwrap();

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
    fn update_input(&mut self, callbacks: &mut impl Callbacks)
    {
        let inputs_polled = callbacks.poll_inputs();

    }*/
    

        



}

//not really used, since there are many special functions inside the libretroEventLoop
impl BackendEventLoop for LibretroEventLoop {

    //called one time, normally loops indefinitely inside, but must return immeadiately for this core type
    fn run(&mut self, game: &mut Game, ctx: &mut Context) {
        let state_ref = unsafe { &mut *game.state.get() };

        ctx.screen_size = (640.0, 480.0);
        state_ref.handle_resize(ctx).unwrap();

        loop {
            game.update(ctx).unwrap();

            if state_ref.shutdown {
                log::info!("Shutting down...");
                break;
            }

            if state_ref.next_scene.is_some() {
                mem::swap(&mut game.scene, &mut state_ref.next_scene);
                state_ref.next_scene = None;
                game.scene.as_mut().unwrap().init(state_ref, ctx).unwrap();
                game.loops = 0;
                state_ref.frame_time = 0.0;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));

            game.draw(ctx).unwrap();
        }


    }

    //initialize imgui renderer
    fn new_renderer(&self, _ctx: *mut Context) -> GameResult<Box<dyn BackendRenderer>> {



        let mut imgui = imgui::Context::create();
        imgui.io_mut().display_size = [640.0, 480.0];
        imgui.fonts().build_alpha8_texture();

        Ok(Box::new(LibretroRenderer(RefCell::new(imgui))))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

}

pub struct LibretroTexture(u16, u16);

impl BackendTexture for LibretroTexture {
    fn dimensions(&self) -> (u16, u16) {
        (self.0, self.1)
    }

    fn add(&mut self, _command: SpriteBatchCommand) {}

    fn clear(&mut self) {}

    fn draw(&mut self) -> GameResult<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct LibretroRenderer(RefCell<imgui::Context>);

//actually puts the stuff onto the screen, 
//render_opengl creates the textures beforehand
impl BackendRenderer for LibretroRenderer {
    fn renderer_name(&self) -> String {
        "Retroarch".to_owned()
    }

    fn clear(&mut self, _color: Color) {}

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
