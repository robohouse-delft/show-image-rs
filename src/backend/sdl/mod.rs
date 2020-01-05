use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;

use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use crate::ImageData;
use crate::ImageInfo;
use crate::KeyState;
use crate::KeyboardEvent;
use crate::PixelFormat;
use crate::WindowOptions;
use crate::oneshot;

mod key_code;
mod key_location;
mod modifiers;
mod scan_code;

/// A context for creating windows.
///
/// The context runs an event loop in a background thread.
/// This context can be used to create windows and manage the background thread.
pub struct Context {
	/// Channel to send command to the background thread.
	command_tx: mpsc::SyncSender<ContextCommand>,

	/// Join handle for the background thread.
	_thread: std::thread::JoinHandle<Result<(), String>>,
}

/// A window capable of displaying images.
///
/// The handle can be used to set the displayed image,
/// handle key events and to close the window.
///
/// If the handle is dropped, the window is closed.
pub struct Window {
	/// The window ID.
	id: u32,

	/// Channel to send commands to the background thread.
	command_tx: mpsc::SyncSender<ContextCommand>,

	/// Channel to receive events from the background thread.
	event_rx: mpsc::Receiver<KeyboardEvent>,
}

/// Commands that can be sent to the context in the background thread.
enum ContextCommand {
	/// Create a window with the given options.
	CreateWindow(WindowOptions, mpsc::SyncSender<ContextCommand>, oneshot::Sender<Result<Window, String>>),

	/// Destroy a window.
	DestroyWindow(u32, oneshot::Sender<Result<(), String>>),

	/// Set the image of the window.
	SetImage(u32, Box<[u8]>, ImageInfo, oneshot::Sender<Result<(), String>>),
}

/// Inner context doing the real work in the background thread.
struct ContextInner {
	/// SDL2 video subsystem to create windows with.
	video: sdl2::VideoSubsystem,

	/// SDL2 event pump to handle events with.
	events: sdl2::EventPump,

	/// List of created windows.
	windows: Vec<WindowInner>,

	/// Channel to receive commands.
	command_rx: mpsc::Receiver<ContextCommand>,
}

/// Inner window doing the real work in the background thread.
struct WindowInner {
	/// The window ID, used to look up the window in the vector.
	id: u32,

	/// The canvas to draw the image on.
	canvas: Canvas<sdl2::video::Window>,

	/// A texture creator for the window.
	texture_creator: TextureCreator<sdl2::video::WindowContext>,

	/// A texture representing the current image to be drawn.
	texture: Option<(Texture<'static>, sdl2::rect::Rect)>,

	/// Channel to send keyboard events.
	event_tx: mpsc::SyncSender<KeyboardEvent>,
}

impl Context {
	/// Create a new context.
	///
	/// The context will spawn a background thread immediately.
	pub fn new() -> Result<Self, String> {
		let (command_tx, command_rx) = mpsc::sync_channel(10);
		let thread = std::thread::spawn(move || {
			let mut context = ContextInner::new(command_rx)?;
			context.run()
		});

		Ok(Context {
			command_tx,
			_thread: thread,
		})
	}

	/// Create a new window with the given options.
	pub fn make_window(&mut self, options: WindowOptions) -> Result<Window, String> {
		let (result_tx, result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::CreateWindow(options, self.command_tx.clone(), result_tx))
			.map_err(|e| format!("failed to send command to context thread: {}", e))?;
		result_rx.recv().map_err(|e| format!("context thread did not create a window: {}", e))?
	}

	// /// Create a new window with the default options.
	// pub fn make_window_defaults(&mut self, name: String) -> Result<Window, String> {
	// 	let options = WindowOptions { name, ..Default::default() };
	// 	self.make_window(options)
	// }

	// /// Close all windows, stop and join the background thread.
	// pub fn close(self) -> Result<(), String> {
	// 	// TODO: close all windows.
	// 	self.thread.join().map_err(|e| format!("failed to join context thread: {:?}", e))?
	// }
}

impl Window {
	/// Set the image to de displayed by the window.
	pub fn set_image(&self, image: &impl ImageData) -> Result<(), String> {
		let data = Box::from(image.data());
		let info = image.info().map_err(|e| format!("failed to display image: {}", e))?;

		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::SetImage(self.id, data, info, result_tx)).unwrap();
		result_rx.recv_timeout(Duration::from_millis(100))
			.map_err(|e| format!("failed to set image: {}", e))?
			.map_err(|e| format!("failed to set image: {}", e))
	}

	/// Close the window.
	///
	/// The window is automatically closed if the handle is dropped,
	/// but this function allows you to handle errors that may occur.
	pub fn close(mut self) -> Result<(), String> {
		self.close_impl()
	}

	/// Get the receiver for keyboard events.
	pub fn events(&self) -> &mpsc::Receiver<KeyboardEvent> {
		&self.event_rx
	}

	/// Wait for a key-down event with a timeout.
	///
	/// This function discards all key-up events, blocking until a key is pressed or the timeout occured.
	pub fn wait_key(&self, timeout: Duration) -> Option<KeyboardEvent> {
		self.wait_key_deadline(Instant::now() + timeout)
	}

	/// Wait for a key-down event with a dealine.
	///
	/// This function discards all key-up events, blocking until a key is pressed or the deadline passes.
	pub fn wait_key_deadline(&self, deadline: Instant) -> Option<KeyboardEvent> {
		loop {
			let now = Instant::now();
			if now <= deadline {
				return None;
			}
			let event = match self.events().recv_timeout(deadline - now) {
				Ok(x) => x,
				Err(_) => return None,
			};

			if event.state == KeyState::Down {
				return Some(event)
			}
		}
	}

	/// Close the window without dropping the handle.
	pub fn close_impl(&mut self) -> Result<(), String> {
		let (result_tx, result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::DestroyWindow(self.id, result_tx))
			.map_err(|e| format!("failed to send command to window: {}", e))?;
		result_rx.recv().map_err(|e| format!("error receiving response from window: {}", e))?
	}
}

/// Close the window when the handle is dropped.
impl Drop for Window {
	fn drop(&mut self) {
		let _ = self.close_impl();
	}
}

impl ContextInner {
	/// Create a new context.
	fn new(command_rx: mpsc::Receiver<ContextCommand>) -> Result<Self, String> {
		sdl2::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");
		let context = sdl2::init().map_err(|e| format!("failed to initialize SDL2: {}", e))?;
		let video = context.video().map_err(|e| format!("failed to get SDL2 video subsystem: {}", e))?;
		let events = context.event_pump().map_err(|e| format!("failed to get SDL2 event pump: {}", e))?;

		Ok(Self {
			video,
			events,
			windows: Vec::new(),
			command_rx,
		})
	}

	/// Run the event loop.
	fn run(&mut self) -> Result<(), String> {
		let delay = Duration::from_nanos(1_000_000_000 / 60);
		let mut next_frame = Instant::now() + delay;

		loop {
			self.run_one()?;

			// Sleep till the next scheduled frame.
			let now = Instant::now();
			if now < next_frame {
				std::thread::sleep(next_frame - now);
				next_frame += delay;
			} else {
				next_frame = now.max(next_frame + delay);
			}
		}
	}

	/// Run one iteration of the event loop.
	fn run_one(&mut self) -> Result<(), String> {
		// Handle all queued SDL events.
		while let Some(event) = self.events.poll_event() {
			self.handle_sdl_event(event)?;
		}

		// Handle all queued commands for the context.
		self.poll_commands();

		// Loop over all windows.
		for window in &mut self.windows {
			// Always clear the whole window, to avoid artefacts.
			window.canvas.clear();

			// Redraw the image, if any.
			let texture = window.texture.take();
			if let Some((texture, image_size)) = texture {
				let viewport = window.canvas.viewport();
				window.canvas.copy(&texture, image_size.clone(), viewport)
					.map_err(|e| format!("failed to copy data to window: {}", e))?;
				window.texture = Some((texture, image_size));
				window.canvas.window_mut().show();
			}

			window.canvas.present();
		}

		Ok(())
	}

	/// Handle an SDL2 event.
	fn handle_sdl_event(&mut self, event: Event) -> Result<(), String> {
		match event {
			Event::Window { window_id, win_event, .. } => {
				self.handle_sdl_window_event(window_id, win_event)
			},
			Event::KeyDown { window_id, keycode, scancode, keymod, repeat, .. } => {
				let event = convert_keyboard_event(KeyState::Down, keycode, scancode, keymod, repeat);
				self.handle_sdl_keyboard_event(window_id, event)
			},
			Event::KeyUp { window_id, keycode, scancode, keymod, repeat, .. } => {
				let event = convert_keyboard_event(KeyState::Up, keycode, scancode, keymod, repeat);
				self.handle_sdl_keyboard_event(window_id, event)
			},
			_ => Ok(()),
		}
	}

	/// Handle an SDL2 window event.
	fn handle_sdl_window_event(&mut self, window_id: u32, event: WindowEvent) -> Result<(), String> {
		match event {
			WindowEvent::Close => {
				self.destroy_window(window_id)?;
			},
			_ => (),
		}
		Ok(())
	}

	/// Handle an SDL2 keyboard event.
	fn handle_sdl_keyboard_event(&mut self, window_id: u32, event: KeyboardEvent) -> Result<(), String> {
		if let Some(window) = self.windows.iter().find(|x| x.id == window_id) {
			// Ignore errors, it likely means the receiver isn't handling events.
			let _ = window.event_tx.try_send(event);
		}
		Ok(())
	}

	/// Find a created window by ID.
	fn find_window_mut(&mut self, id: u32) -> Result<&mut WindowInner, String> {
		self.windows.iter_mut().find(|x| x.id == id)
			.ok_or_else(|| format!("failed to find window with ID {}", id))
	}

	/// Handle all queued commands.
	fn poll_commands(&mut self) {
		while let Ok(command) = self.command_rx.try_recv() {
			self.handle_command(command);
		}
	}

	/// Handle a single command.
	fn handle_command(&mut self, command: ContextCommand) {
		match command {
			ContextCommand::CreateWindow(options, command_tx, result_tx) => {
				result_tx.send(self.make_window(options, command_tx));
			},
			ContextCommand::DestroyWindow(id, result_tx) => {
				result_tx.send(self.destroy_window(id));
			}
			ContextCommand::SetImage(id, data, info, result_tx) => {
				let result = self.find_window_mut(id).and_then(|window| window.set_image(data, info));
				result_tx.send(result);
			}
		}
	}

	/// Create a new window.
	fn make_window(&mut self, options: WindowOptions, command_tx: mpsc::SyncSender<ContextCommand>) -> Result<Window, String> {
		let window = self.video.window(&options.name, options.size[0], options.size[1])
			.borderless()
			.resizable()
			.build()
			.map_err(|e| format!("failed to create window {:?}: {}", options.name, e))?;

		let id = window.id();
		let canvas = window.into_canvas().build().map_err(|e| format!("failed to create canvas for window {:?}: {}", options.name, e))?;
		let texture_creator = canvas.texture_creator();
		let (event_tx, event_rx) = mpsc::sync_channel(10);

		let inner = WindowInner {
			id,
			canvas,
			texture_creator,
			texture: None,
			event_tx,
		};

		self.windows.push(inner);

		Ok(Window { id, command_tx, event_rx })
	}

	/// Destroy a window by ID.
	fn destroy_window(&mut self, id: u32) -> Result<(), String> {
		let index = self.windows.iter().position(|x| x.id == id)
			.ok_or_else(|| format!("failed to find window with ID {}", id))?;
		let mut window = self.windows.remove(index);
		window.close();
		Ok(())
	}
}

impl WindowInner {
	/// Set the displayed image.
	fn set_image(&mut self, mut data: Box<[u8]>, info: ImageInfo) -> Result<(), String> {
		let pixel_format = match info.pixel_format {
			PixelFormat::Bgr8  => PixelFormatEnum::RGB24,
			PixelFormat::Rgba8 => PixelFormatEnum::RGBA32,
			PixelFormat::Rgb8  => PixelFormatEnum::BGR24,
			PixelFormat::Bgra8 => PixelFormatEnum::BGRA32,
			PixelFormat::Mono8 => return Err(String::from("8-bit mono images are not yet supported")),
		};

		let surface = Surface::from_data(&mut data, info.width as u32, info.height as u32, info.row_stride as u32, pixel_format)
			.map_err(|e| format!("failed to create surface for pixel data: {}", e))?;
		let image_size = surface.rect();

		let texture = self.texture_creator.create_texture_from_surface(surface)
			.map_err(|e| format!("failed to create texture from surface: {}", e))?;
		let texture = unsafe { std::mem::transmute::<_, Texture<'static>>(texture) };
		self.texture = Some((texture, image_size));

		Ok(())
	}

	/// Close the window.
	fn close(&mut self) {
		self.canvas.window_mut().hide();
	}
}

/// Convert an SDL2 keyboard event to the more generic KeyboardEvent.
fn convert_keyboard_event(
	state: KeyState,
	key_code: Option<sdl2::keyboard::Keycode>,
	scan_code: Option<sdl2::keyboard::Scancode>,
	modifiers: sdl2::keyboard::Mod,
	repeat: bool,
) -> KeyboardEvent {
	KeyboardEvent {
		state,
		key: key_code::convert_key_code(key_code),
		code: scan_code::convert_scan_code(scan_code),
		location: key_location::get_key_location(scan_code),
		modifiers: modifiers::convert_modifiers(modifiers),
		repeat,
		is_composing: false,
	}
}
