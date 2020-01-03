use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

use crate::ImageData;
use crate::KeyboardEvent;
use crate::PixelFormat;
use crate::KeyState;

mod key_code;
mod key_location;
mod modifiers;
mod scan_code;

pub struct Context {
	video: sdl2::VideoSubsystem,
	events: sdl2::EventPump,
	windows: Vec<ContextWindow>,
}

struct WindowInner {
	canvas: Canvas<sdl2::video::Window>,
	texture_creator: TextureCreator<sdl2::video::WindowContext>,
	texture: Option<(Texture<'static>, sdl2::rect::Rect)>,
}

struct ContextWindow {
	id: u32,
	inner: Arc<Mutex<WindowInner>>,
	event_tx: mpsc::SyncSender<KeyboardEvent>,
}

pub struct Window {
	inner: Arc<Mutex<WindowInner>>,
	event_rx: mpsc::Receiver<KeyboardEvent>,
}

impl Context {
	/// Create a new context.
	pub fn new() -> Result<Self, String> {
		sdl2::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");
		let context = sdl2::init().map_err(|e| format!("failed to initialize SDL2: {}", e))?;
		let video = context.video().map_err(|e| format!("failed to get SDL2 video subsystem: {}", e))?;
		let events = context.event_pump().map_err(|e| format!("failed to get SDL2 event pump: {}", e))?;

		Ok(Self {
			video,
			events,
			windows: Vec::new(),
		})
	}

	/// Create a new window.
	pub fn window(&mut self, name: &str) -> Result<Window, String> {
		let window = self.video.window(name, 800, 600)
			.borderless()
			.resizable()
			.build()
			.map_err(|e| format!("failed to create window {:?}: {}", name, e))?;

		let id = window.id();
		let canvas = window.into_canvas().build().map_err(|e| format!("failed to create canvas for window {:?}: {}", name, e))?;
		let texture_creator = canvas.texture_creator();
		let (event_tx, event_rx) = mpsc::sync_channel(10);

		let inner = Arc::new(Mutex::new(WindowInner {
			canvas,
			texture_creator,
			texture: None,
		}));

		self.windows.push(ContextWindow {
			id,
			inner: inner.clone(),
			event_tx,
		});

		Ok(Window { inner, event_rx })
	}

	pub fn run(&mut self) -> Result<(), String> {
		let delay = std::time::Duration::from_nanos(1_000_000_000 / 60);
		let mut next_frame = std::time::Instant::now() + delay;

		loop {
			// Handle all queued events.
			while let Some(event) = self.events.poll_event() {
				self.handle_event(event)?;
			}

			// Loop over all windows.
			for window in &self.windows {
				let mut window = window.inner.lock().unwrap();
				let texture = window.texture.take();

				// Always clear the whole window, to avoid artefacts.
				window.canvas.clear();

				// Redraw the image, if any.
				if let Some((texture, image_size)) = texture {
					let viewport = window.canvas.viewport();
					window.canvas.copy(&texture, image_size.clone(), viewport)
						.map_err(|e| format!("failed to copy data to window: {}", e))?;
					window.texture = Some((texture, image_size));
					window.canvas.window_mut().show();
				}

				window.canvas.present();
			}

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

	fn handle_event(&mut self, event: Event) -> Result<(), String> {
		match event {
			Event::Window { window_id, win_event, .. } => {
				self.handle_window_event(window_id, win_event)
			},
			Event::KeyDown { window_id, keycode, scancode, keymod, repeat, .. } => {
				self.handle_key_event(window_id, convert_key_event(KeyState::Down, keycode, scancode, keymod, repeat))
			},
			Event::KeyUp { window_id, keycode, scancode, keymod, repeat, .. } => {
				self.handle_key_event(window_id, convert_key_event(KeyState::Up, keycode, scancode, keymod, repeat))
			},
			_ => Ok(()),
		}

	}

	fn handle_window_event(&mut self, window_id: u32, event: WindowEvent) -> Result<(), String> {
		match event {
			WindowEvent::Close => self.close_window(window_id),
			_ => (),
		}
		Ok(())
	}

	fn handle_key_event(&mut self, window_id: u32, event: KeyboardEvent) -> Result<(), String> {
		if let Some(window) = self.windows.iter().find(|x| x.id == window_id) {
			// Ignore errors, it likely means the receiver isn't handling events.
			let _ = window.event_tx.try_send(event);
		}
		Ok(())
	}

	fn close_window(&mut self, window_id: u32) {
		// Only hide the window.
		// Destroy it in the main loop if all handles are dropped.
		if let Some(window) = self.windows.iter().find(|x| x.id == window_id) {
			let mut window = window.inner.lock().unwrap();
			window.texture = None;
			window.canvas.window_mut().hide();
		}
	}
}

impl Window {
	pub fn show(&self, image: &impl ImageData) -> Result<(), String> {
		let size = image.data().len();
		let data = unsafe { std::slice::from_raw_parts_mut(image.data().as_ptr() as *mut u8, size) };
		let info = image.info().map_err(|e| format!("failed to display image: {}", e))?;

		let pixel_format = match info.pixel_format {
			PixelFormat::Bgr8  => PixelFormatEnum::RGB24,
			PixelFormat::Rgba8 => PixelFormatEnum::RGBA32,
			PixelFormat::Rgb8  => PixelFormatEnum::BGR24,
			PixelFormat::Bgra8 => PixelFormatEnum::BGRA32,
			PixelFormat::Mono8 => unimplemented!(),
		};

		let surface = Surface::from_data(data, info.width as u32, info.height as u32, info.row_stride as u32, pixel_format)
			.map_err(|e| format!("failed to create surface for pixel data: {}", e))?;
		let image_size = surface.rect();

		let mut inner = self.inner.lock().unwrap();
		let texture = inner.texture_creator.create_texture_from_surface(surface)
			.map_err(|e| format!("failed to create texture from surface: {}", e))?;
		let texture = unsafe { std::mem::transmute::<_, Texture<'static>>(texture) };
		inner.texture = Some((texture, image_size));

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
