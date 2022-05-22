use core::num::NonZeroU64;
use crate::backend::proxy::ContextFunction;
use crate::backend::util::GpuImage;
use crate::backend::util::{ToStd140, UniformsBuffer};
use crate::backend::window::Window;
use crate::backend::window::WindowUniforms;
use crate::background_thread::BackgroundThread;
use crate::error::CreateWindowError;
use crate::error::GetDeviceError;
use crate::error::InvalidWindowId;
use crate::error::NoSuitableAdapterFound;
use crate::event::{self, Event, EventHandlerControlFlow, WindowEvent};
use crate::ContextProxy;
use crate::ImageView;
use crate::WindowHandle;
use crate::WindowId;
use crate::WindowOptions;
use glam::Affine2;

/// Internal shorthand type-alias for the correct [`winit::event_loop::EventLoop`].
///
/// Not for use in public APIs.
type EventLoop = winit::event_loop::EventLoop<ContextFunction>;

/// Internal shorthand for context event handlers.
///
/// Not for use in public APIs.
type DynContextEventHandler = dyn FnMut(&mut ContextHandle, &mut Event, &mut event::EventHandlerControlFlow);

/// Internal shorthand type-alias for the correct [`winit::event_loop::EventLoopWindowTarget`].
///
/// Not for use in public APIs.
type EventLoopWindowTarget = winit::event_loop::EventLoopWindowTarget<ContextFunction>;

impl From<crate::Color> for wgpu::Color {
	fn from(other: crate::Color) -> Self {
		Self {
			r: other.red,
			g: other.green,
			b: other.blue,
			a: other.alpha,
		}
	}
}

pub struct GpuContext {
	/// The wgpu device to use.
	pub device: wgpu::Device,

	/// The wgpu command queue to use.
	pub queue: wgpu::Queue,

	/// The bind group layout for the window specific bindings.
	pub window_bind_group_layout: wgpu::BindGroupLayout,

	/// The bind group layout for the image specific bindings.
	pub image_bind_group_layout: wgpu::BindGroupLayout,

	/// The render pipeline to use for windows.
	pub window_pipeline: wgpu::RenderPipeline,

	/// The render pipeline to use for rendering to image.
	pub image_pipeline: wgpu::RenderPipeline,
}

/// The global context managing all windows and the main event loop.
pub struct Context {
	/// Marker to make context !Send.
	pub unsend: std::marker::PhantomData<*const ()>,

	/// The wgpu instance to create surfaces with.
	pub instance: wgpu::Instance,

	/// GPU related context that can not be initialized until we have a valid surface.
	pub gpu: Option<GpuContext>,

	/// The event loop to use.
	///
	/// Running the event loop consumes it,
	/// so from that point on this field is `None`.
	pub event_loop: Option<EventLoop>,

	/// A proxy object to clone for new requests.
	pub proxy: ContextProxy,

	/// The swap chain format to use.
	pub swap_chain_format: wgpu::TextureFormat,

	/// The windows.
	pub windows: Vec<Window>,

	/// Cache for mouse state.
	pub mouse_cache: super::mouse_cache::MouseCache,

	/// If true, exit the program when the last window closes.
	pub exit_with_last_window: bool,

	/// The global event handlers.
	pub event_handlers: Vec<Box<DynContextEventHandler>>,

	/// Background tasks, like saving images.
	pub background_tasks: Vec<BackgroundThread<()>>,
}

/// Handle to the global context.
///
/// You can interact with the global context through a [`ContextHandle`] only from the global context thread.
/// To interact with the context from a different thread, use a [`ContextProxy`].
pub struct ContextHandle<'a> {
	pub(crate) context: &'a mut Context,
	pub(crate) event_loop: &'a EventLoopWindowTarget,
}

impl GpuContext {
	pub fn new(instance: &wgpu::Instance, swap_chain_format: wgpu::TextureFormat, surface: &wgpu::Surface) -> Result<Self, GetDeviceError> {
		let (device, queue) = futures::executor::block_on(get_device(instance, surface))?;
		device.on_uncaptured_error(|error| {
			panic!("Unhandled WGPU error: {}", error);
		});

		let window_bind_group_layout = create_window_bind_group_layout(&device);
		let image_bind_group_layout = create_image_bind_group_layout(&device);

		let vertex_shader = device.create_shader_module(&wgpu::include_spirv!("../../shaders/shader.vert.spv"));
		let fragment_shader_unorm8 = device.create_shader_module(&wgpu::include_spirv!("../../shaders/unorm8.frag.spv"));

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("show-image-pipeline-layout"),
			bind_group_layouts: &[&window_bind_group_layout, &image_bind_group_layout],
			push_constant_ranges: &[],
		});

		let window_pipeline = create_render_pipeline(
			&device,
			&pipeline_layout,
			&vertex_shader,
			&fragment_shader_unorm8,
			swap_chain_format,
		);
		let image_pipeline = create_render_pipeline(
			&device,
			&pipeline_layout,
			&vertex_shader,
			&fragment_shader_unorm8,
			wgpu::TextureFormat::Rgba8Unorm,
		);

		Ok(Self {
			device,
			queue,
			window_bind_group_layout,
			image_bind_group_layout,
			window_pipeline,
			image_pipeline,
		})
	}
}

impl Context {
	/// Create a new global context.
	///
	/// You can theoreticlly create as many contexts as you want,
	/// but they must be run from the main thread and the [`run`](Self::run) function never returns.
	/// So it is not possible to *run* more than one context.
	pub fn new(swap_chain_format: wgpu::TextureFormat) -> Result<Self, GetDeviceError> {
		let instance = wgpu::Instance::new(select_backend());
		let event_loop = EventLoop::with_user_event();
		let proxy = ContextProxy::new(event_loop.create_proxy(), std::thread::current().id());

		Ok(Self {
			unsend: Default::default(),
			instance,
			gpu: None,
			event_loop: Some(event_loop),
			proxy,
			swap_chain_format,
			windows: Vec::new(),
			mouse_cache: Default::default(),
			exit_with_last_window: false,
			event_handlers: Vec::new(),
			background_tasks: Vec::new(),
		})
	}

	/// Add a global event handler.
	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: 'static + FnMut(&mut ContextHandle, &mut Event, &mut EventHandlerControlFlow),
	{
		self.event_handlers.push(Box::new(handler))
	}

	/// Run the event loop of the context.
	///
	/// This function must be run from the main thread and never returns.
	///
	/// Not all platforms have this restriction,
	/// but for portability reasons it is enforced on all platforms.
	pub fn run(mut self) -> ! {
		let event_loop = self.event_loop.take().unwrap();
		event_loop.run(move |event, event_loop, control_flow| {
			let initial_window_count = self.windows.len();
			self.handle_event(event, event_loop, control_flow);

			// Check if the event handlers caused the last window(s) to close.
			// If so, generate an AllWIndowsClosed event for the event handlers.
			if self.windows.is_empty() && initial_window_count > 0 {
				self.run_event_handlers(&mut Event::AllWindowsClosed, event_loop);
				if self.exit_with_last_window {
					self.exit(0);
				}
			}
		});
	}
}

impl<'a> ContextHandle<'a> {
	/// Create a new context handle.
	fn new(context: &'a mut Context, event_loop: &'a EventLoopWindowTarget) -> Self {
		Self { context, event_loop }
	}

	/// Reborrow self with a shorter lifetime.
	pub(crate) fn reborrow(&mut self) -> ContextHandle {
		ContextHandle {
			context: self.context,
			event_loop: self.event_loop,
		}
	}

	/// Get a proxy for the context to interact with it from a different thread.
	///
	/// You should not use proxy objects from withing the global context thread.
	/// The proxy objects often wait for the global context to perform some action.
	/// Doing so from within the global context thread would cause a deadlock.
	pub fn proxy(&self) -> ContextProxy {
		self.context.proxy.clone()
	}

	/// Exit the program when the last window closes.
	pub fn set_exit_with_last_window(&mut self, exit_with_last_window: bool) {
		self.context.exit_with_last_window = exit_with_last_window;
	}

	/// Get a window handle for the given window ID.
	pub fn window(&mut self, window_id: WindowId) -> Result<WindowHandle, InvalidWindowId> {
		let index = self.context.windows.iter().position(|x| x.id() == window_id).ok_or(InvalidWindowId { window_id })?;
		Ok(WindowHandle::new(self.reborrow(), index, None))
	}

	/// Create a new window.
	pub fn create_window(&mut self, title: impl Into<String>, options: WindowOptions) -> Result<WindowHandle, CreateWindowError> {
		let index = self.context.create_window(self.event_loop, title, options)?;
		Ok(WindowHandle::new(self.reborrow(), index, None))
	}

	/// Add a global event handler.
	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: 'static + FnMut(&mut ContextHandle, &mut Event, &mut EventHandlerControlFlow),
	{
		self.context.add_event_handler(handler);
	}

	/// Run a task in a background thread and register it with the context.
	///
	/// The task will be executed in a different thread than the context.
	/// Currently, each task is spawned in a separate thread.
	/// In the future, tasks may be run in a dedicated thread pool.
	///
	/// The background task will be joined before the process is terminated when you use [`Self::exit()`] or one of the other exit functions of this crate.
	pub fn run_background_task<F>(&mut self, task: F)
	where
		F: FnOnce() + Send + 'static,
	{
		self.context.run_background_task(task);
	}

	/// Join all background tasks and then exit the process.
	///
	/// If you use [`std::process::exit`], running background tasks may be killed.
	/// To ensure no data loss occurs, you should use this function instead.
	///
	/// Background tasks are spawned when an image is saved through the built-in Ctrl+S or Ctrl+Shift+S shortcut, or by user code.
	pub fn exit(&mut self, code: i32) -> ! {
		self.context.exit(code);
	}
}

impl Context {
	/// Create a window.
	fn create_window(
		&mut self,
		event_loop: &EventLoopWindowTarget,
		title: impl Into<String>,
		options: WindowOptions,
	) -> Result<usize, CreateWindowError> {
		let fullscreen = if options.fullscreen {
			Some(winit::window::Fullscreen::Borderless(None))
		} else {
			None
		};
		let mut window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_visible(!options.start_hidden)
			.with_resizable(options.resizable)
			.with_decorations(!options.borderless)
			.with_fullscreen(fullscreen);

		if let Some(size) = options.size {
			window = window.with_inner_size(winit::dpi::PhysicalSize::new(size[0], size[1]));
		}

		let window = window.build(event_loop)?;
		let surface = unsafe { self.instance.create_surface(&window) };


		let gpu = match &self.gpu {
			Some(x) => x,
			None => {
				let gpu = GpuContext::new(&self.instance, self.swap_chain_format, &surface)?;
				self.gpu.insert(gpu)
			}
		};

		let size = glam::UVec2::new(window.inner_size().width, window.inner_size().height);
		configure_surface(size, &surface, self.swap_chain_format, &gpu.device);
		let uniforms = UniformsBuffer::from_value(&gpu.device, &WindowUniforms::no_image(), &gpu.window_bind_group_layout);

		let window = Window {
			window,
			preserve_aspect_ratio: options.preserve_aspect_ratio,
			background_color: options.background_color,
			surface,
			uniforms,
			image: None,
			user_transform: Affine2::IDENTITY,
			overlays: Vec::new(),
			overlays_visible: options.overlays_visible,
			event_handlers: Vec::new(),
		};

		self.windows.push(window);
		let index = self.windows.len() - 1;
		if options.default_controls {
			self.windows[index].event_handlers.push(Box::new(super::window::default_controls_handler));
		}
		Ok(index)
	}

	/// Destroy a window.
	fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowId> {
		let index = self
			.windows
			.iter()
			.position(|w| w.id() == window_id)
			.ok_or(InvalidWindowId { window_id })?;
		self.windows.remove(index);
		Ok(())
	}

	/// Upload an image to the GPU.
	pub fn make_gpu_image(&self, name: impl Into<String>, image: &ImageView) -> GpuImage {
		let gpu = self.gpu.as_ref().unwrap();
		GpuImage::from_data(name.into(), &gpu.device, &gpu.image_bind_group_layout, image)
	}

	/// Resize a window.
	fn resize_window(&mut self, window_id: WindowId, new_size: glam::UVec2) -> Result<(), InvalidWindowId> {
		let window = self
			.windows
			.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or(InvalidWindowId { window_id })?;

		let gpu = self.gpu.as_ref().unwrap();
		configure_surface(new_size, &window.surface, self.swap_chain_format, &gpu.device);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	/// Render the contents of a window.
	fn render_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowId> {
		let window = self
			.windows
			.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or(InvalidWindowId { window_id })?;

		let image = match &window.image {
			Some(x) => x,
			None => return Ok(()),
		};

		let frame = window
			.surface
			.get_current_texture()
			.expect("Failed to acquire next frame");

		let gpu = self.gpu.as_ref().unwrap();
		let mut encoder = gpu.device.create_command_encoder(&Default::default());

		if window.uniforms.is_dirty() {
			window
				.uniforms
				.update_from(&gpu.device, &mut encoder, &window.calculate_uniforms());
		}

		render_pass(
			&mut encoder,
			&gpu.window_pipeline,
			&window.uniforms,
			image,
			Some(window.background_color),
			&frame.texture.create_view(&wgpu::TextureViewDescriptor::default()),
		);
		if window.overlays_visible {
			for overlay in &window.overlays {
				render_pass(
					&mut encoder,
					&gpu.window_pipeline,
					&window.uniforms,
					overlay,
					None,
					&frame.texture.create_view(&wgpu::TextureViewDescriptor::default()),
				);
			}
		}
		gpu.queue.submit(std::iter::once(encoder.finish()));
		frame.present();
		Ok(())
	}

	#[cfg(feature = "save")]
	fn render_to_texture(&self, window_id: WindowId, overlays: bool) -> Result<Option<(String, crate::BoxImage)>, InvalidWindowId> {
		use core::num::NonZeroU32;

		let window = self
			.windows
			.iter()
			.find(|w| w.id() == window_id)
			.ok_or(InvalidWindowId { window_id })?;

		let image = match &window.image {
			Some(x) => x,
			None => return Ok(None),
		};

		let bytes_per_row = align_next_u32(image.info().size.x * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
		let width_scale = image.info().size.x as f32 * 4.0 / bytes_per_row as f32;

		let size = wgpu::Extent3d {
			width: div_round_up(bytes_per_row, 4),
			height: image.info().size.y,
			depth_or_array_layers: 1,
		};

		let gpu = self.gpu.as_ref().unwrap();
		let window_uniforms = WindowUniforms {
			transform: Affine2::from_scale([width_scale, 1.0].into()),
			image_size: image.info().size.as_vec2(),
		};
		let window_uniforms = UniformsBuffer::from_value(&gpu.device, &window_uniforms, &gpu.window_bind_group_layout);

		let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
			label: Some(&format!("{}_render", image.name())),
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
			sample_count: 1,
			mip_level_count: 1,
			format: wgpu::TextureFormat::Rgba8Unorm,
			dimension: wgpu::TextureDimension::D2,
			size,
		});

		let mut encoder = gpu.device.create_command_encoder(&Default::default());
		let transparent = crate::Color::rgba(0.0, 0.0, 0.0, 0.0);
		let render_target = target.create_view(&wgpu::TextureViewDescriptor {
			label: None,
			format: None,
			dimension: None,
			aspect: wgpu::TextureAspect::All,
			base_mip_level: 0,
			mip_level_count: None,
			base_array_layer: 0,
			array_layer_count: None,
		});

		render_pass(
			&mut encoder,
			&gpu.image_pipeline,
			&window_uniforms,
			image,
			Some(transparent),
			&render_target,
		);
		if overlays {
			for overlay in &window.overlays {
				render_pass(&mut encoder, &gpu.image_pipeline, &window_uniforms, overlay, None, &render_target);
			}
		}

		let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: u64::from(bytes_per_row) * u64::from(image.info().size.y),
			usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
			mapped_at_creation: false,
		});

		encoder.copy_texture_to_buffer(
			wgpu::ImageCopyTexture {
				texture: &target,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
				aspect: wgpu::TextureAspect::All,
			},
			wgpu::ImageCopyBuffer {
				buffer: &buffer,
				layout: wgpu::ImageDataLayout {
					offset: 0,
					bytes_per_row: NonZeroU32::new(bytes_per_row),
					rows_per_image: NonZeroU32::new(image.info().size.y),
				},
			},
			size,
		);

		gpu.queue.submit(std::iter::once(encoder.finish()));

		let view = super::util::map_buffer(&gpu.device, buffer.slice(..)).unwrap();
		let info = crate::ImageInfo {
			pixel_format: crate::PixelFormat::Rgba8(crate::Alpha::Unpremultiplied),
			size: image.info().size,
			stride: glam::UVec2::new(4, bytes_per_row),
		};
		let data: Box<[u8]> = Box::from(&view[..]);
		Ok(Some((image.name().to_string(), crate::BoxImage::new(info, data))))
	}

	/// Handle an event from the event loop.
	fn handle_event(
		&mut self,
		event: winit::event::Event<ContextFunction>,
		event_loop: &EventLoopWindowTarget,
		control_flow: &mut winit::event_loop::ControlFlow,
	) {
		*control_flow = winit::event_loop::ControlFlow::Wait;

		// Split between Event<ContextFunction> and ContextFunction commands.
		let event = match super::event::map_nonuser_event(event) {
			Ok(event) => event,
			Err(function) => {
				(function)(&mut ContextHandle::new(self, event_loop));
				return;
			},
		};

		self.mouse_cache.handle_event(&event);

		// Convert to own event type.
		let mut event = match super::event::convert_winit_event(event, &self.mouse_cache) {
			Some(x) => x,
			None => return,
		};

		// If we have nothing more to do, clean the background tasks.
		if let Event::MainEventsCleared = &event {
			self.clean_background_tasks();
		}

		// Run window event handlers.
		let run_context_handlers = match &mut event {
			Event::WindowEvent(event) => self.run_window_event_handlers(event, event_loop),
			_ => true,
		};

		// Run context event handlers.
		if run_context_handlers {
			self.run_event_handlers(&mut event, event_loop);
		}

		// Perform default actions for events.
		match event {
			#[cfg(feature = "save")]
			#[allow(deprecated)]
			Event::WindowEvent(WindowEvent::KeyboardInput(event)) => {
				if event.input.state.is_pressed() && event.input.key_code == Some(event::VirtualKeyCode::S) {
					let overlays = event.input.modifiers.alt();
					let modifiers = event.input.modifiers & !event::ModifiersState::ALT;
					if modifiers == event::ModifiersState::CTRL {
						self.save_image_prompt(event.window_id, overlays);
					} else if modifiers == event::ModifiersState::CTRL | event::ModifiersState::SHIFT {
						self.save_image(event.window_id, overlays);
					}
				}
			},
			Event::WindowEvent(WindowEvent::Resized(event)) => {
				if event.size.x > 0 && event.size.y > 0 {
					let _ = self.resize_window(event.window_id, event.size);
				}
			},
			Event::WindowEvent(WindowEvent::RedrawRequested(event)) => {
				let _ = self.render_window(event.window_id);
			},
			Event::WindowEvent(WindowEvent::CloseRequested(event)) => {
				let _ = self.destroy_window(event.window_id);
			},
			_ => {},
		}
	}

	/// Run global event handlers.
	fn run_event_handlers(&mut self, event: &mut Event, event_loop: &EventLoopWindowTarget) {
		use super::util::RetainMut;

		// Event handlers could potentially modify the list of event handlers.
		// Also, even if they couldn't we'd still need borrow self mutably multiple times to run the event handlers.
		// That's not allowed, of course, so temporarily swap the event handlers with a new vector.
		// When we've run all handlers, we add the new handlers to the original vector and place it back.
		// https://newfastuff.com/wp-content/uploads/2019/05/dVIkgAf.png
		let mut event_handlers = std::mem::take(&mut self.event_handlers);

		let mut stop_propagation = false;
		RetainMut::retain_mut(&mut event_handlers, |handler| {
			if stop_propagation {
				true
			} else {
				let mut context_handle = ContextHandle::new(self, event_loop);
				let mut control = EventHandlerControlFlow::default();
				(handler)(&mut context_handle, event, &mut control);
				stop_propagation = control.stop_propagation;
				!control.remove_handler
			}
		});

		let new_event_handlers = std::mem::take(&mut self.event_handlers);
		event_handlers.extend(new_event_handlers);
		self.event_handlers = event_handlers;
	}

	/// Run window-specific event handlers.
	fn run_window_event_handlers(&mut self, event: &mut WindowEvent, event_loop: &EventLoopWindowTarget) -> bool {
		use super::util::RetainMut;

		let window_index = match self.windows.iter().position(|x| x.id() == event.window_id()) {
			Some(x) => x,
			None => return true,
		};

		let mut event_handlers = std::mem::take(&mut self.windows[window_index].event_handlers);

		let mut stop_propagation = false;
		let mut window_destroyed = false;
		RetainMut::retain_mut(&mut event_handlers, |handler| {
			if window_destroyed || stop_propagation {
				true
			} else {
				let context_handle = ContextHandle::new(self, event_loop);
				let window_handle = WindowHandle::new(context_handle, window_index, Some(&mut window_destroyed));
				let mut control = EventHandlerControlFlow::default();
				(handler)(window_handle, event, &mut control);
				stop_propagation = control.stop_propagation;
				!control.remove_handler
			}
		});

		if !window_destroyed {
			let new_event_handlers = std::mem::take(&mut self.windows[window_index].event_handlers);
			event_handlers.extend(new_event_handlers);
			self.windows[window_index].event_handlers = event_handlers;
		}

		!stop_propagation && !window_destroyed
	}

	/// Run a background task in a separate thread.
	fn run_background_task<F>(&mut self, task: F)
	where
		F: FnOnce() + Send + 'static,
	{
		self.background_tasks.push(BackgroundThread::new(task))
	}

	/// Clean-up finished background tasks.
	fn clean_background_tasks(&mut self) {
		self.background_tasks.retain(|task| !task.is_done());
	}

	/// Join all background tasks.
	fn join_background_tasks(&mut self) {
		for task in std::mem::take(&mut self.background_tasks) {
			task.join().unwrap();
		}
	}

	/// Join all background tasks and then exit the process.
	fn exit(&mut self, code: i32) -> ! {
		self.join_background_tasks();
		std::process::exit(code);
	}

	#[cfg(feature = "save")]
	fn save_image_prompt(&mut self, window_id: WindowId, overlays: bool) {
		let (name, image) = match self.render_to_texture(window_id, overlays) {
			Ok(Some(x)) => x,
			Ok(None) => return,
			Err(e) => return log::error!("failed to render window contents: {}", e),
		};

		let info = image.info();
		let name = format!("{}.png", name);
		self.run_background_task(move || {
			let path = match tinyfiledialogs::save_file_dialog("Save image", &name) {
				Some(x) => x,
				_ => return,
			};
			if let Err(e) = crate::save_rgba8_image(&path, image.data(), info.size, info.stride.y) {
				log::error!("failed to save image to {}: {}", path, e);
			}
		});
	}

	#[cfg(feature = "save")]
	fn save_image(&mut self, window_id: WindowId, overlays: bool) {
		let (name, image) = match self.render_to_texture(window_id, overlays) {
			Ok(Some(x)) => x,
			Ok(None) => return,
			Err(e) => return log::error!("failed to render window contents: {}", e),
		};

		let info = image.info();
		let name = format!("{}.png", name);
		self.run_background_task(move || {
			if let Err(e) = crate::save_rgba8_image(&name, image.data(), info.size, info.stride.y) {
				log::error!("failed to save image to {}: {}", name, e);
			}
		});
	}
}

fn select_backend() -> wgpu::Backends {
	let backend = std::env::var_os("WGPU_BACKEND").unwrap_or_else(|| "primary".into());
	let backend = match backend.to_str() {
		Some(backend) => backend,
		None => {
			eprintln!("Unknown WGPU_BACKEND: {:?}", backend);
			std::process::exit(1);
		}
	};

	if backend.eq_ignore_ascii_case("primary") {
		wgpu::Backends::PRIMARY
	} else if backend.eq_ignore_ascii_case("vulkan") {
		wgpu::Backends::VULKAN
	} else if backend.eq_ignore_ascii_case("metal") {
		wgpu::Backends::METAL
	} else if backend.eq_ignore_ascii_case("dx12") {
		wgpu::Backends::DX12
	} else if backend.eq_ignore_ascii_case("dx11") {
		wgpu::Backends::DX11
	} else if backend.eq_ignore_ascii_case("gl") {
		wgpu::Backends::GL
	} else if backend.eq_ignore_ascii_case("webgpu") {
		wgpu::Backends::BROWSER_WEBGPU
	} else {
		eprintln!("Unknown WGPU_BACKEND: {:?}", backend);
		std::process::exit(1);
	}
}

fn select_power_preference() -> wgpu::PowerPreference {
	let power_pref = std::env::var_os("WGPU_POWER_PREF").unwrap_or_else(|| "low".into());
	let power_pref = match power_pref.to_str() {
		Some(power_pref) => power_pref,
		None => {
			eprintln!("Unknown WGPU_POWER_PREF: {:?}", power_pref);
			std::process::exit(1);
		}
	};

	if power_pref.eq_ignore_ascii_case("low") {
		wgpu::PowerPreference::LowPower
	} else if power_pref.eq_ignore_ascii_case("high") {
		wgpu::PowerPreference::HighPerformance
	} else {
		eprintln!("Unknown WGPU_POWER_PREF: {:?}", power_pref);
		std::process::exit(1);
	}
}

/// Get a wgpu device to use.
async fn get_device(instance: &wgpu::Instance, surface: &wgpu::Surface) -> Result<(wgpu::Device, wgpu::Queue), GetDeviceError> {
	// Find a suitable display adapter.
	let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
		power_preference: select_power_preference(),
		compatible_surface: Some(surface),
		force_fallback_adapter: false,
	});

	let adapter = adapter.await.ok_or(NoSuitableAdapterFound)?;

	// Create the logical device and command queue
	let device = adapter.request_device(
		&wgpu::DeviceDescriptor {
			label: Some("show-image"),
			limits: wgpu::Limits::default(),
			features: wgpu::Features::default(),
		},
		None,
	);

	let (device, queue) = device.await?;

	Ok((device, queue))
}

/// Create the bind group layout for the window specific bindings.
fn create_window_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("window_bind_group_layout"),
		entries: &[wgpu::BindGroupLayoutEntry {
			binding: 0,
			visibility: wgpu::ShaderStages::VERTEX,
			count: None,
			ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: Some(NonZeroU64::new(WindowUniforms::STD140_SIZE).unwrap()),
			},
		}],
	})
}

/// Create the bind group layout for the image specific bindings.
fn create_image_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("image_bind_group_layout"),
		entries: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::FRAGMENT,
				count: None,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: Some(NonZeroU64::new(std::mem::size_of::<super::util::GpuImageUniforms>() as u64).unwrap()),
				},
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				count: None,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Storage {
						read_only: true,
					},
					has_dynamic_offset: false,
					min_binding_size: None,
				},
			},
		],
	})
}

/// Create a render pipeline with the specified device, layout, shaders and swap chain format.
fn create_render_pipeline(
	device: &wgpu::Device,
	layout: &wgpu::PipelineLayout,
	vertex_shader: &wgpu::ShaderModule,
	fragment_shader: &wgpu::ShaderModule,
	swap_chain_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("show-image-pipeline"),
		layout: Some(layout),
		vertex: wgpu::VertexState {
			module: vertex_shader,
			entry_point: "main",
			buffers: &[],
		},
		fragment: Some(wgpu::FragmentState {
			module: fragment_shader,
			entry_point: "main",
			targets: &[wgpu::ColorTargetState {
				format: swap_chain_format,
				blend: Some(wgpu::BlendState {
					color: wgpu::BlendComponent {
						src_factor: wgpu::BlendFactor::SrcAlpha,
						dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
						operation: wgpu::BlendOperation::Add,
					},
					alpha: wgpu::BlendComponent {
						src_factor: wgpu::BlendFactor::One,
						dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
						operation: wgpu::BlendOperation::Add,
					},
				}),
				write_mask: wgpu::ColorWrites::ALL,
			}],
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleList,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Cw,
			cull_mode: Some(wgpu::Face::Back),
			unclipped_depth: false,
			polygon_mode: wgpu::PolygonMode::Fill,
			conservative: false,
		},
		depth_stencil: None,
		multisample: wgpu::MultisampleState {
			count: 1,
			mask: !0,
			alpha_to_coverage_enabled: false,
		},
		multiview: None,
	})
}

/// Create a swap chain for a surface.
fn configure_surface(
	size: glam::UVec2,
	surface: &wgpu::Surface,
	format: wgpu::TextureFormat,
	device: &wgpu::Device,
) {
	let config = wgpu::SurfaceConfiguration {
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
		format,
		width: size.x,
		height: size.y,
		present_mode: wgpu::PresentMode::Mailbox,
	};
	surface.configure(device, &config);
}

/// Perform a render pass of an image.
fn render_pass(
	encoder: &mut wgpu::CommandEncoder,
	render_pipeline: &wgpu::RenderPipeline,
	window_uniforms: &UniformsBuffer<WindowUniforms>,
	image: &GpuImage,
	clear: Option<crate::Color>,
	target: &wgpu::TextureView,
) {
	let load = match clear {
		Some(color) => wgpu::LoadOp::Clear(color.into()),
		None => wgpu::LoadOp::Load,
	};

	let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		label: Some("render-image"),
		color_attachments: &[wgpu::RenderPassColorAttachment {
			view: target,
			resolve_target: None,
			ops: wgpu::Operations { load, store: true },
		}],
		depth_stencil_attachment: None,
	});

	render_pass.set_pipeline(render_pipeline);
	render_pass.set_bind_group(0, window_uniforms.bind_group(), &[]);
	render_pass.set_bind_group(1, image.bind_group(), &[]);
	render_pass.draw(0..6, 0..1);
	drop(render_pass);
}

#[cfg(feature = "save")]
fn align_next_u32(input: u32, alignment: u32) -> u32 {
	let remainder = input % alignment;
	if remainder == 0 {
		input
	} else {
		input - remainder + alignment
	}
}

#[cfg(feature = "save")]
fn div_round_up(input: u32, divisor: u32) -> u32 {
	if input % divisor == 0 {
		input / divisor
	} else {
		input / divisor + 1
	}
}
