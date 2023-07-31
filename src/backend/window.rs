use crate::Color;
use crate::ContextHandle;
use crate::ImageInfo;
use crate::ImageView;
use crate::WindowId;
use crate::WindowProxy;
use crate::backend::Context;
use crate::backend::util::GpuImage;
use crate::backend::util::UniformsBuffer;
use crate::error;
use crate::event::EventHandlerControlFlow;
use crate::event::WindowEvent;
use glam::Vec3;
use glam::{Affine2, Vec2};
use indexmap::IndexMap;

/// Internal shorthand for window event handlers.
type DynWindowEventHandler = dyn FnMut(WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow);

/// Window capable of displaying images using wgpu.
pub(crate) struct Window {
	/// The winit window.
	pub window: winit::window::Window,

	/// If true, preserve the aspect ratio of images.
	pub preserve_aspect_ratio: bool,

	/// The background color of the window.
	pub background_color: Color,

	/// The wgpu surface to render to.
	pub surface: wgpu::Surface,

	/// The window specific uniforms for the render pipeline.
	pub uniforms: UniformsBuffer<WindowUniforms>,

	/// The image to display (if any).
	pub image: Option<GpuImage>,

	/// Overlays for the window.
	pub overlays: IndexMap<String, Overlay>,

	/// Transformation to apply to the image, in virtual window space.
	///
	/// Virtual window space goes from (0, 0) in the top left to (1, 1) in the bottom right.
	pub user_transform: Affine2,

	/// The event handlers for this specific window.
	pub event_handlers: Vec<Box<DynWindowEventHandler>>,
}

/// An overlay added to a window.
pub(crate) struct Overlay {
	/// The image to show.
	pub image: GpuImage,

	/// If true, show the overlay, otherwise do not.
	pub visible: bool,
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

	/// Flag to signal to the handle creator that the window was destroyed.
	destroy_flag: Option<&'a mut bool>,
}

impl<'a> WindowHandle<'a> {
	/// Create a new window handle from a context handle and a window ID.
	pub fn new(context_handle: ContextHandle<'a>, index: usize, destroy_flag: Option<&'a mut bool>) -> Self {
		Self { context_handle, index, destroy_flag }
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
		self.context_handle.context
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
	/// Any subsequent operation on the window through an existing [`WindowProxy`] will return [`InvalidWindowId`](crate::error::InvalidWindowId).
	pub fn destroy(self) -> ContextHandle<'a> {
		let WindowHandle { context_handle, index, destroy_flag } = self;
		context_handle.context.windows.remove(index);
		if let Some(destroy_flag) =  destroy_flag {
			*destroy_flag = true;
		}
		context_handle
	}

	/// Get the image info.
	///
	/// Returns [`None`] if no image is set for the window.
	pub fn image_info(&self) -> Option<&ImageInfo> {
		Some(self.window().image.as_ref()?.info())
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

	/// Set the window position in pixels.
	///
	/// This will automatically un-maximize the window.
	///
	/// Some window managers or platforms may ignore this property.
	pub fn set_outer_position(&self, position: impl Into<glam::IVec2>) {
		let position = position.into();
		self.window().window.set_outer_position(winit::dpi::PhysicalPosition::new(position.x, position.y));
	}

	/// Get the inner size of the window in physical pixels.
	///
	/// This returns the size of the window contents, excluding borders, the title bar and other decorations.
	pub fn inner_size(&self) -> glam::UVec2 {
		let size = self.window().window.inner_size();
		glam::UVec2::new(size.width, size.height)
	}

	/// Get the outer size of the window in physical pixels.
	///
	/// This returns the size of the entire window, including borders, the title bar and other decorations.
	pub fn outer_size(&self) -> glam::UVec2 {
		let size = self.window().window.outer_size();
		glam::UVec2::new(size.width, size.height)
	}

	/// Set the inner size of the window in pixels.
	///
	/// The size is excluding borders, the title bar and other decorations.
	///
	/// Some window managers may ignore this property.
	pub fn set_inner_size(&mut self, size: impl Into<glam::UVec2>) {
		let size = size.into();
		self.window_mut().window.set_inner_size(winit::dpi::PhysicalSize::new(size.x, size.y));
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

	/// Set the window in fullscreen mode or back.
	///
	/// This will set the window to borderless fullscreen on the current monitor or back.
	/// Fullscreen is set if the argument is `true`, otherwise the window is returned to normal size.
	///
	/// Some window managers may ignore this property.
	pub fn set_fullscreen(&mut self, fullscreen: bool) {
		let opt = if fullscreen {
			Some(winit::window::Fullscreen::Borderless(None))
		} else {
			None
		};
		self.window().window.set_fullscreen(opt);
	}

	/// Check if the window is set to fullscreen mode.
	///
	/// Note that some window managers may ignore the request for fullscreen mode.
	/// In that case, this function may return true while the window is not displayed in fullscreen mode.
	pub fn is_fullscreen(&self) -> bool {
		self.window().window.fullscreen().is_some()
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
	/// Overlays are drawn on top of the image in the order that they are first added.
	/// If you wish to change the order of existing overlays, you must remove and re-add the overlays.
	///
	/// If the window already has an overlay with the same name,
	/// the overlay is overwritten and the `initially_visible` argument is ignored.
	/// If you want to change the visibility of the overlay, you can call [`set_overlay_visible()`][Self::set_overlay_visible].
	/// If you do so before your function returns, it is guaranteed to have taken effect before the next redraw.
	pub fn set_overlay(&mut self, name: impl Into<String>, image: &ImageView, initially_visible: bool) {
		use indexmap::map::Entry;

		let name = name.into();
		let image = self.context().make_gpu_image(name.clone(), image);
		match self.window_mut().overlays.entry(name) {
			Entry::Occupied(mut entry) => {
				entry.get_mut().image = image;
			},
			Entry::Vacant(entry) => {
				entry.insert(Overlay {
					image,
					visible: initially_visible,
				});
			},
		};
		self.window().window.request_redraw()
	}

	/// Remove an overlay from the window.
	///
	/// Returns `true` if there was an overlay to remove.
	pub fn remove_overlay(&mut self, name: &impl AsRef<str>) -> bool {
		let removed = self.window_mut().overlays.remove(name.as_ref()).is_some();
		self.window().window.request_redraw();
		removed
	}

	/// Remove all overlays from the window.
	pub fn clear_overlays(&mut self) {
		self.window_mut().overlays.clear();
		self.window().window.request_redraw()
	}

	/// Check if an overlay is visible or not.
	pub fn is_overlay_visible(&mut self, name: impl AsRef<str>) -> Result<bool, error::UnknownOverlay> {
		Ok(self.window().get_overlay(name)?.visible)
	}

	/// Make a specific overlay visible or invisible for this window.
	///
	/// The overlay is not removed, but it will not be rendered anymore untill you make it visible again.
	pub fn set_overlay_visible(&mut self, name: impl AsRef<str>, visible: bool) -> Result<(), error::UnknownOverlay> {
		self.window_mut().get_overlay_mut(name)?.visible = visible;
		self.window().window.request_redraw();
		Ok(())
	}

	/// Toggle an overlay between visible and invisible.
	pub fn toggle_overlay_visible(&mut self, name: impl AsRef<str>) -> Result<(), error::UnknownOverlay> {
		let overlay = self.window_mut().get_overlay_mut(name)?;
		overlay.visible = !overlay.visible;
		self.window().window.request_redraw();
		Ok(())
	}

	/// Make all overlays visible or invisible for this window.
	pub fn set_all_overlays_visible(&mut self, visible: bool) {
		for (_name, overlay) in &mut self.window_mut().overlays {
			overlay.visible = visible;
		}
		self.window().window.request_redraw()
	}

	/// Add an event handler to the window.
	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: 'static + FnMut(WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow),
	{
		self.window_mut().event_handlers.push(Box::new(handler))
	}

	/// Get the image transformation.
	///
	/// The image transformation is applied to the image and all overlays in virtual window space.
	///
	/// Virtual window space goes from `(0, 0)` in the top left corner of the window to `(1, 1)` in the bottom right corner.
	///
	/// This transformation does not include scaling introduced by the [`Self::preserve_aspect_ratio()`] property.
	/// Use [`Self::effective_transform()`] if you need that.
	pub fn transform(&self) -> Affine2 {
		self.window().user_transform
	}

	/// Get the full effective transformation from image space to virtual window space.
	///
	/// This transformation maps the image coordinates to virtual window coordinates.
	/// Unlike [`Self::transform()`], this function returns a transformation that include the scaling introduced by the [`Self::preserve_aspect_ratio()`] property.
	/// This is useful to transform between window coordinates and image coordinates.
	///
	/// If no image is set on the window yet, this returns the same transformation as [`Self::transform()`].
	///
	/// Virtual window space goes from `(0, 0)` in the top left corner of the window to `(1, 1)` in the bottom right corner.
	///
	/// Note that physical pixel locations must be transformed to virtual window coordinates first.
	pub fn effective_transform(&self) -> Affine2 {
		self.window().calculate_uniforms().transform
	}

	/// Set the image transformation to a value.
	///
	/// The image transformation is applied to the image and all overlays in virtual window space.
	///
	/// Virtual window space goes from `(0, 0)` in the top left corner of the window to `(1, 1)` in the bottom right corner.
	///
	/// This transformation should not include any scaling related to the [`Self::preserve_aspect_ratio()`] property.
	pub fn set_transform(&mut self, transform: Affine2) {
		self.window_mut().user_transform = transform;
		self.window_mut().uniforms.mark_dirty(true);
		self.window().window.request_redraw();
	}

	/// Pre-apply a transformation to the existing image transformation.
	///
	/// This is equivalent to:
	/// ```
	/// # use show_image::{glam::Affine2, WindowHandle};
	/// # fn foo(window: &mut WindowHandle, transform: Affine2) {
	/// window.set_transform(transform * window.transform())
	/// # }
	/// ```
	///
	/// See [`Self::set_transform`] for more information about the image transformation.
	pub fn pre_apply_transform(&mut self, transform: Affine2) {
		self.set_transform(transform * self.transform());
	}

	/// Post-apply a transformation to the existing image transformation.
	///
	/// This is equivalent to:
	/// ```
	/// # use show_image::{glam::Affine2, WindowHandle};
	/// # fn foo(window: &mut WindowHandle, transform: Affine2) {
	/// window.set_transform(window.transform() * transform)
	/// # }
	/// ```
	///
	/// See [`Self::set_transform`] for more information about the image transformation.
	pub fn post_apply_transform(&mut self, transform: Affine2) {
		self.set_transform(self.transform() * transform)
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
	///
	/// This may be ignored by some window managers.
	pub borderless: bool,

	/// Make the window fullscreen.
	///
	/// This may be ignored by some window managers.
	pub fullscreen: bool,

	/// If true, draw overlays on the image.
	///
	/// Defaults to true.
	pub overlays_visible: bool,

	/// If true, enable default mouse based controls for panning and zooming the image.
	///
	/// Defaults to true.
	pub default_controls: bool,
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
			fullscreen: false,
			overlays_visible: true,
			default_controls: true,
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

	/// Make the window fullscreen or not.
	///
	/// This function consumes and returns `self` to allow daisy chaining.
	pub fn set_fullscreen(mut self, fullscreen: bool) -> Self {
		self.fullscreen = fullscreen;
		self
	}

	/// Set whether or not overlays should be drawn on the window.
	pub fn set_show_overlays(mut self, overlays_visible: bool) -> Self {
		self.overlays_visible = overlays_visible;
		self
	}

	/// Set whether or not default mouse controls for panning and zooming the image should be added.
	pub fn set_default_controls(mut self, default_controls: bool) -> Self {
		self.default_controls = default_controls;
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
			let image_size = image.info().size.as_vec2();
			if !self.preserve_aspect_ratio {
				WindowUniforms::stretch(image_size)
					.pre_apply_transform(self.user_transform)
			} else {
				let window_size = glam::UVec2::new(self.window.inner_size().width, self.window.inner_size().height).as_vec2();
				WindowUniforms::fit(window_size, image_size)
					.pre_apply_transform(self.user_transform)
			}
		} else {
			WindowUniforms {
				transform: self.user_transform,
				image_size: Vec2::new(0.0, 0.0),
			}
		}
	}

	fn get_overlay(&self, name: impl AsRef<str>) -> Result<&Overlay, error::UnknownOverlay> {
		let name = name.as_ref();
		self.overlays.get(name)
			.ok_or_else(|| error::UnknownOverlay { name: name.into() })
	}

	fn get_overlay_mut(&mut self, name: impl AsRef<str>) -> Result<&mut Overlay, error::UnknownOverlay> {
		let name = name.as_ref();
		self.overlays.get_mut(name)
			.ok_or_else(|| error::UnknownOverlay { name: name.into() })
	}
}

/// The window specific uniforms for the render pipeline.
#[derive(Debug, Copy, Clone)]
pub(crate) struct WindowUniforms {
	/// The transformation applied to the image.
	///
	/// With the identity transform, the image is stretched to the inner window size,
	/// without preserving the aspect ratio.
	pub transform: Affine2,

	/// The size of the image in pixels.
	pub image_size: Vec2,
}

impl WindowUniforms {
	pub fn no_image() -> Self {
		Self::stretch(Vec2::new(0.0, 0.0))
	}

	pub fn stretch(image_size: Vec2) -> Self {
		Self {
			transform: Affine2::IDENTITY,
			image_size,
		}
	}

	pub fn fit(window_size: Vec2, image_size: Vec2) -> Self {
		let ratios = image_size / window_size;

		let w;
		let h;
		if ratios.x >= ratios.y {
			w = 1.0;
			h = ratios.y / ratios.x;
		} else {
			w = ratios.x / ratios.y;
			h = 1.0;
		}

		let transform = Affine2::from_scale_angle_translation(Vec2::new(w, h), 0.0, 0.5 * Vec2::new(1.0 - w, 1.0 - h));
		Self {
			transform,
			image_size,
		}
	}

	/// Pre-apply a transformation.
	pub fn pre_apply_transform(mut self, transform: Affine2) -> Self {
		self.transform = transform * self.transform;
		self
	}
}

#[repr(C, align(8))]
#[derive(Debug, Copy, Clone)]
struct Vec2A8 {
	pub x: f32,
	pub y: f32,
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone)]
struct Vec3A16 {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Mat3x3 {
	pub cols: [Vec3A16; 3]
}

impl Vec2A8 {
	pub const fn new(x: f32, y: f32) -> Self {
		Self { x, y }
	}
}

impl Vec3A16 {
	pub const fn new(x: f32, y: f32, z: f32) -> Self {
		Self { x, y, z }
	}
}

impl Mat3x3 {
	pub const fn new(col0: Vec3A16, col1: Vec3A16, col2: Vec3A16) -> Self {
		Self {
			cols: [col0, col1, col2],
		}
	}
}

impl From<Vec2> for Vec2A8 {
	fn from(other: Vec2) -> Self {
		Self::new(other.x, other.y)
	}
}

impl From<Vec3> for Vec3A16 {
	fn from(other: Vec3) -> Self {
		Self::new(other.x, other.y, other.z)
	}
}

impl From<Affine2> for Mat3x3 {
	fn from(other: Affine2) -> Self {
		let x_axis = other.matrix2.x_axis;
		let y_axis = other.matrix2.y_axis;
		let z_axis = other.translation;
		Self::new(
			Vec3A16::new(x_axis.x, x_axis.y, 0.0),
			Vec3A16::new(y_axis.x, y_axis.y, 0.0),
			Vec3A16::new(z_axis.x, z_axis.y, 1.0),
		)
	}
}

/// Window specific unfiforms, layout compatible with glsl std140.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct WindowUniformsStd140 {
	image_size: Vec2A8,
	transform: Mat3x3,
}

unsafe impl crate::backend::util::ToStd140 for WindowUniforms {
	type Output = WindowUniformsStd140;

	fn to_std140(&self) -> Self::Output {
		Self::Output {
			image_size: self.image_size.into(),
			transform: self.transform.into(),
		}
	}
}

/// Event handler that implements the default controls.
pub(super) fn default_controls_handler(mut window: WindowHandle, event: &mut crate::event::WindowEvent, _control_flow: &mut crate::event::EventHandlerControlFlow) {
	match event {
		WindowEvent::MouseWheel(event) => {
			let delta = match event.delta {
				winit::event::MouseScrollDelta::LineDelta(_x, y) => y,
				winit::event::MouseScrollDelta::PixelDelta(delta) => delta.y as f32 / 20.0,
			};
			let scale = 1.1f32.powf(delta);

			let origin = event.position
				.map(|pos| pos / window.inner_size().as_vec2())
				.unwrap_or_else(|| glam::Vec2::new(0.5, 0.5));
			let transform = glam::Affine2::from_scale_angle_translation(glam::Vec2::splat(scale), 0.0, origin - scale * origin);
			window.pre_apply_transform(transform);
		},
		WindowEvent::MouseMove(event) => {
			if event.buttons.is_pressed(crate::event::MouseButton::Left) {
				let translation = (event.position - event.prev_position) / window.inner_size().as_vec2();
				window.pre_apply_transform(Affine2::from_translation(translation));
			}
		},
		_ => (),
	}
}
