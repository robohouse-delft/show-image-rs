use crate::Color;
use crate::WindowId;
use crate::util::Texture;
use crate::util::UniformsBuffer;

pub struct Window {
	pub(crate) window: winit::window::Window,
	pub(crate) options: WindowOptions,
	pub(crate) surface: wgpu::Surface,
	pub(crate) swap_chain: wgpu::SwapChain,
	pub(crate) uniforms: UniformsBuffer<WindowUniforms>,
	pub(crate) image: Option<Texture>,
	pub(crate) load_texture: Option<wgpu::CommandBuffer>,
}

#[derive(Debug, Clone)]
pub struct WindowOptions {
	pub preserve_aspect_ratio: bool,
	pub background_color: Color,
	pub start_hidden: bool,
}

impl Default for WindowOptions {
	fn default() -> Self {
		Self {
			preserve_aspect_ratio: true,
			background_color: Color::BLACK,
			start_hidden: false,
		}
	}
}

impl Window {
	pub fn id(&self) -> WindowId {
		self.window.id()
	}

	pub fn set_visible(&mut self, visible: bool) {
		self.window.set_visible(visible);
	}

	pub(crate) fn calculate_uniforms(&self) -> WindowUniforms {
		WindowUniforms {
			scale: self.calculate_scale(),
		}
	}

	fn calculate_scale(&self) -> [f32; 2] {
		if !self.options.preserve_aspect_ratio {
			[1.0, 1.0]
		} else if let Some(image) = &self.image {
			let image_size = [image.size().width as f32, image.size().height as f32];
			let window_size = [self.window.inner_size().width as f32, self.window.inner_size().height as f32];
			let ratios = [image_size[0] / window_size[0], image_size[1] / window_size[1]];

			if ratios[0] >= ratios[1] {
				[1.0, ratios[1] / ratios[0]]
			} else {
				[ratios[0] / ratios[1], 1.0]
			}
		} else {
			[1.0, 1.0]
		}
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WindowUniforms {
	pub scale: [f32; 2],
}

impl Default for WindowUniforms {
	fn default() -> Self {
		Self {
			scale: [1.0, 1.0],
		}
	}
}
