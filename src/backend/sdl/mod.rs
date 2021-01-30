use sdl2::event::Event as SdlEvent;
use sdl2::event::WindowEvent;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect as SdlRect;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use crate::Image;
use crate::ImageData;
use crate::ImageInfo;
#[cfg(feature = "save")]
use crate::KeyCode;
use crate::KeyState;
use crate::KeyboardEvent;
#[cfg(feature = "save")]
use crate::KeyModifiers;
use crate::PixelFormat;
use crate::Rectangle;
use crate::WindowOptions;
use crate::oneshot;
use crate::background_thread::BackgroundThread;
use crate::MouseState;
use crate::MouseMoveEvent;
use crate::MouseButtonState;
use crate::MouseButton;
use crate::MouseButtonEvent;
use crate::Event;

mod monochrome;
mod key_code;
mod key_location;
mod modifiers;
mod scan_code;

pub use super::EventHandler;
pub use super::EventHandlerContext;

const RESULT_TIMEOUT: Duration = Duration::from_millis(1_000);

/// A context for creating windows.
///
/// Once initialized, the context runs an event loop in a background thread.
/// You can interact with the background thead through the context object.
pub struct Context {
	/// Channel to send command to the background thread.
	command_tx: mpsc::SyncSender<ContextCommand>,

	/// Join handle for the background thread.
	thread: Mutex<Option<BackgroundThread<Result<(), String>>>>,
}

/// A window capable of displaying images.
///
/// The handle can be used to set the displayed image,
/// handle key events and to close the window.
///
/// If the handle is dropped, the window is closed.
#[derive(Clone)]
pub struct Window {
	/// The window ID.
	id: u32,

	/// Channel to send commands to the background thread.
	command_tx: mpsc::SyncSender<ContextCommand>,
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
	SetImage(u32, String, ImageInfo, Box<[u8]>, Option<Vec<(Box<[u8]>, ImageInfo)>>, oneshot::Sender<Result<(), String>>),

	/// Get the currently displayed image of the window.
	GetImage(u32, oneshot::Sender<Result<Option<Image>, String>>),

	/// Register a event handler to be run in the background context.
	AddEventHandler(u32, EventHandler, oneshot::Sender<Result<(), String>>),

	/// Run a custom function on the inner context.
	Custom(Box<dyn FnOnce(&mut ContextInner) + Send>),
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

	/// Event handlers to be run in the context thread.
	event_handlers: Vec<(u32, EventHandler)>,

	/// Background tasks that might be joined later.
	background_tasks: Vec<BackgroundThread<()>>,

	/// Flag to indicate the context should stop as soon as possible.
	stop: bool,
}

/// Inner window doing the real work in the background thread.
pub struct WindowInner {
	/// The window ID, used to look up the window in the vector.
	id: u32,

	/// The canvas to draw the image on.
	canvas: Canvas<sdl2::video::Window>,

	/// A texture creator for the window.
	texture_creator: TextureCreator<sdl2::video::WindowContext>,

	/// Monochrome palette.
	mono_palette: sdl2::pixels::Palette,

	/// A texture representing the current image to be drawn.
	texture: Option<(Texture<'static>, Rectangle)>,

	/// Overlays to be drawn on top of the current image.
	overlays: Vec<(Texture<'static>, Rectangle)>,

	/// The data of the currently displayed image.
	image: Option<Image>,

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
		let thread = BackgroundThread::new(move || {
			match ContextInner::new(command_rx) {
				Err(e) => {
					result_tx.send(Err(e));
					Ok(())
				},
				Ok(mut context) => {
					result_tx.send(Ok(()));
					context.run()?;
					context.join_background_tasks();
					Ok(())
				}
			}
		});

		match result_rx.recv_timeout(Duration::from_millis(1500)) {
			Err(e) => Err(format!("failed to receive ready notification from context thread: {}", e)),
			Ok(Err(e)) => Err(e),
			Ok(Ok(())) => Ok(Context {
				command_tx,
				thread: Mutex::new(Some(thread)),
			})
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
			.map_err(|e| format!("failed to send CreateWindow command to context thread: {}", e))?;
		result_rx.recv_timeout(RESULT_TIMEOUT).map_err(|e| format!("failed to receive CreateWindow result from context thread: {}", e))?
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
			.map_err(|e| format!("failed to send Stop command to context thread: {}", e))?;
		result_rx.recv_timeout(RESULT_TIMEOUT).map_err(|e| format!("failed to receive Stop result from context thread: {}", e))
	}

	/// Join the background thread, blocking until the thread has terminated.
	///
	/// This function also returns any possible error that occured in the background thread.
	///
	/// Note that the background thread will only terminate if an error occurs
	/// or if [`Context::stop`] is called.
	#[allow(unused)]
	pub fn join(&self) -> Result<(), String> {
		// Join main context thread.
		let mut thread = self.thread.lock().unwrap();
		if let Some(thread) = thread.take() {
			thread.join().map_err(|e| format!("failed to join context thread: {:?}", e))?
		} else {
			Ok(())
		}
	}
}

impl Window {
	/// Set the image to de displayed by the window.
	///
	/// The name is used to suggest a defaullt file name when saving images.
	/// It is also returned again by [`Window::get_image`].
	///
	/// Setting the image with this function also clears any added overlays.
	pub fn set_image(&self, name: impl Into<String>, image: impl ImageData) -> Result<(), String> {
		let info = image.info().map_err(|e| format!("failed to display image: {}", e))?;
		let data = image.data();

		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::SetImage(self.id, name.into(), info, data, None, result_tx)).unwrap();
		result_rx.recv_timeout(RESULT_TIMEOUT)
			.map_err(|e| format!("failed to receive SetImage result from context thread: {}", e))?
			.map_err(|e| format!("failed to display image: {}", e))
	}

	/// Get the currently displayed image of the window.
	pub fn get_image(&self) -> Result<Option<Image>, String> {
		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::GetImage(self.id, result_tx)).unwrap();
		result_rx.recv_timeout(RESULT_TIMEOUT)
			.map_err(|e| format!("failed to receive GetImage result from context thread: {}", e))?
	}

	/// Add a handler for events.
	///
	/// The added handler will be run directly in the context thread.
	/// This allows you to handle events asynchronously,
	/// but it also means your hander shouldn't block for long.
	///
	/// The handler can use the [`EventHandlerContext::spawn_task`] to perform long running operations.
	///
	/// If you want to handle events in your own thread,
	/// use [`Window::wait_key`], [`Window::wait_key_deadline`] or [`Window::events`].
	pub fn add_event_handler<Handler>(&self, handler: Handler) -> Result<(), String>
	where
		Handler: FnMut(&mut EventHandlerContext) + Send + 'static,
	{
		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::AddEventHandler(self.id, Box::new(handler), result_tx))
			.map_err(|e| format!("failed to send AddEventHandler command to window: {}", e))?;
		result_rx.recv_timeout(RESULT_TIMEOUT)
			.map_err(|e| format!("failed to receive AddEventHandler result from context thread: {}", e))?
	}

	/// Close the window.
	///
	/// The window is automatically closed if the handle is dropped,
	/// but this function allows you to handle errors that may occur.
	pub fn close(self) -> Result<(), String> {
		self.close_impl()
	}

	/// Execute a custom function on the window from inside the context thread.
	pub fn execute<F, T>(&self, function: F) -> Result<T, String>
	where
		F: FnOnce(&mut WindowInner) -> Result<T, String> + Send + 'static,
		T: Send + 'static,
	{
		let (result_tx, result_rx) = oneshot::channel::<Result<T, String>>();
		let window_id = self.id;
		self.command_tx.send(ContextCommand::Custom(Box::new(move |context| {
			match context.windows.iter_mut().find(|x| x.id == window_id) {
				None => result_tx.send(Err(format!("failed to find window with ID {}", window_id))),
				Some(x) => result_tx.send(function(x))
			}
		}))).unwrap();
		result_rx.recv().unwrap()
	}

	/// Create a new channel for receiving events.
	///
	/// When called multiple times, this will create multiple channels.
	/// Each channel will receive all events.
	///
	/// To disable the created channel, simply drop the receiver.
	pub fn events(&self) -> Result<mpsc::Receiver<Event>, String> {
		let (event_tx, event_rx) = mpsc::sync_channel(10);
		let handler = move |context: &mut EventHandlerContext| {
			// Try to send the event, and remove the handler if the receiver is dropped.
			if let Err(mpsc::TrySendError::Disconnected(_)) = event_tx.try_send(context.event().clone()) {
				context.remove_handler();
			}
		};

		self.add_event_handler(handler)?;
		Ok(event_rx)
	}

	/// Close the window without dropping the handle.
	fn close_impl(&self) -> Result<(), String> {
		let (result_tx, mut result_rx) = oneshot::channel();
		self.command_tx.send(ContextCommand::DestroyWindow(self.id, result_tx))
			.map_err(|e| format!("failed to send DestroyWindow command to window: {}", e))?;
		result_rx.recv_timeout(RESULT_TIMEOUT).map_err(|e| format!("failed to receive DestroyWindow result from context thread: {}", e))?
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
			event_handlers: Vec::new(),
			background_tasks: Vec::new(),
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
		// Skip all key events for windows that just got focused,
		// because these are probably virtual events that happened while the window was not focused.
		// Work-around for https://bugzilla.libsdl.org/show_bug.cgi?id=4989
		let mut focused_windows = Vec::new();
		let mut prev_event = None;
		while let Some(event) = self.events.poll_event() {
			if let SdlEvent::Window { window_id, win_event, .. } = event {
				match win_event {
					WindowEvent::Close => self.destroy_window(window_id)?,
					WindowEvent::FocusGained => focused_windows.push(window_id),
					_ => (),
				}

				continue;
			}

			let should_fold = |a: &MouseMoveEvent, b: &MouseMoveEvent| {
				a.mouse_id == b.mouse_id && a.mouse_state == b.mouse_state
			};

			match (prev_event.take(), convert_event(event, &focused_windows)) {
				(Some((old_window_id, Event::MouseMoveEvent(old))), Some((new_window_id, Event::MouseMoveEvent(new)))) if old_window_id == new_window_id && should_fold(&old, &new) => {
					prev_event = Some((old_window_id, Event::MouseMoveEvent(MouseMoveEvent {
						mouse_id: old.mouse_id,
						mouse_state: old.mouse_state,
						position_x: new.position_x,
						position_y: new.position_y,
						relative_x: old.relative_x + new.relative_x,
						relative_y: old.relative_y + new.relative_y,
					})));
				},
				(old_event, new_event) => {
					if let Some((old_window, old_event)) = old_event {
						self.handle_event(old_window, old_event);
					}
					prev_event = new_event;
				},
			}
		}

		if let Some((window_id, event)) = prev_event {
			self.handle_event(window_id, event)
		}


		// Handle all queued commands for the context.
		self.poll_commands();

		// Loop over all windows.
		for window in &mut self.windows {
			window.draw()?;
		}

		self.clean_background_tasks();
		Ok(())
	}

	fn handle_event(&mut self, window_id: u32, event: Event) {
		if let Some(mut window) = self.windows.iter_mut().find(|x| x.id == window_id) {
			#[cfg(feature = "save")] {
				if let Event::KeyboardEvent(event) = &event {
					let ctrl  = event.modifiers.contains(KeyModifiers::CONTROL);
					let shift = event.modifiers.contains(KeyModifiers::SHIFT);
					let alt   = event.modifiers.contains(KeyModifiers::ALT);
					if event.state == KeyState::Down && event.key == KeyCode::Character("S".into()) && ctrl && !shift && !alt {
						if let Some(work) = window.save_image() {
							self.background_tasks.push(work);
						}
					}
				}
			}

			let mut delete_handlers = Vec::new();
			for (handler_index, (handler_window_id, handler)) in &mut self.event_handlers.iter_mut().enumerate() {
				if *handler_window_id != window_id {
					continue;
				}
				let mut context = EventHandlerContext::new(&mut self.background_tasks, &event, &mut window);
				handler(&mut context);
				if context.should_remove_handler() {
					delete_handlers.push(handler_index);
				}
				if context.should_stop_propagation() {
					break;
				}
			}

			if !delete_handlers.is_empty() {
				let mut index = 0;
				let mut delete_handlers = delete_handlers.as_slice();
				self.event_handlers.retain(|_| {
					if Some(&index) == delete_handlers.first() {
						index += 1;
						delete_handlers = &delete_handlers[1..];
						false
					} else {
						index += 1;
						true
					}
				});
			}
		}
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
			ContextCommand::SetImage(id, name, info, data, overlays, result_tx) => {
				match self.windows.iter_mut().find(|x| x.id == id) {
					None => result_tx.send(Err(format!("failed to find window with ID {}", id))),
					Some(window) => {
						if let Some(overlays) = overlays {
							result_tx.send(window.set_image_with_overlays(name, (data, info), overlays))
						} else {
							result_tx.send(window.set_image(name, (data, info)))
						}
					},
				}
			},
			ContextCommand::GetImage(id, result_tx) => {
				match self.windows.iter_mut().find(|x| x.id == id) {
					None => result_tx.send(Err(format!("failed to find window with ID {}", id))),
					Some(window) => result_tx.send(Ok(window.image.clone())),
				}
			},
			ContextCommand::AddEventHandler(id, handler, result_tx) => {
				match self.windows.iter_mut().find(|x| x.id == id) {
					None => result_tx.send(Err(format!("failed to find window with ID {}", id))),
					Some(_) => {
						self.event_handlers.push((id, handler));
						result_tx.send(Ok(()))
					}
				}
			},
			ContextCommand::Custom(function) => {
				function(self)
			},
		}
	}

	/// Create a new window.
	fn make_window(&mut self, options: WindowOptions, command_tx: mpsc::SyncSender<ContextCommand>) -> Result<Window, String> {
		let mut window_builder = &mut self.video.window(&options.name, options.size[0], options.size[1]);

		if options.borderless{
			window_builder = window_builder.borderless();
		}

		if options.resizable{
			window_builder = window_builder.resizable();
		}

		let window =  window_builder
			.build()
			.map_err(|e| format!("failed to create window {:?}: {}", options.name, e))?;

		let id = window.id();
		let canvas = window.into_canvas().build().map_err(|e| format!("failed to create canvas for window {:?}: {}", options.name, e))?;
		let texture_creator = canvas.texture_creator();
		let mono_palette = monochrome::mono_palette().map_err(|e| format!("failed to create monochrome palette: {}", e))?;

		let inner = WindowInner {
			id,
			canvas,
			texture_creator,
			mono_palette,
			texture: None,
			image: None,
			overlays: Vec::new(),
			preserve_aspect_ratio: options.preserve_aspect_ratio,
		};

		self.windows.push(inner);

		Ok(Window { id, command_tx })
	}

	/// Destroy a window by ID.
	fn destroy_window(&mut self, id: u32) -> Result<(), String> {
		self.event_handlers.retain(|(handler_window_id, _)| *handler_window_id != id);
		let index = self.windows.iter().position(|x| x.id == id)
			.ok_or_else(|| format!("failed to find window with ID {}", id))?;
		let mut window = self.windows.remove(index);
		window.close();
		Ok(())
	}

	/// Clean finished background threads.
	///
	/// Finished threads are joined to check their result.
	/// If a joined thread returns an error, the error is returned and no other threads are cleaned.
	fn clean_background_tasks(&mut self) {
		self.background_tasks.retain(|x| !x.is_done());
	}

	/// Join all background threads.
	///
	/// If a joined thread returns an error, the error is returned and no other threads are joined.
	fn join_background_tasks(&mut self) {
		while !self.background_tasks.is_empty() {
			let _ = self.background_tasks.remove(self.background_tasks.len() - 1).join();
		}
	}
}

impl<'a> From<&'a SdlRect> for Rectangle {
	fn from(other: &'a SdlRect) -> Self {
		Self::from_xywh(other.x(), other.y(), other.width(), other.height())
	}
}

impl From<SdlRect> for Rectangle {
	fn from(other: SdlRect) -> Self {
		(&other).into()
	}
}

impl<'a> From<&'a Rectangle> for SdlRect {
	fn from(other: &'a Rectangle) -> Self {
		Self::new(other.x(), other.y(), other.width(), other.height())
	}
}

impl From<Rectangle> for SdlRect {
	fn from(other: Rectangle) -> Self {
		(&other).into()
	}
}

impl WindowInner {
	pub fn image(&self) -> Option<&Image> {
		self.image.as_ref()
	}
	/// Get the size of the window.
	pub fn size(&self) -> [u32; 2] {
		let viewport = self.canvas.viewport();
		[viewport.width(), viewport.height()]
	}

	/// Get the rectangle with the visible image.
	pub fn image_area(&self) -> Option<Rectangle> {
		let (_texture, image_size) = self.texture.as_ref()?;
		if self.preserve_aspect_ratio {
			let canvas_size = Rectangle::from(&self.canvas.viewport());
			Some(compute_target_rect_with_aspect_ratio(image_size, &canvas_size))
		} else {
			Some(self.canvas.viewport().into())
		}
	}

	/// Set the displayed image of the window.
	pub fn set_image(&mut self, name: String, image: impl ImageData) -> Result<(), String> {
		let info = image.info()?;
		let data = image.data();
		self.set_image_from_data(name, info, data)
	}

	/// Set the displayed image and the overlays of the window.
	pub fn set_image_with_overlays(&mut self, name: String, image: impl ImageData, overlays: Vec<impl ImageData>) -> Result<(), String> {
		let info = image.info()?;
		let data = image.data();
		self.set_image_from_data(name, info, data)?;

		self.overlays.clear();
		self.overlays.reserve(overlays.len());
		for overlay in overlays {
			self.add_overlay(overlay)?;
		}

		Ok(())
	}

	/// Set the displayed image.
	pub fn set_image_from_data(&mut self, name: String, info: ImageInfo, mut data: Box<[u8]>) -> Result<(), String> {
		self.texture = Some(self.make_texture(&info, &mut data)?);
		self.image = Some(Image { data: Arc::from(data), info, name });
		Ok(())
	}

	/// Add an overlay to be drawn over the displayed image.
	pub fn add_overlay(&mut self, image: impl ImageData) -> Result<(), String> {
		let info = image.info()?;
		let mut data = image.data();
		let overlay = self.make_texture(&info, &mut data)?;
		self.overlays.push(overlay);
		Ok(())
	}

	pub fn clear_overlays(&mut self) {
		self.overlays.clear();
	}

	/// Turn an image into a texture.
	fn make_texture(&mut self, info: &ImageInfo, data: &mut [u8]) -> Result<(Texture<'static>, Rectangle), String> {
		let pixel_format = match info.pixel_format {
			PixelFormat::Mono8 => PixelFormatEnum::Index8,
			PixelFormat::Rgb8  => PixelFormatEnum::RGB24,
			PixelFormat::Rgba8 => PixelFormatEnum::RGBA32,
			PixelFormat::Bgr8  => PixelFormatEnum::BGR24,
			PixelFormat::Bgra8 => PixelFormatEnum::BGRA32,
		};

		let mut surface = Surface::from_data(data, info.width as u32, info.height as u32, info.row_stride as u32, pixel_format)
			.map_err(|e| format!("failed to create surface for pixel data: {}", e))?;
		let image_size = Rectangle::from(surface.rect());

		if info.pixel_format == PixelFormat::Mono8 {
			surface.set_palette(&self.mono_palette).map_err(|e| format!("failed to set monochrome palette on canvas: {}", e))?;
		}

		let texture = self.texture_creator.create_texture_from_surface(surface)
			.map_err(|e| format!("failed to create texture from surface: {}", e))?;
		let texture = unsafe { std::mem::transmute::<_, Texture<'static>>(texture) };
		Ok((texture, image_size))
	}

	/// Draw the contents of the window.
	fn draw(&mut self) -> Result<(), String> {
		// Always clear the whole window, to avoid artefacts.
		self.canvas.clear();

		// Redraw the image, if any.
		if let Some((texture, image_size)) = &self.texture {
			let image_area = if self.preserve_aspect_ratio {
				compute_target_rect_with_aspect_ratio(&image_size, &self.canvas.viewport().into())
			} else {
				self.canvas.viewport().into()
			};

			let scale_x = f64::from(image_area.width()) / f64::from(image_size.width());
			let scale_y = f64::from(image_area.height()) / f64::from(image_size.height());

			self.canvas.copy(&texture, None, SdlRect::from(&image_area))
				.map_err(|e| format!("failed to draw image: {}", e))?;

			for (i, (texture, size)) in self.overlays.iter().enumerate() {
				// Draw overlays with the same scaling applied.
				let dest_width = (f64::from(size.width()) * scale_x) as u32;
				let dest_height = (f64::from(size.height()) * scale_y) as u32;
				let dest_area = SdlRect::from(Rectangle::from_xywh(image_area.x(), image_area.y(), dest_width, dest_height));
				self.canvas.copy(&texture, None, SdlRect::from(dest_area))
					.map_err(|e| format!("failed to draw overlay {}: {}", i, e))?;
			}
		}

		self.canvas.present();
		Ok(())
	}

	/// Close the window.
	fn close(&mut self) {
		self.canvas.window_mut().hide();
	}

	#[cfg(feature = "save")]
	fn save_image(&mut self) -> Option<BackgroundThread<()>> {
		let image = self.image.as_ref()?.clone();

		Some(BackgroundThread::new(move || {
			let _ = crate::prompt_save_image(&format!("{}.png", image.name), &image.data, image.info);
		}))
	}
}

/// Convert an SDL2 event to the more generic Event.
fn convert_event(event: SdlEvent, focused_windows: &Vec<u32>) -> Option<(u32, Event)> {
	match event {
		SdlEvent::KeyDown { window_id, keycode, scancode, keymod, repeat, .. } if !focused_windows.contains(&window_id) => {
			Some((window_id, Event::KeyboardEvent(convert_keyboard_event(KeyState::Down, keycode, scancode, keymod, repeat))))
		},
		SdlEvent::KeyUp { window_id, keycode, scancode, keymod, repeat, .. } if !focused_windows.contains(&window_id) => {
			Some((window_id, Event::KeyboardEvent(convert_keyboard_event(KeyState::Up, keycode, scancode, keymod, repeat))))
		},
		SdlEvent::MouseMotion { window_id, which, mousestate, x, y, xrel, yrel, .. } if !focused_windows.contains(&window_id) => {
			Some((window_id, Event::MouseMoveEvent(convert_mouse_move_event(which, mousestate, x, y, xrel, yrel))))
		},
		SdlEvent::MouseButtonDown { window_id, which, mouse_btn, clicks, x, y, .. } if !focused_windows.contains(&window_id) => {
			Some((window_id, Event::MouseButtonEvent(convert_mouse_button_event(which, MouseState::Down, mouse_btn, clicks, x, y))))
		},
		SdlEvent::MouseButtonUp { window_id, which, mouse_btn, clicks, x, y, .. } if !focused_windows.contains(&window_id) => {
			Some((window_id, Event::MouseButtonEvent(convert_mouse_button_event(which, MouseState::Up, mouse_btn, clicks, x, y))))
		},
		_ => None,
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

/// Convert an SDL2 mouse state to the more generic MouseButtonState.
fn convert_mouse_state(
	mouse_state: sdl2::mouse::MouseState,
) -> MouseButtonState {
	MouseButtonState {
		left: mouse_state.left(),
		middle: mouse_state.middle(),
		right: mouse_state.right(),
	}
}

/// Convert an SDL2 mouse motion event to the more generic MouseMoveEvent.
fn convert_mouse_move_event(
	mouse_id: u32,
	mouse_state: sdl2::mouse::MouseState,
	x: i32,
	y: i32,
	xrel: i32,
	yrel: i32,
) -> MouseMoveEvent {
	MouseMoveEvent {
		mouse_id,
		mouse_state: convert_mouse_state(mouse_state),
		position_x: x,
		position_y: y,
		relative_x: xrel,
		relative_y: yrel,
	}
}

/// Convert an SDL2 mouse button to the more generic MouseButton.
fn convert_mouse_button(
	mouse_button: sdl2::mouse::MouseButton,
) -> MouseButton {
	match mouse_button {
		sdl2::mouse::MouseButton::Left => MouseButton::Left,
		sdl2::mouse::MouseButton::Middle => MouseButton::Middle,
		sdl2::mouse::MouseButton::Right => MouseButton::Right,
		_ => MouseButton::Unknown,
	}
}

/// Convert an SDL2 mouse button event to the more generic MouseButtonEvent.
fn convert_mouse_button_event(
	mouse_id: u32,
	state: MouseState,
	button: sdl2::mouse::MouseButton,
	clicks: u8,
	x: i32,
	y: i32,
) -> MouseButtonEvent {
	MouseButtonEvent {
		mouse_id,
		state,
		button: convert_mouse_button(button),
		clicks,
		position_x: x,
		position_y: y,
	}
}

fn compute_target_rect_with_aspect_ratio(source: &Rectangle, canvas: &Rectangle) -> Rectangle {
	let source_w = f64::from(source.width());
	let source_h = f64::from(source.height());
	let canvas_w = f64::from(canvas.width());
	let canvas_h = f64::from(canvas.height());

	let scale_w = canvas_w / source_w;
	let scale_h = canvas_h / source_h;

	if scale_w < scale_h {
		let new_height = (source_h * scale_w).round() as u32;
		let top = (canvas.height() - new_height) / 2;
		Rectangle::from_xywh(canvas.x(), canvas.y() + top as i32, canvas.width(), new_height)
	} else {
		let new_width = (source_w * scale_h).round() as u32;
		let left = (canvas.width() - new_width) / 2;
		Rectangle::from_xywh(canvas.x() + left as i32, canvas.y(), new_width, canvas.height())
	}
}
