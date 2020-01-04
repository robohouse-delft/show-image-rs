use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;

use std::sync::mpsc;
use crate::oneshot;

use crate::ImageData;
use crate::ImageInfo;
use crate::KeyboardEvent;
use crate::PixelFormat;
use crate::KeyState;

mod key_code;
mod key_location;
mod modifiers;
mod scan_code;

pub struct Context {
	command_tx: mpsc::SyncSender<ContextCommand>,
	thread: std::thread::JoinHandle<Result<(), String>>,
}

pub struct Window {
	command_tx: mpsc::SyncSender<WindowCommand>,
	event_rx: mpsc::Receiver<KeyboardEvent>,
}

pub struct WindowOptions {
	pub name: String,
	pub size: [u32; 2],
	pub resizable: bool,
}

enum ContextCommand {
	CreateWindow(WindowOptions, oneshot::Sender<Result<Window, String>>),
}

enum WindowCommand {
	SetImage(Box<[u8]>, ImageInfo),
	Close(oneshot::Sender<()>),
}

struct ContextInner {
	video: sdl2::VideoSubsystem,
	events: sdl2::EventPump,
	windows: Vec<WindowInner>,
	command_rx: mpsc::Receiver<ContextCommand>,
}

struct WindowInner {
	id: u32,
	canvas: Canvas<sdl2::video::Window>,
	texture_creator: TextureCreator<sdl2::video::WindowContext>,
	texture: Option<(Texture<'static>, sdl2::rect::Rect)>,
	command_rx: mpsc::Receiver<WindowCommand>,
	event_tx: mpsc::SyncSender<KeyboardEvent>,
}

impl Context {
	pub fn new() -> Result<Self, String> {
		let (command_tx, command_rx) = mpsc::sync_channel(10);
		let thread = std::thread::spawn(move || {
			let mut context = ContextInner::new(command_rx)?;
			context.run()
		});

		Ok(Context {
			command_tx,
			thread,
		})
	}

	pub fn make_window(&mut self, options: WindowOptions) -> Result<Window, String> {
		let (result_tx, result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::CreateWindow(options, result_tx))
			.map_err(|e| format!("failed to send command to context thread: {}", e))?;
		result_rx.recv().map_err(|e| format!("context thread did not create a window: {}", e))?
	}

	pub fn join(self) -> Result<(), String> {
		self.thread.join().map_err(|e| format!("failed to join context thread: {:?}", e))?
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

	pub fn run(&mut self) -> Result<(), String> {
		let delay = std::time::Duration::from_nanos(1_000_000_000 / 60);
		let mut next_frame = std::time::Instant::now() + delay;

		loop {
			self.run_one()?;

			// Sleep till the next scheduled frame.
			let now = std::time::Instant::now();
			if now < next_frame {
				std::thread::sleep(next_frame - now);
				next_frame += delay;
			} else {
				next_frame = now.max(next_frame + delay);
			}
		}
	}

	fn run_one(&mut self) -> Result<(), String> {
		// Handle all queued SDL events.
		while let Some(event) = self.events.poll_event() {
			self.handle_sdl_event(event)?;
		}

		// Handle all queued commands for the context.
		self.poll_commands();

		// Handle all queued window commandsn.
		for window in &mut self.windows {
			window.poll_commands()?;
		}

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

	fn handle_sdl_event(&mut self, event: Event) -> Result<(), String> {
		match event {
			Event::Window { window_id, win_event, .. } => {
				self.handle_sdl_window_event(window_id, win_event)
			},
			Event::KeyDown { window_id, keycode, scancode, keymod, repeat, .. } => {
				self.handle_sdl_key_event(window_id, convert_key_event(KeyState::Down, keycode, scancode, keymod, repeat))
			},
			Event::KeyUp { window_id, keycode, scancode, keymod, repeat, .. } => {
				self.handle_sdl_key_event(window_id, convert_key_event(KeyState::Up, keycode, scancode, keymod, repeat))
			},
			_ => Ok(()),
		}

	}

	fn handle_sdl_window_event(&mut self, window_id: u32, event: WindowEvent) -> Result<(), String> {
		match event {
			WindowEvent::Close => {
				self.find_window_mut(window_id).map(|x| x.close());
			},
			_ => (),
		}
		Ok(())
	}

	fn handle_sdl_key_event(&mut self, window_id: u32, event: KeyboardEvent) -> Result<(), String> {
		if let Some(window) = self.windows.iter().find(|x| x.id == window_id) {
			// Ignore errors, it likely means the receiver isn't handling events.
			let _ = window.event_tx.try_send(event);
		}
		Ok(())
	}

	fn find_window_mut(&mut self, id: u32) -> Option<&mut WindowInner> {
		self.windows.iter_mut().find(|x| x.id == id)
	}

	fn poll_commands(&mut self) {
		while let Ok(command) = self.command_rx.try_recv() {
			self.handle_command(command);
		}
	}

	fn handle_command(&mut self, command: ContextCommand) {
		match command {
			ContextCommand::CreateWindow(options, channel) => {
				channel.send(self.make_window(options));
			},
		}
	}

	/// Create a new window.
	fn make_window(&mut self, options: WindowOptions) -> Result<Window, String> {
		let window = self.video.window(&options.name, options.size[0], options.size[1])
			.borderless()
			.resizable()
			.build()
			.map_err(|e| format!("failed to create window {:?}: {}", options.name, e))?;

		let id = window.id();
		let canvas = window.into_canvas().build().map_err(|e| format!("failed to create canvas for window {:?}: {}", options.name, e))?;
		let texture_creator = canvas.texture_creator();
		let (command_tx, command_rx) = mpsc::sync_channel(10);
		let (event_tx, event_rx) = mpsc::sync_channel(10);

		let inner = WindowInner {
			id,
			canvas,
			texture_creator,
			texture: None,
			command_rx,
			event_tx,
		};

		self.windows.push(inner);

		Ok(Window { command_tx, event_rx })
	}
}

impl WindowInner {
	fn poll_commands(&mut self) -> Result<(), String> {
		loop {
			match self.command_rx.try_recv() {
				Ok(x) => self.handle_command(x)?,
				Err(mpsc::TryRecvError::Empty) => return Ok(()),
				Err(mpsc::TryRecvError::Disconnected) => {
					self.close();
					return Ok(());
				}
			}
		}
	}

	fn handle_command(&mut self, command: WindowCommand) -> Result<(), String> {
		match command {
			WindowCommand::SetImage(data, info) => self.set_image(data, info),
			WindowCommand::Close(result_tx) => {
				self.close();
				result_tx.send(());
				Ok(())
			},
		}
	}

	fn set_image(&mut self, mut data: Box<[u8]>, info: ImageInfo) -> Result<(), String> {
		let pixel_format = match info.pixel_format {
			PixelFormat::Bgr8  => PixelFormatEnum::RGB24,
			PixelFormat::Rgba8 => PixelFormatEnum::RGBA32,
			PixelFormat::Rgb8  => PixelFormatEnum::BGR24,
			PixelFormat::Bgra8 => PixelFormatEnum::BGRA32,
			PixelFormat::Mono8 => unimplemented!(),
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

	fn close(&mut self) {
		self.canvas.window_mut().hide();
	}
}

impl Window {
	pub fn set_image(&self, image: &impl ImageData) -> Result<(), String> {
		let data = Box::from(image.data());
		let info = image.info().map_err(|e| format!("failed to display image: {}", e))?;
		self.command_tx.send(WindowCommand::SetImage(data, info)).unwrap();
		Ok(())
	}

	pub fn close(&self) -> Result<(), String> {
		let (result_tx, result_rx) = oneshot::channel();
		self.command_tx.send(WindowCommand::Close(result_tx))
			.map_err(|e| format!("failed to send command to window: {}", e))?;
		result_rx.recv().map_err(|e| format!("error receiving response from window: {}", e))?;
		Ok(())
	}

	/// Get the receiver for keyboard events.
	pub fn events(&self) -> &mpsc::Receiver<KeyboardEvent> {
		&self.event_rx
	}

	/// Wait for a key down event with a timeout.
	///
	/// This function discards all key-up events, blocking until a key had been pressed,
	/// or the timeout occured.
	pub fn wait_key(&self, timeout: std::time::Duration) -> Option<KeyboardEvent> {
		let deadline = std::time::Instant::now() + timeout;
		loop {
			let now = std::time::Instant::now();
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

	/// Wait for a key down event with a dealine.
	///
	/// This function discards all key-up events, blocking until a key had been pressed,
	/// or the deadline passes.
	pub fn wait_key_deadline(&self, deadline: std::time::Instant) -> Option<KeyboardEvent> {
		loop {
			let now = std::time::Instant::now();
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
}

/// Convert an SDL key event to the more generic KeyboardEvent.
fn convert_key_event(
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
