//use crate::*;

use std::borrow::Borrow;
use std::pin::Pin;


//use doukutsu_rs::framework::backend::BackendEventLoop;
use doukutsu_rs::framework::backend_libretro::{LibretroEventLoop, LibretroBackend};
use doukutsu_rs::framework::backend::{BackendEventLoop, Backend};
use doukutsu_rs::framework::context::{self, Context};
use doukutsu_rs::game::Game;
use doukutsu_rs::game::shared_game_state::SharedGameState;

//use doukutsu_rs::framework::context;
//use crate::keyboard::KeyState;
use libretro_rs::c_utf8::c_utf8;
use libretro_rs::retro::env::{Init, UnloadGame};
use libretro_rs::retro::pixel::{Format, XRGB8888};
use libretro_rs::retro::av::GLContextType;
use libretro_rs::retro::*;
use libretro_rs::{ext, libretro_core};
use std::error::Error;



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

pub struct LibretroCore<'a> {
	backend: Box<LibretroBackend>,
	event_loop: Box<LibretroEventLoop>,

	//cpu: cpu::Cpu,
	pub audio_buffer: [i16; AUDIO_BUFFER_SIZE * 2],
	pub frame_buffer: [XRGB8888; AREA],
	pub rendering_mode: GLRenderEnabled, //SoftwareRenderEnabled,
	pub pixel_format: Format<XRGB8888>,
    pub game: Pin<Box<Game>>,
    pub context: Pin<Box<Context>>,	
	state_ref: &'a mut SharedGameState,



}


// fn key_to_retro_button(key: keyboard::Key) -> JoypadButton {
//   match key.ordinal() {
//     _ => JoypadButton::Up,
//   }
// }


impl<'a> LibretroCore<'a> {
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
	
		pub fn render_audio(&mut self, runtime: &mut impl Callbacks) {
			// self.cpu.timer.wave(|n, val| {
			//   self.audio_buffer[(n * 2) + 0] = (val * 32767.0).clamp(-32768.0, 32767.0) as i16;
			//   self.audio_buffer[(n * 2) + 1] = (val * 32767.0).clamp(-32768.0, 32767.0) as i16;
			// });

			// let mut samples = Vec::with_capacity(AUDIO_BUFFER_SIZE as usize * 2);
			// let d = AUDIO_BUFFER_SIZE as f32;
	
			// for _ in 0..AUDIO_BUFFER_SIZE as u64 / 60 {
			// 	let value = ((0x800 as f32)
			// 		* (2.0 * std::f32::consts::PI * (self.phase as f32) * 300.0 / d).sin())
			// 		as i16;
	
			// 	samples.push(value);
			// 	samples.push(value);
	
			// 	//self.phase += 1;
			// }
	
			//self.phase %= 100;
	
			//ctx.batch_audio_samples(&samples);

	
			//null-filled audio buffer
			for n in 0..AUDIO_BUFFER_SIZE
			{
				self.audio_buffer[n] = 0 as i16;
			}
	
			runtime.upload_audio_frame(&self.audio_buffer);

		}
	
		pub fn render_video(&mut self, callbacks: &mut impl Callbacks) {
			const PIXEL_SIZE: usize = 1;
			const PITCH: usize = PIXEL_SIZE * WIDTH;
	

			for y in 0..HEIGHT {
				for x in 0..WIDTH {
					//let color = self.cpu.display.pixel(x, y).into();
					let index = (y * PITCH) + (x * PIXEL_SIZE);
	
					//bodge O-O-R catcher
					if index >= self.frame_buffer.len()
					{
						continue;
					}
					
					// if x > 3 && y > 2
					// {
					//   self.set_rgb(index, XRGB8888::new_with_raw_value(0x0000FF));
					// }
					// else
					// {
					//   self.set_rgb(index, color);
					// }
					let color = 0x4F50C0 | (y << 2 | x ) as u32;
					self.set_rgb(index, XRGB8888::new_with_raw_value(color));
	
				}
			}
	
	
			//let yuo = self.rendering_mode.as_ref().unwrap();
			//let yy = self.pixel_format.as_ref().unwrap();
			let yuo = &self.rendering_mode;
			let yy = &self.pixel_format;
	
			let width = WIDTH as u32;
			let height = HEIGHT as u32;
			let frame = Frame::new(&self.frame_buffer, width, height);
			//callbacks.upload_video_frame(yuo, yy, &frame);


		}
	
		fn set_rgb(&mut self, index: usize, color: XRGB8888) {
			self.frame_buffer[index] = color;
		}
	
		pub fn update_input(&mut self, runtime: &mut impl Callbacks) -> InputsPolled {
			let inputs_polled = runtime.poll_inputs();
	
			// for key in keyboard::Keyboard::keys() {
			//   // todo: chip-8 has a very clunky mapping to a controller.
	
			//   let port = DevicePort::new(0);
			//   let btn = key_to_retro_button(key);
			//   if runtime.is_joypad_button_pressed(port, btn) {
			//     self.cpu.keyboard.set_key_state(key, KeyState::Pressed)
			//   } else {
			//     self.cpu.keyboard.set_key_state(key, KeyState::Released)
			//   }
			// }
	
			inputs_polled
		}
	
}

impl<'a> Core<'a> for LibretroCore<'a> {
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

		//let's use gles2, since the backend already supports it
		let gloptions = GLOptions::new(GLContextType::OpenGLES2);
		let rendering_mode = env.set_hw_render_gl(gloptions)?;

		let pixel_format = env.set_pixel_format_xrgb8888(pixel_format)?;
		//let data: &[u8] = game.as_data().ok_or(CoreError::new())?.data();
		

		let options = doukutsu_rs::game::LaunchOptions { server_mode: false, editor: false, return_types: true };
		let (game, context) = doukutsu_rs::game::init(options).unwrap();



		let mut bor_context = context.unwrap();
		let mut borrowed = game.unwrap();
		let nuvis = borrowed.as_mut().get_mut();

		let (a,b) = bor_context.create_backend(nuvis).unwrap();

		let state_ref = unsafe {&mut *borrowed.state.get()};

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

		//self.event_loop.update(self.state_ref, self.game.as_mut().get_mut(), &mut self.context, callbacks);

		//self.cpu.step_for(25);

		self.render_audio(callbacks);
		self.render_video(callbacks);
		inputs_polled
	}

	fn reset(&mut self, _env: &mut impl env::Reset) {
		todo!()
		//send game state back to title screen here.
	}

	fn unload_game(self, _env: &mut impl UnloadGame) -> Self::Init {
		()
	}




}


unsafe impl<'a> OpenGLCore<'a> for LibretroCore<'a> {

	fn context_destroy(&mut self, env: &mut impl env::Environment) {
		let myvar = 3;

	}
	fn context_reset(&mut self, env: &mut impl env::Environment, callbacks: GLContextCallbacks) {
		let myvar = 3;	

		let also = myvar + 2;	
	}

}



libretro_core!(crate::libretro::LibretroCore);

// impl From<display::Pixel> for XRGB8888 {
//   fn from(pixel: display::Pixel) -> Self {
//     match pixel {
//       display::Pixel::Off => XRGB8888::DEFAULT,
//       display::Pixel::On => XRGB8888::new_with_raw_value(0x00FFFFFF),
//     }
//   }
// }








#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InitCoreError;

impl<T: Error> From<T> for InitCoreError {
	fn from(_value: T) -> Self {
		Self
	}
}




