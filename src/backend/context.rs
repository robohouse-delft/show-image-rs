use crate::AsImageView;
use crate::ContextProxy;
use crate::WindowHandle;
use crate::WindowId;
use crate::WindowOptions;
use crate::backend::proxy::ContextFunction;
use crate::backend::util::UniformsBuffer;
use crate::backend::window::Window;
use crate::backend::window::WindowUniforms;
use crate::background_thread::BackgroundThread;
use crate::error::CreateWindowError;
use crate::error::GetDeviceError;
use crate::error::InvalidWindowId;
use crate::error::NoSuitableAdapterFound;
use crate::error::SetImageError;
use crate::event::Event;
use crate::event::EventHandlerControlFlow;
use crate::event::WindowEvent;
use crate::event;

/// Internal shorthand type-alias for the correct [`winit::event_loop::EventLoop`].
///
/// Not for use in public APIs.
type EventLoop = winit::event_loop::EventLoop<ContextFunction>;

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

/// The global context managing all windows and the main event loop.
pub struct Context {
	/// Marker to make context !Send.
	pub unsend: std::marker::PhantomData<*const ()>,

	/// The wgpu instance to create surfaces with.
	pub instance: wgpu::Instance,

	/// The event loop to use.
	///
	/// Running the event loop consumes it,
	/// so from that point on this field is `None`.
	pub event_loop: Option<EventLoop>,

	/// A proxy object to clone for new requests.
	pub proxy: ContextProxy,

	/// The wgpu device to use.
	pub device: wgpu::Device,

	/// The wgpu command queue to use.
	pub queue: wgpu::Queue,

	/// The swap chain format to use.
	pub swap_chain_format: wgpu::TextureFormat,

	/// The bind group layout for the window specific bindings.
	pub window_bind_group_layout: wgpu::BindGroupLayout,

	/// The bind group layout for the image specific bindings.
	pub image_bind_group_layout: wgpu::BindGroupLayout,

	/// The render pipeline to use for windows.
	pub window_pipeline: wgpu::RenderPipeline,

	/// The render pipeline to use for rendering to image.
	pub image_pipeline: wgpu::RenderPipeline,

	/// The windows.
	pub windows: Vec<Window>,

	/// If true, exit the program when the last window closes.
	pub exit_with_last_window: bool,

	/// The global event handlers.
	pub event_handlers: Vec<Box<dyn FnMut(&mut ContextHandle, &mut Event, &mut event::EventHandlerControlFlow) + 'static>>,

	/// Background tasks, like saving images.
	pub background_tasks: Vec<BackgroundThread<()>>,
}

/// Handle to the global context.
///
/// You can interact with the global context through a [`ContextHandle`] only from the global context thread.
/// To interact with the context from a different thread, use a [`ContextProxy`].
pub struct ContextHandle<'a> {
	context: &'a mut Context,
	event_loop: &'a EventLoopWindowTarget,
}

impl Context {
	/// Create a new global context.
	///
	/// You can theoreticlly create as many contexts as you want,
	/// but they must be run from the main thread and the [`run`](Self::run) function never returns.
	/// So it is not possible to *run* more than one context.
	pub fn new(swap_chain_format: wgpu::TextureFormat) -> Result<Self, GetDeviceError> {
		let instance = wgpu::Instance::new(wgpu::BackendBit::all());
		let event_loop = EventLoop::with_user_event();
		let proxy = ContextProxy::new(event_loop.create_proxy(), std::thread::current().id());

		let (device, queue) = futures::executor::block_on(get_device(&instance))?;

		let window_bind_group_layout = create_window_bind_group_layout(&device);
		let image_bind_group_layout = create_image_bind_group_layout(&device);

		let vertex_shader = device.create_shader_module(wgpu::include_spirv!("../../shaders/shader.vert.spv"));
		let fragment_shader_unorm8 = device.create_shader_module(wgpu::include_spirv!("../../shaders/unorm8.frag.spv"));
		let fragment_shader_uint8 = device.create_shader_module(wgpu::include_spirv!("../../shaders/uint8.frag.spv"));

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("show-image-pipeline-layout"),
			bind_group_layouts: &[&window_bind_group_layout, &image_bind_group_layout],
			push_constant_ranges: &[],
		});

		let window_pipeline = create_render_pipeline(&device, &pipeline_layout, &vertex_shader, &fragment_shader_unorm8, swap_chain_format);
		let image_pipeline = create_render_pipeline(&device, &pipeline_layout, &vertex_shader, &fragment_shader_uint8, wgpu::TextureFormat::Rgba8Uint);

		Ok(Self {
			unsend: Default::default(),
			instance,
			event_loop: Some(event_loop),
			proxy,
			device,
			queue,
			swap_chain_format,
			window_bind_group_layout,
			image_bind_group_layout,
			window_pipeline,
			image_pipeline,
			windows: Vec::new(),
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

	/// Add a window-specific event handler.
	pub fn add_window_event_handler<F>(&mut self, window_id: WindowId, handler: F) -> Result<(), InvalidWindowId>
	where
		F: 'static + FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow),
	{
		let window = self.windows.iter_mut()
			.find(|x| x.id() == window_id)
			.ok_or_else(|| InvalidWindowId { window_id })?;

		window.event_handlers.push(Box::new(handler));
		Ok(())
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
	fn new(
		context: &'a mut Context,
		event_loop: &'a EventLoopWindowTarget,
	) -> Self {
		Self { context, event_loop }
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

	/// Create a new window.
	pub fn create_window(&mut self, title: impl Into<String>, options: WindowOptions) -> Result<WindowHandle, CreateWindowError> {
		let window_id = self.context.create_window(self.event_loop, title, options)?;
		Ok(WindowHandle::new(ContextHandle {
			context: self.context,
			event_loop: self.event_loop,
		}, window_id))
	}

	/// Destroy a window.
	pub fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowId> {
		self.context.destroy_window(window_id)
	}

	/// Make a window visible or invisible.
	pub fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowId> {
		self.context.set_window_visible(window_id, visible)
	}

	/// Set the image to be displayed on a window.
	pub fn set_window_image(&mut self, window_id: WindowId, name: impl Into<String>, image: &impl AsImageView) -> Result<(), SetImageError> {
		self.context.set_window_image(window_id, name.into(), image)
	}

	/// Add a global event handler.
	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: 'static + FnMut(&mut ContextHandle, &mut Event, &mut EventHandlerControlFlow),
	{
		self.context.add_event_handler(handler);
	}

	/// Add a window-specific event handler.
	pub fn add_window_event_handler<F>(&mut self, window_id: WindowId, handler: F) -> Result<(), InvalidWindowId>
	where
		F: 'static + FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow),
	{
		self.context.add_window_event_handler(window_id, handler)
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
	) -> Result<WindowId, CreateWindowError> {
		let mut window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_visible(!options.start_hidden)
			.with_resizable(options.resizable);

		if let Some(size) = options.size {
			let size = winit::dpi::LogicalSize::new(size[0], size[1]);
			window = window.with_inner_size(size);
		}

		let window = window.build(event_loop)?;

		let surface = unsafe { self.instance.create_surface(&window) };
		let swap_chain = create_swap_chain(window.inner_size(), &surface, self.swap_chain_format, &self.device);
		let uniforms = UniformsBuffer::from_value(&self.device, &Default::default(), &self.window_bind_group_layout);

		let window = Window {
			window,
			options,
			surface,
			swap_chain,
			uniforms,
			image: None,
			event_handlers: Vec::new(),
		};

		let window_id = window.id();
		self.windows.push(window);
		Ok(window_id)
	}

	/// Destroy a window.
	fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowId> {
		let index = self.windows.iter().position(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowId { window_id })?;
		self.windows.remove(index);
		Ok(())
	}

	/// Make a window visible or invisible.
	fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowId> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowId { window_id })?;
		window.set_visible(visible);
		Ok(())
	}

	/// Set the image to be displayed on a window.
	fn set_window_image(&mut self, window_id: WindowId, name: String, image: &impl AsImageView) -> Result<(), SetImageError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowId { window_id })?;

		let image = image.as_image_view()?;
		let texture = super::util::GpuImage::from_data(name, &self.device, &self.image_bind_group_layout, image);
		window.image = Some(texture);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	/// Resize a window.
	fn resize_window(&mut self, window_id: WindowId, new_size: winit::dpi::PhysicalSize<u32>) -> Result<(), InvalidWindowId> {
		let window = self.windows
			.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowId { window_id })?;

		window.swap_chain = create_swap_chain(new_size, &window.surface, self.swap_chain_format, &self.device);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	/// Render the contents of a window.
	fn render_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowId> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowId { window_id })?;

		let image = match &window.image {
			Some(x) => x,
			None => return Ok(()),
		};

		let frame = window.swap_chain
			.get_current_frame()
			.expect("Failed to acquire next swap chain texture");

		let mut encoder = self.device.create_command_encoder(&Default::default());

		if window.uniforms.is_dirty() {
			window.uniforms.update_from(&self.device, &mut encoder, &window.calculate_uniforms());
		}

		render_pass(&mut encoder, &self.window_pipeline, &window.uniforms, image, window.options.background_color, &frame.output.view);
		self.queue.submit(std::iter::once(encoder.finish()));
		Ok(())
	}

	fn render_to_texture(&self, window_id: WindowId) -> Result<Option<(String, crate::BoxImage)>, InvalidWindowId> {
		let window = self.windows.iter()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowId { window_id })?;

		let image = match &window.image {
			Some(x) => x,
			None => return Ok(None),
		};

		let bytes_per_row = align_next_u32(image.width() * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

		let size = wgpu::Extent3d {
			width: div_round_up(bytes_per_row, 4),
			height: image.height(),
			depth: 1,
		};

		let window_uniforms = WindowUniforms {
			offset: [0.0, 0.0],
			size: [image.width() as f32 / size.width as f32, 1.0],
		};
		let window_uniforms = UniformsBuffer::from_value(&self.device, &window_uniforms, &self.window_bind_group_layout);

		let target = self.device.create_texture(&wgpu::TextureDescriptor {
			label: Some(&format!("{}_render", image.name())),
			usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
			sample_count: 1,
			mip_level_count: 1,
			format: wgpu::TextureFormat::Rgba8Uint,
			dimension: wgpu::TextureDimension::D2,
			size,
		});

		let mut encoder = self.device.create_command_encoder(&Default::default());
		let transparent = crate::Color::rgba(0.0, 0.0, 0.0, 0.0);
		render_pass(
			&mut encoder,
			&self.image_pipeline,
			&window_uniforms,
			image,
			transparent,
			&target.create_view(&wgpu::TextureViewDescriptor {
				label: None,
				format: None,
				dimension: None,
				aspect: wgpu::TextureAspect::All,
				base_mip_level: 0,
				level_count: None,
				base_array_layer: 0,
				array_layer_count: None,
			}),
		);

		let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			size: u64::from(bytes_per_row) * u64::from(image.height()),
			usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
			mapped_at_creation: false,
		});

		encoder.copy_texture_to_buffer(
			wgpu::TextureCopyView {
				texture: &target,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
			},
			wgpu::BufferCopyView {
				buffer: &buffer,
				layout: wgpu::TextureDataLayout {
					bytes_per_row,
					rows_per_image: image.height(),
					offset: 0,
				},
			},
			size,
		);

		self.queue.submit(std::iter::once(encoder.finish()));

		let view = super::util::map_buffer(&self.device, buffer.slice(..)).unwrap();
		let info = crate::ImageInfo {
			pixel_format: crate::PixelFormat::Rgba8(crate::Alpha::Unpremultiplied),
			width: image.width(),
			height: image.height(),
			stride_x: 4,
			stride_y: bytes_per_row,
		};
		let data: Box<[u8]> = Box::from(&*view);
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

		// Convert to own event type.
		let mut event = match super::event::convert_winit_event(event) {
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
					if event.input.modifiers == event::ModifiersState::CTRL {
						self.save_image_prompt(event.window_id);
					} else if event.input.modifiers == event::ModifiersState::CTRL | event::ModifiersState::SHIFT {
						self.save_image(event.window_id);
					}
				}
			},
			Event::WindowEvent(WindowEvent::Resized(event)) => {
				let _  = self.resize_window(event.window_id, event.size);
			}
			Event::WindowEvent(WindowEvent::RedrawRequested(event)) => {
				let _ = self.render_window(event.window_id);
			}
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
		// Also, even if they couldn't we'd still need borrow self mutably multible times to run the event handlers.
		// That's not allowed, of course, so temporarily swap the event handlers with a new vector.
		// When we've run all handlers, we add the new handlers to the original vector and place it back.
		// https://newfastuff.com/wp-content/uploads/2019/05/dVIkgAf.png
		let mut event_handlers = std::mem::replace(&mut self.event_handlers, Vec::new());

		let mut stop_propagation = false;
		event_handlers.retain_mut(|handler| {
			if stop_propagation {
				false
			} else {
				let mut context_handle = ContextHandle::new(self, event_loop);
				let mut control = EventHandlerControlFlow::default();
				(handler)(&mut context_handle, event, &mut control);
				stop_propagation = control.stop_propagation;
				!control.remove_handler
			}
		});

		let new_event_handlers = std::mem::replace(&mut self.event_handlers, Vec::new());
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

		let mut event_handlers = std::mem::replace(&mut self.windows[window_index].event_handlers, Vec::new());

		let mut stop_propagation = false;
		event_handlers.retain_mut(|handler| {
			if stop_propagation {
				false
			} else {
				let context_handle = ContextHandle::new(self, event_loop);
				let mut window_handle = WindowHandle::new(context_handle, event.window_id());
				let mut control = EventHandlerControlFlow::default();
				(handler)(&mut window_handle, event, &mut control);
				stop_propagation = control.stop_propagation;
				!control.remove_handler
			}
		});

		let new_event_handlers = std::mem::replace(&mut self.windows[window_index].event_handlers, Vec::new());
		event_handlers.extend(new_event_handlers);
		self.windows[window_index].event_handlers = event_handlers;

		return !stop_propagation;
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
		for task in std::mem::replace(&mut self.background_tasks, Vec::new()) {
			task.join().unwrap();
		}
	}

	/// Join all background tasks and then exit the process.
	fn exit(&mut self, code: i32) -> ! {
		self.join_background_tasks();
		std::process::exit(code);
	}

	#[cfg(feature = "save")]
	fn save_image_prompt(&mut self, window_id: WindowId) {
		let (name, image) = match self.render_to_texture(window_id) {
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
			if let Err(e) = crate::save_rgba8_image(&path, image.data(), info.width, info.height, info.stride_y) {
				log::error!("failed to save image to {}: {}", path, e);
			}
		});
	}

	#[cfg(feature = "save")]
	fn save_image(&mut self, window_id: WindowId) {
		let (name, image) = match self.render_to_texture(window_id) {
			Ok(Some(x)) => x,
			Ok(None) => return,
			Err(e) => return log::error!("failed to render window contents: {}", e),
		};

		let info = image.info();
		let name = format!("{}.png", name);
		self.run_background_task(move || {
			if let Err(e) = crate::save_rgba8_image(&name, image.data(), info.width, info.height, info.stride_y) {
				log::error!("failed to save image to {}: {}", name, e);
			}
		});
	}
}

/// Get a wgpu device to use.
async fn get_device(instance: &wgpu::Instance) -> Result<(wgpu::Device, wgpu::Queue), GetDeviceError> {
	// Find a suitable display adapter.
	let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
		power_preference: wgpu::PowerPreference::Default,
		compatible_surface: None, // TODO: can we use a hidden window or something?
	}).await;

	let adapter = adapter.ok_or(NoSuitableAdapterFound)?;

	// Create the logical device and command queue
	let (device, queue) = adapter.request_device(
		&wgpu::DeviceDescriptor {
			limits: wgpu::Limits::default(),
			features: wgpu::Features::default(),
			shader_validation: true,
		},
		None,
	).await?;

	Ok((device, queue))
}

/// Create the bind group layout for the window specific bindings.
fn create_window_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("window_bind_group_layout"),
		entries: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStage::VERTEX,
				count: None,
				ty: wgpu::BindingType::UniformBuffer {
					dynamic: false,
					min_binding_size: Some(std::num::NonZeroU64::new(std::mem::size_of::<WindowUniforms>() as u64).unwrap()),
				},
			},
		],
	})
}

/// Create the bind group layout for the image specific bindings.
fn create_image_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("image_bind_group_layout"),
		entries: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStage::FRAGMENT,
				count: None,
				ty: wgpu::BindingType::UniformBuffer {
					dynamic: false,
					min_binding_size: Some(std::num::NonZeroU64::new(std::mem::size_of::<super::util::GpuImageUniforms>() as u64).unwrap()),
				},
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStage::FRAGMENT,
				count: None,
				ty: wgpu::BindingType::StorageBuffer {
					readonly: true,
					dynamic: false,
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
		layout: Some(&layout),
		vertex_stage: wgpu::ProgrammableStageDescriptor {
			module: &vertex_shader,
			entry_point: "main",
		},
		fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
			module: &fragment_shader,
			entry_point: "main",
		}),

		rasterization_state: None,
		primitive_topology: wgpu::PrimitiveTopology::TriangleList,
		color_states: &[wgpu::ColorStateDescriptor {
			format: swap_chain_format,
			color_blend: wgpu::BlendDescriptor {
				src_factor: wgpu::BlendFactor::SrcAlpha,
				dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
				operation: wgpu::BlendOperation::Add,
			},
			alpha_blend: wgpu::BlendDescriptor {
				src_factor: wgpu::BlendFactor::One,
				dst_factor: wgpu::BlendFactor::Zero,
				operation: wgpu::BlendOperation::Add,
			},
			write_mask: wgpu::ColorWrite::ALL,
		}],
		depth_stencil_state: None,
		vertex_state: wgpu::VertexStateDescriptor {
			index_format: wgpu::IndexFormat::Uint16,
			vertex_buffers: &[],
		},
		sample_count: 1,
		sample_mask: !0,
		alpha_to_coverage_enabled: false,
	})
}

/// Create a swap chain for a surface.
fn create_swap_chain(size: winit::dpi::PhysicalSize<u32>, surface: &wgpu::Surface, format: wgpu::TextureFormat, device: &wgpu::Device) -> wgpu::SwapChain {
	let swap_chain_desc = wgpu::SwapChainDescriptor {
		usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
		format,
		width: size.width,
		height: size.height,
		present_mode: wgpu::PresentMode::Mailbox,
	};

	device.create_swap_chain(&surface, &swap_chain_desc)
}

/// Perform a render pass of an image.
fn render_pass(
	encoder: &mut wgpu::CommandEncoder,
	render_pipeline: &wgpu::RenderPipeline,
	window_uniforms: &UniformsBuffer<WindowUniforms>,
	image: &super::util::GpuImage,
	background_color: crate::Color,
	target: &wgpu::TextureView,
) {
	let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
			ops: wgpu::Operations {
				load: wgpu::LoadOp::Clear(background_color.into()),
				store: true,
			},
			attachment: &target,
			resolve_target: None,
		}],
		depth_stencil_attachment: None,
	});

	render_pass.set_pipeline(render_pipeline);
	render_pass.set_bind_group(0, window_uniforms.bind_group(), &[]);
	render_pass.set_bind_group(1, image.bind_group(), &[]);
	render_pass.draw(0..6, 0..1);
	drop(render_pass);
}

fn align_next_u32(input: u32, alignment: u32) -> u32 {
	let remainder = input % alignment;
	if remainder == 0 {
		input
	} else {
		input - remainder + alignment
	}
}

fn div_round_up(input: u32, divisor: u32) -> u32 {
	if input % divisor == 0 {
		input / divisor
	} else {
		input / divisor + 1
	}
}
