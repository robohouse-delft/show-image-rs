use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::surface::Surface;

use std::sync::Arc;
use std::sync::Mutex;

use crate::ImageData;
use crate::PixelFormat;

pub struct Context {
	video: sdl2::VideoSubsystem,
	events: sdl2::EventPump,
	windows: Vec<(u32, Arc<Mutex<WindowInner>>)>,
}

struct WindowInner {
	canvas: Canvas<sdl2::video::Window>,
	texture_creator: TextureCreator<sdl2::video::WindowContext>,
	texture: Option<(Texture<'static>, sdl2::rect::Rect)>,
}

pub struct Window {
	inner: Arc<Mutex<WindowInner>>,
}

impl Context {
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

	pub fn window(&mut self, name: &str) -> Result<Window, String> {
		let window = self.video.window(name, 800, 600)
			.borderless()
			.resizable()
			.build()
			.map_err(|e| format!("failed to create window {:?}: {}", name, e))?;

		let id = window.id();

		let canvas = window.into_canvas().build().map_err(|e| format!("failed to create canvas for window {:?}: {}", name, e))?;

		let texture_creator = canvas.texture_creator();

		let inner = WindowInner {
			canvas,
			texture_creator,
			texture: None,
		};
		let inner = Arc::new(Mutex::new(inner));

		self.windows.push((id, inner.clone()));

		Ok(Window { inner })
	}

	pub fn run(&mut self) -> Result<(), String> {
		let delay = std::time::Duration::from_nanos(1_000_000_000 / 60);
		let mut next_frame = std::time::Instant::now() + delay;
		loop {
			while let Some(event) = self.events.poll_event() {
				match event {
					Event::Window { window_id, win_event, .. } => self.handle_window_event(window_id, win_event)?,
					_ => (),
				}
			}

			for (_id, window) in &self.windows {
				let mut window = window.lock().unwrap();
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

			let now = std::time::Instant::now();
			if now < next_frame {
				std::thread::sleep(next_frame - now);
				next_frame += delay;
			} else {
				next_frame = now.max(next_frame + delay);
			}
		}
	}

	fn handle_window_event(&mut self, window_id: u32, event: WindowEvent) -> Result<(), String> {
		match event {
			WindowEvent::Close => self.close_window(window_id),
			_ => (),
		}
		// TODO;
		Ok(())
	}

	fn close_window(&mut self, window_id: u32) {
		if let Some(index) = self.windows.iter().position(|(id, _)| *id == window_id) {
			let (_id, window) = self.windows.remove(index);
			let mut window = window.lock().unwrap();
			window.texture = None;
			window.canvas.window_mut().hide();
		}
	}
}

impl Window {
	pub fn show(&mut self, image: &impl ImageData) -> Result<(), String> {
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
}
