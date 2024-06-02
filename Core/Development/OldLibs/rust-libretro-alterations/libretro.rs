//use crate::*;

use std::borrow::Borrow;
use std::pin::Pin;
use std::error::Error;
use std::ffi::CString;

use doukutsu_rs::framework::backend_libretro::{LibretroEventLoop, LibretroBackend};
use doukutsu_rs::framework::backend::{BackendEventLoop, Backend};
use doukutsu_rs::framework::context::Context;
use doukutsu_rs::game::Game;
use doukutsu_rs::game::shared_game_state::SharedGameState;

use rust_libretro::{
    contexts::*, core::Core, env_version, input_descriptors, proc::*, retro_core, sys::*, types::*,
};



pub const WIDTH: usize = 128;//64;
pub const HEIGHT: usize = 64;//32;
pub const AREA: usize = WIDTH * HEIGHT;
pub const SAMPLE_FREQUENCY: usize = 44_100;
pub const AUDIO_BUFFER_SIZE: usize = SAMPLE_FREQUENCY / 60;
//pub const AUDIO_FREQUENCY: f64 = 440.0;
//pub const RATE: f64 = (std::f64::consts::TAU * AUDIO_FREQUENCY) / (SAMPLE_FREQUENCY as f64);



// pub struct LibretroCore {
// 	//cpu: cpu::Cpu,
// 	game: Pin<Box<dGame>>,
// 	context: Pin<Box<dContext>>,
// 	audio_buffer: [i16; AUDIO_BUFFER_SIZE * 2],
// 	frame_buffer: [XRGB8888; AREA],
// 	rendering_mode: SoftwareRenderEnabled,
// 	pixel_format: Format<XRGB8888>,
// }

//holds all the refrences to the game itself, put in a wrapper so i don't have to option<> everything
pub struct BackendWrapper<'a> {
	backend: Box<LibretroBackend>,
	event_loop: Box<LibretroEventLoop>,
    pub game: Pin<Box<Game>>,
    pub context: Pin<Box<Context>>,	
	state_ref: &'a mut SharedGameState,


	//old stuff
	//cpu: cpu::Cpu,
	// pub audio_buffer: [i16; AUDIO_BUFFER_SIZE * 2],
	// pub frame_buffer: [XRGB8888; AREA],
	// pub rendering_mode: SoftwareRenderEnabled,
	// pub pixel_format: Format<XRGB8888>,

}

impl<'a> BackendWrapper<'a> {

	/*
	    //values don't matter too much here, since we configure these at the end of load_game, which is the only place where this should be called
		// pub fn new() -> LibretroCore
		// {
		// 	(Self {
		// 		rendering_mode: None,
		// 		pixel_format: None,
		// 		audio_buffer: [0; AUDIO_BUFFER_SIZE * 2],
		// 		frame_buffer: [XRGB8888::DEFAULT; AREA],
		// 		game: None,
		// 		context: None,
		// 	})
		// }
	
		// pub fn render_audio(&mut self, runtime: &mut impl Callbacks) {
		// 	// self.cpu.timer.wave(|n, val| {
		// 	//   self.audio_buffer[(n * 2) + 0] = (val * 32767.0).clamp(-32768.0, 32767.0) as i16;
		// 	//   self.audio_buffer[(n * 2) + 1] = (val * 32767.0).clamp(-32768.0, 32767.0) as i16;
		// 	// });
	
		// 	//null-filled audio buffer
		// 	for n in 0..AUDIO_BUFFER_SIZE
		// 	{
		// 		self.audio_buffer[n] = 0 as i16;
		// 	}
	
		// 	runtime.upload_audio_frame(&self.audio_buffer);
		// }
		// pub fn render_video(&mut self, callbacks: &mut impl Callbacks) {
		// 	const PIXEL_SIZE: usize = 1;
		// 	const PITCH: usize = PIXEL_SIZE * WIDTH;
		// 	for y in 0..HEIGHT {
		// 		for x in 0..WIDTH {
		// 			//let color = self.cpu.display.pixel(x, y).into();
		// 			let index = (y * PITCH) + (x * PIXEL_SIZE);
		// 			//bodge O-O-R catcher
		// 			if index >= self.frame_buffer.len()
		// 			{
		// 				continue;
		// 			}				
		// 			// if x > 3 && y > 2
		// 			// {
		// 			//   self.set_rgb(index, XRGB8888::new_with_raw_value(0x0000FF));
		// 			// }
		// 			// else
		// 			// {
		// 			//   self.set_rgb(index, color);
		// 			// }
		// 			let color = 0x4F50C0 | (y << 2 | x ) as u32;
		// 			self.set_rgb(index, XRGB8888::new_with_raw_value(color));
		// 		}
		// 	}
	
	
		// 	//let yuo = self.rendering_mode.as_ref().unwrap();
		// 	//let yy = self.pixel_format.as_ref().unwrap();
		// 	let yuo = &self.rendering_mode;
		// 	let yy = &self.pixel_format;
	
		// 	let width = WIDTH as u32;
		// 	let height = HEIGHT as u32;
		// 	let frame = Frame::new(&self.frame_buffer, width, height);
		// 	callbacks.upload_video_frame(yuo, yy, &frame);


		// }
	
		// fn set_rgb(&mut self, index: usize, color: XRGB8888) {
		// 	self.frame_buffer[index] = color;
		// }
	
		// pub fn update_input(&mut self, runtime: &mut impl Callbacks) -> InputsPolled {
		// 	let inputs_polled = runtime.poll_inputs();
	
		// 	// for key in keyboard::Keyboard::keys() {
		// 	//   // todo: chip-8 has a very clunky mapping to a controller.
	
		// 	//   let port = DevicePort::new(0);
		// 	//   let btn = key_to_retro_button(key);
		// 	//   if runtime.is_joypad_button_pressed(port, btn) {
		// 	//     self.cpu.keyboard.set_key_state(key, KeyState::Pressed)
		// 	//   } else {
		// 	//     self.cpu.keyboard.set_key_state(key, KeyState::Released)
		// 	//   }
		// 	// }
	
		// 	inputs_polled
		// }

	 */




}

/*
impl<'a> BackendWrapper<'a> {
//impl<'a> Core<'a> for LibretroCore<'a> {
	type Init = ();

	fn get_system_info() -> SystemInfo {
		SystemInfo::new(
			c_utf8!("doukutsu.rs"),
			c_utf8!(env!("CARGO_PKG_VERSION")),
			ext!["png"],
		)
	}

	fn init(_env: &mut impl Init) -> Self::Init {
		()
	}

	//this is the important init() section, since it has access to the ROM data at this point
	fn load_game<E: env::LoadGame>(
		_game: &GameInfo,
		args: LoadGameExtraArgs<'a, '_, E, Self::Init>,
	) -> Result<Self, CoreError> {
		let LoadGameExtraArgs {
			env,
			pixel_format,
			rendering_mode,
			..
		} = args;

		let pixel_format = env.set_pixel_format_xrgb8888(pixel_format)?;
		//let data: &[u8] = game.as_data().ok_or(CoreError::new())?.data();
		

		let options = doukutsu_rs::game::LaunchOptions { server_mode: false, editor: false, return_types: true };
		let (game, context) = doukutsu_rs::game::init(options).unwrap();

		//let event_loop = Box::new(LibretroEventLoop);

		//skip for now... (need a way to set the backend renderer )
		// if let Some(context2) = context.as_mut()
		// {
		// 	//self.renderer = Some(event_loop.new_renderer(self as *mut Context)?); 
		// 	//context2.set_renderer(Some(event_loop.new_renderer()).unwrap()))?;
		// 	context2.renderer = event_loop.new_renderer(**context2.as_mut());
		// }
		let mut bor_context = context.unwrap();
		let mut borrowed = game.unwrap();
		let nuvis = borrowed.as_mut().get_mut();

		let (a, mut b) = bor_context.create_backend(nuvis).unwrap();

		let state_ref = unsafe {&mut *borrowed.state.get()};

		b.init(state_ref, borrowed.as_mut().get_mut(), &mut bor_context);

		//return new emulator
		Ok(Self {
			rendering_mode: rendering_mode,
			pixel_format: pixel_format,
			audio_buffer: [0; AUDIO_BUFFER_SIZE * 2],
			frame_buffer: [XRGB8888::DEFAULT; AREA],
			game: borrowed,
			context: bor_context,
			event_loop: b,
			backend: a,
			state_ref,
		}
		)
	}

	fn get_system_av_info(&self, _env: &mut impl env::GetAvInfo) -> SystemAVInfo {
		const WINDOW_SCALE: u16 = 8;
		const WINDOW_WIDTH: u16 = WINDOW_SCALE * WIDTH as u16;
		const WINDOW_HEIGHT: u16 = WINDOW_SCALE * HEIGHT as u16;
		SystemAVInfo::default_timings(GameGeometry::fixed(WINDOW_WIDTH, WINDOW_HEIGHT))
	}

	//this is what should go in the "main loop" of the program, in loop{}
	fn run(&mut self, _env: &mut impl env::Run, callbacks: &mut impl Callbacks) -> InputsPolled {
		let inputs_polled = self.update_input(callbacks);


		self.event_loop.update(self.state_ref, self.game.as_mut().get_mut(), &mut self.context, callbacks);

		//self.cpu.step_for(25);

		self.render_audio(callbacks);
		self.render_video(callbacks);
		inputs_polled
	}

	fn reset(&mut self, _env: &mut impl env::Reset) {
		todo!()
		//send the game state back to the title screen here
	}

	fn unload_game(self, _env: &mut impl UnloadGame) -> Self::Init {
		//kill the backend if unload is requested
		self.state_ref.shutdown();
		()
	}
}

 */


// impl From<display::Pixel> for XRGB8888 {
//   fn from(pixel: display::Pixel) -> Self {
//     match pixel {
//       display::Pixel::Off => XRGB8888::DEFAULT,
//       display::Pixel::On => XRGB8888::new_with_raw_value(0x00FFFFFF),
//     }
//   }
// }


//human readable text to help with bindings
const INPUT_DESCRIPTORS: &[retro_input_descriptor] = &input_descriptors!(
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_UP, "Up" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_DOWN, "Down" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_LEFT, "Left" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_RIGHT, "Right" },
    { 0, RETRO_DEVICE_JOYPAD, 0, RETRO_DEVICE_ID_JOYPAD_A, "Action" },
);


//maifest options for settings relating to the core
#[derive(CoreOptions)]
#[categories({
    "advanced_settings",
    "Advanced",
    "Options affecting low-level emulation performance and accuracy."
},{
    "not_so_advanced_settings",
    "Not So Advanced",
    "Options not affecting low-level emulation performance and accuracy."
})]
#[options({
    "foo_option_1",
    "Advanced > Speed hack coprocessor X",
    "Speed hack coprocessor X",
    "Setting 'Advanced > Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
    "Setting 'Speed hack coprocessor X' to 'true' or 'Turbo' provides increased performance at the expense of reduced accuracy",
    "advanced_settings",
    {
        { "false" },
        { "true" },
        { "unstable", "Turbo (Unstable)" },
    }
}, {
    "foo_option_2",
    "Simple > Toggle Something",
    "Toggle Something",
    "Setting 'Simple > Toggle Something' to 'true' does something.",
    "Setting 'Toggle Something' to 'true' does something.",
    "not_so_advanced_settings",
    {
        { "false" },
        { "true" },
    }
})]
struct LibretroCore<'a> {
	backend: Option<BackendWrapper<'a>>,
	//option selection placeholders
	option_1: bool,
    option_2: bool,

	pixels: Vec<u8>,
    timer: i64,
    even: bool,
}



retro_core!(LibretroCore {
    backend: None,
	option_1: true,
    option_2: true,

	pixels: vec![0; 800 * 600 * 4],
    timer: 5_000_001,
    even: true,
});


impl<'a> Core for LibretroCore<'a> {

	//required #1:
    //returns info about the core to the system
    fn get_info(&self) -> SystemInfo {
        SystemInfo {
            library_name: CString::new("Example Core").unwrap(),
            library_version: CString::new(env_version!("CARGO_PKG_VERSION").to_string()).unwrap(),
            valid_extensions: CString::new("").unwrap(),

            need_fullpath: false,
            block_extract: false,
        }
    }

	//required #2:
	//tells the frontend what type of screen to make
	fn on_get_av_info(&mut self, _ctx: &mut GetAvInfoContext) -> retro_system_av_info {
		retro_system_av_info {
			geometry: retro_game_geometry {
				base_width: 800,
				base_height: 600,
				max_width: 800,
				max_height: 600,
				aspect_ratio: 0.0,
			},
			timing: retro_system_timing {
				fps: 60.0,
				sample_rate: 0.0,
			},
		}
	}

	//when core is selected? not sure when this is called...
	fn on_set_environment(&mut self, initial: bool, ctx: &mut SetEnvironmentContext) {
		if !initial {
			return;
		}

		ctx.set_support_no_game(true);
	}

	//or this... (think this happens when the core is selected initially)
	fn on_init(&mut self, ctx: &mut InitContext) {

		let gctx: GenericContext = ctx.into();
		//pass input initializers back to the frontend
		gctx.set_input_descriptors(INPUT_DESCRIPTORS);
	}

	//callback when settings from the frontend have changed (see json at top of page)
	fn on_options_changed(&mut self, ctx: &mut OptionsChangedContext) {
		match ctx.get_variable("foo_option_1") {
			Some("true") => self.option_1 = true,
			Some("false") => self.option_1 = false,
			_ => (),
		}

		match ctx.get_variable("foo_option_2") {
			Some("true") => self.option_2 = true,
			Some("false") => self.option_2 = false,
			_ => (),
		}
	}
	
	//when a core is loaded in ("true" init)
	fn on_load_game(
		&mut self,
		_info: Option<retro_game_info>,
		ctx: &mut LoadGameContext,
	) -> Result<(), Box<dyn std::error::Error>> {
		ctx.set_pixel_format(PixelFormat::XRGB8888);
		ctx.set_performance_level(0);
		//set microseconds between each execution, the standalone program has the thread sleep for 10 milliseconds,
		//but we might be able to hook into the timing logic to use this instead. This will allow frame stepping or fast-forward on the core
		//ctx.enable_frame_time_callback((1000000.0f64 / 60.0).round() as retro_usec_t);
		ctx.enable_frame_time_callback(std::time::Duration::from_millis(10).as_micros() as retro_usec_t);
		
		
		//audio callbacks (good for audio backend, since it depends on separate callbacks)
		let gctx: GenericContext = ctx.into();
		gctx.enable_audio_callback();


		//setup the backend wrapper
		let options = doukutsu_rs::game::LaunchOptions { server_mode: false, editor: false, return_types: true };
		let (game, context) = doukutsu_rs::game::init(options).unwrap();

		let mut bor_context = context.unwrap();
		let mut bor_game = game.unwrap();
		let game_address = bor_game.as_mut().get_mut();

		let (backend, mut event_loop) = bor_context.create_backend(game_address).unwrap();

		let state_ref = unsafe {&mut *bor_game.state.get()};

		event_loop.init(state_ref, bor_game.as_mut().get_mut(), &mut bor_context);

		//create new backend
		self.backend = Some(BackendWrapper {
			game: bor_game,
			context: bor_context,
			event_loop,
			backend,
			state_ref,
		});


		Ok(())
	}


	#[inline]
    fn on_run(&mut self, ctx: &mut RunContext, delta_us: Option<i64>) {
        let gctx: GenericContext = ctx.into();

        self.timer += delta_us.unwrap_or(16_666);

        let input = unsafe { ctx.get_joypad_bitmask(0, 0) };

        if input.contains(JoypadState::START) && input.contains(JoypadState::SELECT) {
            return gctx.shutdown();
        }

        if !ctx.can_dupe() || self.timer >= 1_000_000 || input.contains(JoypadState::A) {
            self.timer = 0;
            self.even = !self.even;

            let width = 800u32;
            let height = 600u32;

            let color_a = if self.even { 0xFF } else { 0 };
            let color_b = !color_a;

            for (i, chunk) in self.pixels.chunks_exact_mut(4).enumerate() {
                let x = (i % width as usize) as f64 / width as f64;
                let y = (i / width as usize) as f64 / height as f64;

                let total = (50.0f64 * x).floor() + (37.5f64 * y).floor();
                let even = total as usize % 2 == 0;

                let color = if even { color_a } else { color_b };

                chunk.fill(color);
            }

            ctx.draw_frame(self.pixels.as_ref(), width, height, width as usize * 4);
        } else if ctx.can_dupe() {
            ctx.dupe_frame();
        }



    }


	//audion callback
	fn on_audio_set_state(&mut self, _enabled: bool) {
		
	}
	fn on_write_audio(&mut self, _ctx: &mut AudioContext) {
		
	}
	fn on_audio_buffer_status(&mut self, _active: bool, _occupancy: u32, _underrun_likely: bool) {
		
	}






}






