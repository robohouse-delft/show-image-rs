use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;

#[cfg(feature = "save")]
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use crate::ImageData;
use crate::ImageInfo;
#[cfg(feature = "save")]
use crate::KeyCode;
use crate::KeyState;
use crate::KeyboardEvent;
#[cfg(feature = "save")]
use crate::KeyModifiers;
use crate::PixelFormat;
use crate::WaitKeyError;
use crate::WindowOptions;
use crate::oneshot;

mod monochrome;
mod key_code;
mod key_location;
mod modifiers;
mod scan_code;

const RESULT_TIMEOUT: Duration = Duration::from_millis(500);

/// A context for creating windows.
///
/// Once initialized, the context runs an event loop in a background thread.
/// You can interact with the background thead through the context object.
pub struct Context {
	/// Channel to send command to the background thread.
	command_tx: mpsc::SyncSender<ContextCommand>,

	/// Join handle for the background thread.
	thread: std::thread::JoinHandle<Result<(), String>>,
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
	/// Stop the background thread as soon as possible.
	Stop(oneshot::Sender<()>),

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

	/// The palette to use for drawing monochrome pictures.
	mono_palette: sdl2::pixels::Palette,

	/// List of created windows.
	windows: Vec<WindowInner>,

	/// Channel to receive commands.
	command_rx: mpsc::Receiver<ContextCommand>,

	/// Flag to indicate the context should stop as soon as possible.
	stop: bool,
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
	texture: Option<(Texture<'static>, Rect)>,

	/// The data of the currently displayed image.
	#[cfg(feature = "save")]
	data: Option<(Arc<Box<[u8]>>, ImageInfo)>,

	/// Join handles for background threads saving images.
	#[cfg(feature = "save")]
	save_threads: Vec<std::thread::JoinHandle<Result<(), String>>>,

	/// Channel to send keyboard events.
	event_tx: mpsc::SyncSender<KeyboardEvent>,

	/// If true, preserve aspect ratio when scaling image.
	preserve_aspect_ratio: bool,
}

impl Context {
	/// Create a new context.
	///
	/// The context will spawn a background thread immediately.
	pub fn new() -> Result<Self, String> {
		let (result_tx, mut result_rx) = oneshot::channel();
		let (command_tx, command_rx) = mpsc::sync_channel(10);
		let thread = std::thread::spawn(move || {
			match ContextInner::new(command_rx) {
				Err(e) => {
					result_tx.send(Err(e));
					Ok(())
				},
				Ok(mut context) => {
					result_tx.send(Ok(()));
					context.run()
				}
			}
		});

		match result_rx.recv_timeout(Duration::from_millis(1500)) {
			Err(e) => Err(format!("failed to receive result from context thread: {}", e)),
			Ok(Err(e)) => Err(e),
			Ok(Ok(())) => Ok(Context { command_tx, thread })
		}
	}

	/// Create a new window with default options.
	pub fn make_window(&self, name: impl Into<String>) -> Result<Window, String> {
		let options = WindowOptions { name: name.into(), ..Default::default() };
		self.make_window_full(options)
	}

	/// Create a new window with the given options.
	pub fn make_window_full(&self, options: WindowOptions) -> Result<Window, String> {
		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::CreateWindow(options, self.command_tx.clone(), result_tx))
			.map_err(|e| format!("failed to send command to context thread: {}", e))?;
		result_rx.recv_timeout(RESULT_TIMEOUT).map_err(|e| format!("failed to receive result from context thread: {}", e))?
	}

	/// Close all windows and stop the background thread.
	///
	/// The background thread will stop as soon as possible,
	/// but it may still be running when this function returns.
	///
	/// Use [`Context::join`] to join the background thread if desired.
	#[allow(unused)]
	pub fn stop(&self) -> Result<(), String> {
		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::Stop(result_tx))
			.map_err(|e| format!("failed to send command to context thread: {}", e))?;
		result_rx.recv_timeout(RESULT_TIMEOUT).map_err(|e| format!("failed to receive result from context thread: {}", e))
	}

	/// Join the background thread, blocking until the thread has terminated.
	///
	/// This function also returns any possible error that occured in the background thread.
	///
	/// Note that the background thread will only terminate if an error occurs
	/// or if [`Context::stop`] is called.
	#[allow(unused)]
	pub fn join(self) -> Result<(), String> {
		self.thread.join().map_err(|e| format!("failed to join context thread: {:?}", e))?
	}
}

impl Window {
	/// Set the image to de displayed by the window.
	pub fn set_image(&self, image: impl ImageData) -> Result<(), String> {
		let info = image.info().map_err(|e| format!("failed to display image: {}", e))?;
		let data = image.data();

		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::SetImage(self.id, data, info, result_tx)).unwrap();
		result_rx.recv_timeout(RESULT_TIMEOUT)
			.map_err(|e| format!("failed to receive result from context thread: {}", e))?
			.map_err(|e| format!("failed to display image: {}", e))
	}

	/// Close the window.
	///
	/// The window is automatically closed if the handle is dropped,
	/// but this function allows you to handle errors that may occur.
	pub fn close(self) -> Result<(), String> {
		self.close_impl()
	}

	/// Get the receiver for keyboard events.
	pub fn events(&self) -> &mpsc::Receiver<KeyboardEvent> {
		&self.event_rx
	}

	/// Wait for a key-down event with a timeout.
	///
	/// If an error is returned, no further key events will be received.
	/// Any loop processing keyboard input should terminate.
	///
	/// If no key press was available within the timeout, `Ok(None)` is returned.
	///
	/// This function discards all key-up events and blocks until a key is pressed or the timeout occurs.
	pub fn wait_key(&self, timeout: Duration) -> Result<Option<KeyboardEvent>, WaitKeyError> {
		self.wait_key_deadline(Instant::now() + timeout)
	}

	/// Wait for a key-down event with a deadline.
	///
	/// If an error is returned, no further key events will be received.
	/// Any loop processing keyboard input should terminate.
	///
	/// If no key press was available within the timeout, `Ok(None)` is returned.
	///
	/// This function discards all key-up events and blocks until a key is pressed or the deadline passes.
	pub fn wait_key_deadline(&self, deadline: Instant) -> Result<Option<KeyboardEvent>, WaitKeyError> {
		loop {
			let now = Instant::now();
			if now >= deadline {
				return Ok(None);
			}
			let event = match self.events().recv_timeout(deadline - now) {
				Ok(x) => x,
				Err(mpsc::RecvTimeoutError::Timeout) => return Ok(None),
				Err(mpsc::RecvTimeoutError::Disconnected) => return Err(WaitKeyError::WindowClosed),
			};

			if event.state == KeyState::Down {
				return Ok(Some(event))
			}
		}
	}

	/// Close the window without dropping the handle.
	pub fn close_impl(&self) -> Result<(), String> {
		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::DestroyWindow(self.id, result_tx))
			.map_err(|e| format!("failed to send command to window: {}", e))?;
		result_rx.recv_timeout(RESULT_TIMEOUT).map_err(|e| format!("failed to receive result from context thread: {}", e))?
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
		let mono_palette = monochrome::mono_palette().map_err(|e| format!("failed to create monochrome palette: {}", e))?;

		Ok(Self {
			video,
			events,
			mono_palette,
			windows: Vec::new(),
			command_rx,
			stop: false,
		})
	}

	/// Run the event loop.
	fn run(&mut self) -> Result<(), String> {
		let delay = Duration::from_nanos(1_000_000_000 / 60);
		let mut next_frame = Instant::now() + delay;

		while !self.stop {
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

		Ok(())
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
			window.draw()?;
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
	#[allow(clippy::single_match)]
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
		if let Some(window) = self.windows.iter_mut().find(|x| x.id == window_id) {
			window.handle_keyboard_event(event)?;
		}
		Ok(())
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
			ContextCommand::Stop(result_tx) => {
				self.stop = true;
				result_tx.send(());
			},
			ContextCommand::CreateWindow(options, command_tx, result_tx) => {
				result_tx.send(self.make_window(options, command_tx));
			},
			ContextCommand::DestroyWindow(id, result_tx) => {
				result_tx.send(self.destroy_window(id));
			},
			ContextCommand::SetImage(id, data, info, result_tx) => {
				match self.windows.iter_mut().find(|x| x.id == id) {
					None => result_tx.send(Err(format!("failed to find window with ID {}", id))),
					Some(window) => result_tx.send(window.set_image(&self.mono_palette, data, info)),
				}
			},
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
			#[cfg(feature = "save")]
			data: None,
			#[cfg(feature = "save")]
			save_threads: Vec::new(),
			event_tx,
			preserve_aspect_ratio: options.preserve_aspect_ratio,
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
	fn set_image(&mut self, mono_palette: &sdl2::pixels::Palette, mut data: Box<[u8]>, info: ImageInfo) -> Result<(), String> {
		let pixel_format = match info.pixel_format {
			PixelFormat::Mono8 => PixelFormatEnum::Index8,
			PixelFormat::Rgb8  => PixelFormatEnum::RGB24,
			PixelFormat::Rgba8 => PixelFormatEnum::RGBA32,
			PixelFormat::Bgr8  => PixelFormatEnum::BGR24,
			PixelFormat::Bgra8 => PixelFormatEnum::BGRA32,
		};

		let mut surface = Surface::from_data(&mut data, info.width as u32, info.height as u32, info.row_stride as u32, pixel_format)
			.map_err(|e| format!("failed to create surface for pixel data: {}", e))?;
		let image_size = surface.rect();


		if info.pixel_format == PixelFormat::Mono8 {
			surface.set_palette(mono_palette).map_err(|e| format!("failed to set monochrome palette on canvas: {}", e))?;
		}

		let texture = self.texture_creator.create_texture_from_surface(surface)
			.map_err(|e| format!("failed to create texture from surface: {}", e))?;
		let texture = unsafe { std::mem::transmute::<_, Texture<'static>>(texture) };
		self.texture = Some((texture, image_size));

		#[cfg(feature = "save")] {
			self.data = Some((Arc::new(data), info));
		}

		Ok(())
	}

	/// Draw the contents of the window.
	fn draw(&mut self) -> Result<(), String> {
		// Always clear the whole window, to avoid artefacts.
		self.canvas.clear();

		// Redraw the image, if any.
		if let Some((texture, image_size)) = &self.texture {
			let rect = if self.preserve_aspect_ratio {
				compute_target_rect_with_aspect_ratio(image_size, &self.canvas.viewport())
			} else {
				self.canvas.viewport()
			};

			self.canvas.copy(&texture, image_size.clone(), rect)
				.map_err(|e| format!("failed to copy data to self: {}", e))?;
			self.canvas.window_mut().show();
		}

		self.canvas.present();
		Ok(())
	}

	/// Close the window.
	fn close(&mut self) {
		self.canvas.window_mut().hide();
	}

	fn handle_keyboard_event(&mut self, event: KeyboardEvent) -> Result<(), String> {
		#[cfg(feature = "save")] {
			if event.state == KeyState::Down && event.key == KeyCode::Character("S".into()) && event.modifiers == KeyModifiers::CONTROL {
				return self.save_image();
			}
		}
		// Ignore errors, it means the receiver isn't handling events.
		let _ = self.event_tx.try_send(event);

		Ok(())
	}

	#[cfg(feature = "save")]
	fn save_image(&mut self) -> Result<(), String> {
		let (data, info) = match &self.data {
			Some(x) => x.clone(),
			None => return Ok(()),
		};

		let thread = std::thread::spawn(move || {
			let path = match tinyfiledialogs::save_file_dialog("Save image", "image.png") {
				Some(x) => x,
				None => return Ok(()),
			};

			save_image(path.as_ref(), &data, info)
		});

		// TODO: Reap finished join handles at some point.
		// TODO: Join them somehow when cleanly stopping context.
		self.save_threads.push(thread);

		Ok(())
	}
}

#[cfg(feature = "save")]
fn save_image(path: &std::path::Path, data: &[u8], info: ImageInfo) -> Result<(), String> {
	let color_type = match info.pixel_format {
		PixelFormat::Mono8 => image::ColorType::Gray(8),
		PixelFormat::Rgb8  => image::ColorType::RGB(8),
		PixelFormat::Rgba8 => image::ColorType::RGBA(8),
		PixelFormat::Bgr8  => image::ColorType::BGR(8),
		PixelFormat::Bgra8 => image::ColorType::BGRA(8),
	};

	let bytes_per_pixel = usize::from(info.pixel_format.bytes_per_pixel());

	if info.row_stride == info.width * bytes_per_pixel {
		image::save_buffer(path, data, info.width as u32, info.height as u32, color_type)
			.map_err(|e| format!("failed to save image: {}", e))
	} else {
		let mut packed = Vec::with_capacity(info.width * info.height * bytes_per_pixel);
		for row in 0..info.height {
			packed.extend_from_slice(&data[info.row_stride * row..][..info.width * bytes_per_pixel]);
		}
		image::save_buffer(path, &packed, info.width as u32, info.height as u32, color_type)
			.map_err(|e| format!("failed to save image: {}", e))
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

fn compute_target_rect_with_aspect_ratio(source: &Rect, canvas: &Rect) -> Rect {
	let source_w = f64::from(source.width());
	let source_h = f64::from(source.height());
	let canvas_w = f64::from(canvas.width());
	let canvas_h = f64::from(canvas.height());

	let scale_w = canvas_w / source_w;
	let scale_h = canvas_h / source_h;

	if scale_w < scale_h {
		let new_height = (source_h * scale_w).round() as u32;
		let top = (canvas.height() - new_height) / 2;
		Rect::new(canvas.x(), canvas.y() + top as i32, canvas.width(), new_height)
	} else {
		let new_width = (source_w * scale_h).round() as u32;
		let left = (canvas.width() - new_width) / 2;
		Rect::new(canvas.x() + left as i32, canvas.y(), new_width, canvas.height())
	}
}
