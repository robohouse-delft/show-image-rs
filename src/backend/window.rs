use crate::AsImageView;
use crate::Color;
use crate::ContextHandle;
use crate::WindowId;
use crate::WindowProxy;
use crate::backend::util::GpuImage;
use crate::backend::util::UniformsBuffer;
use crate::error::InvalidWindowIdError;
use crate::error::SetImageError;
use crate::event::EventHandlerControlFlow;
use crate::event::WindowEvent;

/// Window capable of displaying images using wgpu.
pub struct Window {
	/// The winit window.
	pub window: winit::window::Window,

	/// The window options.
	pub options: WindowOptions,

	/// The wgpu surface to render to.
	pub surface: wgpu::Surface,

	/// The swap chain for the surface.
	pub swap_chain: wgpu::SwapChain,

	/// The window specific uniforms for the render pipeline.
	pub uniforms: UniformsBuffer<WindowUniforms>,

	/// The image to display (if any).
	pub image: Option<GpuImage>,

	/// The event handlers for this specific window.
	pub event_handlers: Vec<Box<dyn FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow)>>,
}

/// Handle to a window.
///
/// A [`WindowHandle`] can be used to interact with a window from within the global context thread.
/// To interact with a window from another thread, you need a [`WindowProxy`].
pub struct WindowHandle<'a> {
	/// The context handle to use.
	context_handle: ContextHandle<'a>,

	/// The window ID of the managed window.
	window_id: WindowId,
}

impl<'a> WindowHandle<'a> {
	/// Create a new window handle from a context handle and a window ID.
	pub fn new(context_handle: ContextHandle<'a>, window_id: WindowId) -> Self {
		Self { context_handle, window_id }
	}

	/// Get the window ID.
	pub fn id(&self) -> WindowId {
		self.window_id
	}

	/// Get a proxy object for the window to interact with it from a different thread.
	///
	/// You should not use proxy objects from withing the global context thread.
	/// The proxy objects often wait for the global context to perform some action.
	/// Doing so from within the global context thread would cause a deadlock.
	pub fn proxy(&self) -> WindowProxy {
		WindowProxy::new(self.window_id, self.context_handle.proxy())
	}

	/// Get the context handle as mutable reference.
	pub fn context_handle(&mut self) -> &mut ContextHandle<'a> {
		&mut self.context_handle
	}

	/// Destroy the window.
	///
	/// Any subsequent operation on the window will return [`InvalidWindowIdError`].
	pub fn destroy(&mut self) -> Result<(), InvalidWindowIdError> {
		self.context_handle.destroy_window(self.window_id)
	}

	/// Make the window visible or invisible.
	pub fn set_visible(&mut self, visible: bool) -> Result<(), InvalidWindowIdError> {
		self.context_handle.set_window_visible(self.window_id, visible)
	}

	/// Set the image to display on the window.
	pub fn set_image(&mut self, name: impl AsRef<str>, image: &impl AsImageView) -> Result<(), SetImageError> {
		self.context_handle.set_window_image(self.window_id, name.as_ref(), image)
	}

	/// Add an event handler to the window.
	pub fn add_event_handler<F>(&mut self, handler: F) -> Result<(), InvalidWindowIdError>
	where
		F: 'static + FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow),
	{
		self.context_handle.add_window_event_handler(self.window_id, handler)
	}
}

/// Options for creating a new window.
#[derive(Debug, Clone)]
pub struct WindowOptions {
	/// Preserve the aspect ratio of the image when scaling.
	pub preserve_aspect_ratio: bool,

	/// The background color for the window.
	///
	/// This is used to color areas without image data if `preserve_aspect_ratio` is true.
	pub background_color: Color,

	/// Create the window hidden.
	///
	/// The window can manually be made visible at a later time.
	pub start_hidden: bool,

	/// The initial size of the window in pixel.
	///
	/// This may be ignored by a window manager.
	pub size: Option<[u32; 2]>,

	/// If true allow the window to be resized.
	///
	/// This may be ignored by a window manager.
	pub resizable: bool,
}

impl Default for WindowOptions {
	fn default() -> Self {
		Self {
			preserve_aspect_ratio: true,
			background_color: Color::black(),
			start_hidden: false,
			size: None,
			resizable: true,
		}
	}
}

impl WindowOptions {
	/// Preserve the aspect ratio of displayed images, or not.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_preserve_aspect_ratio(mut self, preserve_aspect_ratio: bool) -> Self {
		self.preserve_aspect_ratio = preserve_aspect_ratio;
		self
	}
	/// Set the background color of the window.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_background_color(mut self, background_color: Color) -> Self {
		self.background_color = background_color;
		self
	}

	/// Start the window hidden.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_start_hidden(mut self, start_hidden: bool) -> Self {
		self.start_hidden = start_hidden;
		self
	}

	/// Set the initial size of the window.
	///
	/// This property may be ignored by a window manager.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_size(mut self, size: [u32; 2]) -> Self {
		self.size = Some(size);
		self
	}

	/// Make the window resizable or not.
	///
	/// This property may be ignored by a window manager.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_resizable(mut self, resizable: bool) -> Self {
		self.resizable = resizable;
		self
	}
}

impl Window {
	/// Get the window ID.
	pub fn id(&self) -> WindowId {
		self.window.id()
	}

	/// Make the window visible or invisible.
	pub fn set_visible(&mut self, visible: bool) {
		self.window.set_visible(visible);
	}

	/// Recalculate the uniforms for the render pipeline from the window state.
	pub fn calculate_uniforms(&self) -> WindowUniforms {
		if !self.options.preserve_aspect_ratio {
			Default::default()
		} else if let Some(image) = &self.image {
			let image_size = [image.width() as f32, image.height() as f32];
			let window_size = [self.window.inner_size().width as f32, self.window.inner_size().height as f32];
			let ratios = [image_size[0] / window_size[0], image_size[1] / window_size[1]];

			let w;
			let h;
			if ratios[0] >= ratios[1] {
				w = 1.0;
				h = ratios[1] / ratios[0];
			} else {
				w = ratios[0] / ratios[1];
				h = 1.0;
			}

			WindowUniforms {
				offset: [0.5 - 0.5 * w, 0.5 - 0.5 * h],
				size: [w, h],
			}
		} else {
			Default::default()
		}
	}
}

/// The window specific uniforms for the render pipeline.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WindowUniforms {
	/// The offset of the image in normalized window coordinates.
	///
	/// The normalized window coordinates go from (0, 0) to (1, 1).
	pub offset: [f32; 2],

	/// The size of the image in normalized window coordinates.
	///
	/// The normalized window coordinates go from (0, 0) to (1, 1).
	pub size: [f32; 2],
}

impl Default for WindowUniforms {
	fn default() -> Self {
		Self {
			offset: [0.0, 0.0],
			size: [1.0, 1.0],
		}
	}
}
