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
	windows: Vec<Arc<Mutex<WindowInner>>>,
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
		let context = sdl2::init().map_err(|e| format!("Failed to initialize SDL2: {}", e))?;
		let video = context.video().map_err(|e| format!("Failed to get SDL2 video subsystem: {}", e))?;
		let events = context.event_pump().map_err(|e| format!("Failed to get SDL2 event pump: {}", e))?;

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

		let canvas = window.into_canvas().build().map_err(|e| format!("failed to create canvasr for window {:?}: {}", name, e))?;

		let texture_creator = canvas.texture_creator();

		let inner = Arc::new(Mutex::new(WindowInner {
			canvas,
			texture_creator,
			texture: None,
		}));

		self.windows.push(inner.clone());

		Ok(Window { inner })
	}

	pub fn run(&mut self) -> Result<(), String> {
		loop {
			for event in self.events.poll_iter() {
				// TODO
			}

			for window in &self.windows {
				let mut window = window.lock().unwrap();
				let texture = window.texture.take();
				if let Some((texture, image_size)) = texture {
					let viewport = window.canvas.viewport();
					window.canvas.copy(&texture, image_size.clone(), viewport)
						.map_err(|e| format!("failed to copy data to window: {}", e))?;
					window.texture = Some((texture, image_size));
				}
				window.canvas.present();
			}

			std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
		}
	}
}

impl Window {
	pub fn show(&mut self, image: &impl ImageData) -> Result<(), String> {
		let size = image.data().len();
		let data = unsafe { std::slice::from_raw_parts_mut(image.data().as_ptr() as *mut u8, size) };
		let info = image.info().map_err(|e| format!("failed to display imge: {}", e))?;

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
