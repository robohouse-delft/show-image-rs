use crate::Color;
use crate::ContextHandle;
use crate::EventHandlerOutput;
use crate::Image;
use crate::WindowId;
use crate::backend::util::GpuImage;
use crate::backend::util::UniformsBuffer;
use crate::error::InvalidWindowIdError;
use crate::event::WindowEvent;

/// A window.
pub struct Window<UserEvent: 'static> {
	/// The winit window.
	pub(crate) window: winit::window::Window,

	/// The window options.
	pub(crate) options: WindowOptions,

	/// The wgpu surface to render to.
	pub(crate) surface: wgpu::Surface,

	/// The swap chain for the surface.
	pub(crate) swap_chain: wgpu::SwapChain,

	/// The window specific uniforms for the render pipeline.
	pub(crate) uniforms: UniformsBuffer<WindowUniforms>,

	/// The image to display (if any).
	pub(crate) image: Option<GpuImage>,

	/// The event handlers for this specific window.
	pub(crate) event_handlers: Vec<Box<dyn FnMut(WindowHandle<UserEvent>, &mut crate::event::WindowEvent) -> EventHandlerOutput>>,
}

/// A handle to a window.
pub struct WindowHandle<'a, UserEvent: 'static> {
	/// The context handle to use.
	context_handle: ContextHandle<'a, UserEvent>,

	/// The window ID of the managed window.
	window_id: WindowId,
}

impl<'a, UserEvent> WindowHandle<'a, UserEvent> {
	/// Create a new window handle from a context handle and a window ID.
	pub fn new(context_handle: ContextHandle<'a, UserEvent>, window_id: WindowId) -> Self {
		Self { context_handle, window_id }
	}

	/// Get the window ID.
	pub fn id(&self) -> WindowId {
		self.window_id
	}

	/// Get the context handle as mutable reference.
	pub fn context_handle(&mut self) -> &mut ContextHandle<'a, UserEvent> {
		&mut self.context_handle
	}

	/// Destroy the window.
	pub fn destroy(mut self) -> Result<ContextHandle<'a, UserEvent>, InvalidWindowIdError> {
		self.context_handle.destroy_window(self.window_id)?;
		Ok(self.context_handle)
	}

	/// Make the window visible or invisible.
	pub fn set_visible(&mut self, visible: bool) -> Result<(), InvalidWindowIdError> {
		self.context_handle.set_window_visible(self.window_id, visible)
	}

	/// Set the image to display on the window.
	pub fn set_image(&mut self, name: impl AsRef<str>, image: &Image) -> Result<(), InvalidWindowIdError> {
		self.context_handle.set_window_image(self.window_id, name.as_ref(), image)
	}

	/// Add an event handler to the window.
	pub fn add_event_handler<F>(&mut self, handler: F) -> Result<(), InvalidWindowIdError>
	where
		F: 'static + FnMut(WindowHandle<UserEvent>, &mut WindowEvent) -> EventHandlerOutput,
	{
		self.context_handle.add_window_event_handler(self.window_id, handler)
	}

	/// Add an event handler to the window.
	///
	/// This does the same as [`Self::add_event_handler`],
	/// but doesn't add another layer of boxing if you already have a boxed function.
	pub fn add_boxed_event_handler(
		&mut self,
		handler: Box<dyn FnMut(WindowHandle<UserEvent>, &mut WindowEvent) -> EventHandlerOutput>,
	) -> Result<(), InvalidWindowIdError> {
		self.context_handle.add_boxed_window_event_handler(self.window_id, handler)
	}
}

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
			background_color: Color::BLACK,
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

impl<UserEvent> Window<UserEvent> {
	/// Get the window ID.
	pub fn id(&self) -> WindowId {
		self.window.id()
	}

	/// Make the window visible or invisible.
	pub fn set_visible(&mut self, visible: bool) {
		self.window.set_visible(visible);
	}

	/// Recalculate the uniforms for the render pipeline from the window state.
	pub(crate) fn calculate_uniforms(&self) -> WindowUniforms {
		WindowUniforms {
			scale: self.calculate_scale(),
		}
	}

	/// Calculate the image size in normalized window coordinates.
	///
	/// The normalized window coordinates go from (0, 0) to (1, 1).
	fn calculate_scale(&self) -> [f32; 2] {
		if !self.options.preserve_aspect_ratio {
			[1.0, 1.0]
		} else if let Some(image) = &self.image {
			let image_size = [image.width() as f32, image.height() as f32];
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

/// The window specific uniforms for the render pipeline.
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
