use crate::backend::Context;
use crate::backend::util::GpuImage;
use crate::backend::util::UniformsBuffer;
use crate::event::EventHandlerControlFlow;
use crate::event::WindowEvent;
use crate::Color;
use crate::ContextHandle;
use crate::ImageInfo;
use crate::ImageView;
use crate::Rectangle;
use crate::WindowId;
use crate::WindowProxy;

/// Internal shorthand for window event handlers.
type DynWindowEventHandler = dyn FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow);

/// Window capable of displaying images using wgpu.
pub struct Window {
	/// The winit window.
	pub window: winit::window::Window,

	/// If true, preserve the aspect ratio of images.
	pub preserve_aspect_ratio: bool,

	/// The background color of the window.
	pub background_color: Color,

	/// If true, draw overlays on top of the main image.
	pub overlays_visible: bool,

	/// The wgpu surface to render to.
	pub surface: wgpu::Surface,

	/// The swap chain for the surface.
	pub swap_chain: wgpu::SwapChain,

	/// The window specific uniforms for the render pipeline.
	pub uniforms: UniformsBuffer<WindowUniforms>,

	/// The image to display (if any).
	pub image: Option<GpuImage>,

	/// The zoom of the image.
	pub zoom: f32,

	/// The translation of the image.
	/// This determines how much the image is translated along each axis.
	/// A positive X value moves the image to the right and positive Y value moves it down.
	pub translate: [f32; 2],

	/// Overlays to draw on top of images.
	pub overlays: Vec<GpuImage>,

	/// The event handlers for this specific window.
	pub event_handlers: Vec<Box<DynWindowEventHandler>>,
}

/// Handle to a window.
///
/// A [`WindowHandle`] can be used to interact with a window from within the global context thread.
/// To interact with a window from another thread, you need a [`WindowProxy`].
pub struct WindowHandle<'a> {
	/// The context handle to use.
	context_handle: ContextHandle<'a>,

	/// The index of the window in [`Context::windows`].
	index: usize,
}

impl<'a> WindowHandle<'a> {
	/// Create a new window handle from a context handle and a window ID.
	pub fn new(context_handle: ContextHandle<'a>, index: usize) -> Self {
		Self { context_handle, index }
	}

	/// Get a reference to the context.
	fn context(&self) -> &Context {
		self.context_handle().context
	}

	/// Get a mutable reference to the context.
	///
	/// # Safety
	/// The current window may not be moved or removed through the returned reference.
	/// In practise, this means that you may not create or destroy any windows.
	unsafe fn context_mut(&mut self) -> &mut Context {
		&mut self.context_handle.context
	}

	/// Get a reference to the window.
	fn window(&self) -> &Window {
		&self.context().windows[self.index]
	}

	/// Get a mutable reference to the window.
	fn window_mut(&mut self) -> &mut Window {
		let index = self.index;
		unsafe { &mut self.context_mut().windows[index] }
	}

	/// Get the window ID.
	pub fn id(&self) -> WindowId {
		self.window().id()
	}

	/// Get a proxy object for the window to interact with it from a different thread.
	///
	/// You should not use proxy objects from withing the global context thread.
	/// The proxy objects often wait for the global context to perform some action.
	/// Doing so from within the global context thread would cause a deadlock.
	pub fn proxy(&self) -> WindowProxy {
		WindowProxy::new(self.id(), self.context_handle.proxy())
	}

	/// Release the window handle to get a [`ContextHandle`].
	///
	/// This can be used inside a window event handler to gain access to the [`ContextHandle`].
	/// If you do not need mutable access to the context, you can also use [`context_handle()`](Self::context_handle).
	pub fn release(self) -> ContextHandle<'a> {
		self.context_handle
	}

	/// Get a reference to the context handle.
	///
	/// If you need mutable access to the context, use [`release()`](Self::release) instead.
	pub fn context_handle(&self) -> &ContextHandle<'a> {
		&self.context_handle
	}

	/// Destroy the window.
	///
	/// Any subsequent operation on the window throuw an existing [`WindowProxy`] will return [`InvalidWindowId`](crate::error::InvalidWindowId).
	pub fn destroy(self) -> ContextHandle<'a> {
		let WindowHandle { context_handle, index } = self;
		context_handle.context.windows.remove(index);
		context_handle
	}

	/// Get the image info and the area of the window where the image is drawn.
	pub fn image_info(&self) -> Option<&ImageInfo> {
		Some(self.window().image.as_ref()?.info())
	}

	/// Get the area of the window where the image is drawn.
	pub fn image_area(&self) -> Rectangle {
		let uniforms = self.window().calculate_uniforms();
		let window_size = self.window().window.inner_size();

		let [x, y] = uniforms.offset;
		let [width, height] = uniforms.relative_size;

		let x = (x * window_size.width as f32) as i32;
		let y = (y * window_size.height as f32) as i32;
		let width = (width * window_size.width as f32) as u32;
		let height = (height * window_size.height as f32) as u32;

		Rectangle::from_xywh(x, y, width, height)
	}

	/// Check if the window will preserve the aspect ratio of images it displays.
	pub fn preserve_aspect_ratio(&self) -> bool {
		self.window().preserve_aspect_ratio
	}

	/// Set if the window will preserve the aspect ratio of images it displays.
	pub fn set_preserve_aspect_ratio(&mut self, preserve_aspect_ratio: bool) {
		self.window_mut().preserve_aspect_ratio = preserve_aspect_ratio;
		self.window().window.request_redraw();
	}

	/// Get the background color of the window.
	pub fn background_color(&self) -> Color {
		self.window().background_color
	}

	/// Set the background color of the window.
	pub fn set_background_color(&mut self, background_color: Color) {
		self.window_mut().background_color = background_color;
		self.window().window.request_redraw();
	}

	/// Make the window visible or invisible.
	pub fn set_visible(&mut self, visible: bool) {
		self.window_mut().set_visible(visible);
		self.window().window.request_redraw();
	}

	/// Get the inner size of the window in pixels.
	///
	/// This returns the size of the window contents, excluding borders, the title bar and other decorations.
	pub fn inner_size(&self) -> [u32; 2] {
		self.window().window.inner_size().into()
	}

	/// Get the outer size of the window in pixel.
	///
	/// This returns the size of the entire window, including borders, the title bar and other decorations.
	pub fn outer_size(&self) -> [u32; 2] {
		self.window().window.outer_size().into()
	}

	/// Set the inner size of the window in pixels.
	///
	/// The size is excluding borders, the title bar and other decorations.
	///
	/// Some window managers may ignore this property.
	pub fn set_inner_size(&mut self, size: [u32; 2]) {
		self.window_mut().window.set_inner_size(winit::dpi::PhysicalSize::<u32>::from(size));
		self.window().window.request_redraw();
	}

	/// Set if the window should be resizable for the user.
	///
	/// Some window managers may ignore this property.
	pub fn set_resizable(&mut self, resizable: bool) {
		self.window().window.set_resizable(resizable);
	}

	/// Set if the window should be drawn without borders.
	///
	/// Some window managers may ignore this property.
	pub fn set_borderless(&mut self, borderless: bool) {
		self.window().window.set_decorations(!borderless);
	}

	/// Check if the window is currently showing overlays.
	pub fn overlays_visible(&self) -> bool {
		self.window().overlays_visible
	}

	/// Enable or disable the overlays for this window.
	pub fn set_overlays_visible(&mut self, overlays_visible: bool) {
		self.window_mut().overlays_visible = overlays_visible;
		self.window().window.request_redraw()
	}

	/// Set the image to display on the window.
	pub fn set_image(&mut self, name: impl Into<String>, image: &ImageView) {
		let image = self.context().make_gpu_image(name, image);
		self.window_mut().image = Some(image);
		self.window_mut().uniforms.mark_dirty(true);
		self.window_mut().window.request_redraw();
	}

	/// Add an overlay to the window.
	///
	/// Overlays are drawn on top of the image.
	/// Overlays remain active until you call they are cleared.
	pub fn add_overlay(&mut self, name: impl Into<String>, image: &ImageView) {
		let image = self.context().make_gpu_image(name, image);
		self.window_mut().overlays.push(image);
		self.window().window.request_redraw()
	}

	/// Clear the overlays of the window.
	pub fn clear_overlays(&mut self) {
		self.window_mut().overlays.clear();
		self.window().window.request_redraw()
	}

	/// Add an event handler to the window.
	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: 'static + FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow),
	{
		self.window_mut().event_handlers.push(Box::new(handler))
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
	/// This may be ignored by some window managers.
	pub size: Option<[u32; 2]>,

	/// If true allow the window to be resized.
	///
	/// This may be ignored by some window managers.
	pub resizable: bool,

	/// Make the window borderless.
	pub borderless: bool,

	/// If true, draw overlays on the image.
	///
	/// Defaults to true.
	pub overlays_visible: bool,
}

impl Default for WindowOptions {
	fn default() -> Self {
		Self::new()
	}
}

impl WindowOptions {
	/// Create new window options with default values.
	pub fn new() -> Self {
		Self {
			preserve_aspect_ratio: true,
			background_color: Color::black(),
			start_hidden: false,
			size: None,
			resizable: true,
			borderless: false,
			overlays_visible: true,
		}
	}

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
	/// Pass [`None`] to clear a previously set value,
	/// which will let the window manager choose the initial size.
	///
	/// This property may be ignored by some window managers.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_size(mut self, size: impl Into<Option<[u32; 2]>>) -> Self {
		self.size = size.into();
		self
	}

	/// Make the window resizable or not.
	///
	/// This property may be ignored by some window managers.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_resizable(mut self, resizable: bool) -> Self {
		self.resizable = resizable;
		self
	}

	/// Make the window borderless or not.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_borderless(mut self, borderless: bool) -> Self {
		self.borderless = borderless;
		self
	}

	/// Set whether or not overlays should be drawn on the window.
	pub fn set_show_overlays(mut self, overlays_visible: bool) -> Self {
		self.overlays_visible = overlays_visible;
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
		if let Some(image) = &self.image {
			let image_size = [image.info().width as f32, image.info().height as f32];
			if !self.preserve_aspect_ratio {
				WindowUniforms::stretch(image_size)
					.set_zoom(self.zoom)
					.set_translation(self.translate)
			} else {
				let window_size = [self.window.inner_size().width as f32, self.window.inner_size().height as f32];
				WindowUniforms::fit(window_size, image_size)
					.set_zoom(self.zoom)
					.set_translation(self.translate)
			}
		} else {
			WindowUniforms::no_image()
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
	pub relative_size: [f32; 2],

	/// The size of the image in pixels.
	pub pixel_size: [f32; 2],
}

impl WindowUniforms {
	pub fn no_image() -> Self {
		Self::stretch([0.0; 2])
	}

	pub fn stretch(pixel_size: [f32; 2]) -> Self {
		Self {
			offset: [0.0; 2],
			relative_size: [1.0; 2],
			pixel_size,
		}
	}

	pub fn fit(window_size: [f32; 2], image_size: [f32; 2]) -> Self {
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

		Self {
			offset: [0.5 - 0.5 * w, 0.5 - 0.5 * h],
			relative_size: [w, h],
			pixel_size: image_size,
		}
	}

	/// Set the zoom of the image.
	pub fn set_zoom(mut self, zoom: f32) -> Self {
		self.relative_size = [zoom * self.relative_size[0], zoom * self.relative_size[1]] ;
		self
	}

	/// Set the pan of the image.
	/// This determines how much the image is translated along each axis.
	/// A positive X value moves the image to the right and positive Y value moves it down.
	pub fn set_translation(mut self, translate: [f32; 2]) -> Self {
		self.offset = [self.offset[0] + translate[0], self.offset[1] + translate[1]];
		self
	}
}
