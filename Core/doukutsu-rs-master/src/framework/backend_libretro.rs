use std::any::Any;
use std::cell::{RefCell, UnsafeCell};
use std::ffi::c_void;
use std::io::Read;
use std::mem;
use std::rc::Rc;
use std::sync::Arc;
use std::vec::Vec;


//new libretro stuff (copied from example)
use libretro_rs::c_utf8::c_utf8;
use libretro_rs::retro::env::{Init, UnloadGame};
use libretro_rs::retro::pixel::{Format, XRGB8888};
//log conflicts, we need to explicitly include everything
//use libretro_rs::retro::*;
use libretro_rs::retro::{av, cores, device, env, error, fs, game, log as retro_log, mem as retro_mem, str};
use libretro_rs::retro::av::*;
use libretro_rs::retro::cores::*;
use libretro_rs::{ext, libretro_core};

use imgui::{DrawData, TextureId, Ui};

use crate::common::{Color, Rect};
use crate::framework::backend::{
    Backend, BackendEventLoop, BackendRenderer, BackendShader, BackendTexture, SpriteBatchCommand, VertexData,
};
use crate::framework::context::Context;
use crate::framework::error::{GameResult, GameError};
use crate::framework::graphics::BlendMode;


//gl stuff
use crate::framework::render_opengl::{GLContext, OpenGLRenderer};
use crate::framework::gl;

use crate::game::shared_game_state::SharedGameState;
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
    //the stuff in run() before the loop
    // pub fn init(&mut self)
    // {
    //     if let (Some(ctx), Some(game), Some(state_ref)) = (self.context.as_mut(), self.game.as_mut(), self.state_ref.as_mut())
    //     {
    //         *state_ref = unsafe { &mut *game.state.get() };
    //         ctx.screen_size = (640.0, 480.0);
    //         state_ref.handle_resize(ctx).unwrap();
    //     }
    // }

    //like run(), but called repeatedly
    pub fn update(&mut self, state_ref: &mut SharedGameState, game: &mut Game, ctx: &mut Context)
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

    } 

    //congruent to new_renderer, except it takes extra arguments
    pub fn new_renderer_nd(&self, ctx: *mut Context) -> GameResult<Box<dyn BackendRenderer>> {


        let mut imgui = imgui::Context::create();
        imgui.io_mut().display_size = [320.0, 240.0];
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
            // let refs = Rc::from_raw(*user_data as *mut RefCell<LibretroContext>);
            // {
            //     let refs = &mut *refs.as_ptr();//*refs.get();
            //     let cur_fb = (refs.get_current_framebuffer)();
            // }
            // *user_data = Rc::into_raw(refs) as *mut c_void;
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
        //Err(super::error::GameError::CommandLineError(("Not Done Yet!".to_owned())))//=>{log::error!("not done yet!")}
        Ok(Box::new(OpenGLRenderer::new(gl_context, UnsafeCell::new(imgui))))

    }




}

//not really used, since there are many special functions inside the libretroEventLoop
impl BackendEventLoop for LibretroEventLoop {

    //called one time, normally loops indefinitely inside, but must return immeadiately for this core type (is unused here: see update())
    fn run(&mut self, _game: &mut Game, _ctx: &mut Context) {
        // let state_ref = unsafe { &mut *game.state.get() };

        // ctx.screen_size = (640.0, 480.0);
        // state_ref.handle_resize(ctx).unwrap();

        // loop {
        //     game.update(ctx).unwrap();

        //     if state_ref.shutdown {
        //         log::info!("Shutting down...");
        //         break;
        //     }

        //     if state_ref.next_scene.is_some() {
        //         mem::swap(&mut game.scene, &mut state_ref.next_scene);
        //         state_ref.next_scene = None;
        //         game.scene.as_mut().unwrap().init(state_ref, ctx).unwrap();
        //         game.loops = 0;
        //         state_ref.frame_time = 0.0;
        //     }
        //     std::thread::sleep(std::time::Duration::from_millis(10));

        //     game.draw(ctx).unwrap();
        // }


    }

    //initialize imgui renderer (and main renderer)
    fn new_renderer(&self, _ctx: *mut Context) -> GameResult<Box<dyn BackendRenderer>> {
        let mut imgui = imgui::Context::create();
        imgui.io_mut().display_size = [640.0, 480.0];
        imgui.fonts().build_alpha8_texture();

        return Ok(Box::new(LibretroRenderer(RefCell::new(imgui))));


        //load example:
        //let gl = gl::Gles2::load_with(|ptr| (gl_context.get_proc_address)(&mut gl_context.user_data, ptr));


        //function to use in order to refresh the buffer

        //these are responsible for turning a data dump over user_data into 
        unsafe fn get_proc_address(user_data: &mut *mut c_void, name: &str) -> *const c_void {
            let refs = Rc::from_raw(*user_data as *mut UnsafeCell<Option<LibretroEventLoop>>);

            let result = {
                let refs = &mut *refs.get();

                if let Some(refs) = refs {
                    //refs.get_proc_address(name)
                } else {
                    //std::ptr::null()
                }
                std::ptr::null()
            };

            *user_data = Rc::into_raw(refs) as *mut c_void;

            result
        }

        unsafe fn swap_buffers(user_data: &mut *mut c_void) {
            let refs = Rc::from_raw(*user_data as *mut UnsafeCell<Option<LibretroEventLoop>>);

            {
                let refs = &mut *refs.get();

                if let Some(refs) = refs {
                    //refs.swap_buffers();
                }
            }

            *user_data = Rc::into_raw(refs) as *mut c_void;
        }


    
        //let gl_context = GLContext { gles2_mode: true, is_sdl: false, get_proc_address, swap_buffers, user_data, ctx };
        //Err(super::error::GameError::CommandLineError(("Not Done Yet!".to_owned())))//=>{log::error!("not done yet!")}
        //Ok(Box::new(OpenGLRenderer::new(gl_context, UnsafeCell::new(imgui))))

    }

    fn as_any(&self) -> &dyn Any {
        self
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
