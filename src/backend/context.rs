use crate::ContextProxy;
use crate::EventHandlerOutput;
use crate::Image;
use crate::Window;
use crate::WindowHandle;
use crate::WindowId;
use crate::WindowOptions;
use crate::backend::event::map_nonuser_event;
use crate::backend::proxy::ContextFunction;
use crate::backend::util::RetainMut;
use crate::backend::util::GpuImage;
use crate::backend::util::UniformsBuffer;
use crate::backend::window::WindowUniforms;
use crate::error::GetDeviceError;
use crate::error::InvalidWindowIdError;
use crate::error::NoSuitableAdapterFoundError;
use crate::error::OsError;
use crate::event::Event;
use crate::event::WindowEvent;

/// Shorthand type-alias for the correct [`winit::event_loop::EventLoop`].
type EventLoop = winit::event_loop::EventLoop<ContextFunction>;

/// Shorthand type-alias for the correct [`winit::event_loop::EventLoopWindowTarget`].
type EventLoopWindowTarget = winit::event_loop::EventLoopWindowTarget<ContextFunction>;

/// The global context managing all windows and the main event loop.
pub struct Context {
	/// The wgpu instance to create surfaces with.
	instance: wgpu::Instance,

	/// The event loop to use.
	///
	/// Running the event loop consumes it,
	/// so from that point on this field is `None`.
	event_loop: Option<EventLoop>,

	/// A proxy object to clone for new requests.
	proxy: ContextProxy,

	/// The wgpu device to use.
	device: wgpu::Device,

	/// The wgpu command queue to use.
	queue: wgpu::Queue,

	/// The swap chain format to use.
	swap_chain_format: wgpu::TextureFormat,

	/// The bind group layout for the window specific bindings.
	window_bind_group_layout: wgpu::BindGroupLayout,

	/// The bind group layout for the image specific bindings.
	image_bind_group_layout: wgpu::BindGroupLayout,

	/// The render pipeline to use.
	render_pipeline: wgpu::RenderPipeline,

	/// The windows.
	windows: Vec<Window>,

	/// The global event handlers.
	event_handlers: Vec<Box<dyn FnMut(ContextHandle, &mut crate::Event) -> EventHandlerOutput + 'static>>,
}

/// A handle to the global context.
///
/// You can interact with the global context through a [`ContextHandle`] only from the context thread.
/// To interact with the context from a different thread, use a [`ContextProxy`].
pub struct ContextHandle<'a> {
	context: &'a mut Context,
	event_loop: &'a EventLoopWindowTarget,
}

impl Context {
	/// Create a new global context.
	///
	/// You can theoreticlly create as many as you want,
	/// but they must be run from the main thread and the [`run`](Self::run) function never returns.
	/// So you can only *run* a single context.
	pub fn new(swap_chain_format: wgpu::TextureFormat) -> Result<Self, GetDeviceError> {
		let instance = wgpu::Instance::new(wgpu::BackendBit::all());
		let event_loop = EventLoop::with_user_event();
		let proxy = ContextProxy::new(event_loop.create_proxy());

		let (device, queue) = futures::executor::block_on(get_device(&instance))?;

		let window_bind_group_layout = create_window_bind_group_layout(&device);
		let image_bind_group_layout = create_image_bind_group_layout(&device);

		let vertex_shader = device.create_shader_module(wgpu::include_spirv!("../../shaders/shader.vert.spv"));
		let fragment_shader = device.create_shader_module(wgpu::include_spirv!("../../shaders/shader.frag.spv"));

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("show-image-pipeline-layout"),
			bind_group_layouts: &[&window_bind_group_layout, &image_bind_group_layout],
			push_constant_ranges: &[],
		});

		let render_pipeline = create_render_pipeline(&device, &pipeline_layout, &vertex_shader, &fragment_shader, swap_chain_format);

		Ok(Self {
			instance,
			event_loop: Some(event_loop),
			proxy,
			device,
			queue,
			swap_chain_format,
			window_bind_group_layout,
			image_bind_group_layout,
			render_pipeline,
			windows: Vec::new(),
			event_handlers: Vec::new(),
		})
	}

	/// Get a proxy for the context to interact with it from a different thread.
	pub fn proxy(&self) -> ContextProxy {
		self.proxy.clone()
	}

	/// Add a global event handler.
	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: 'static + FnMut(ContextHandle, &mut crate::Event) -> EventHandlerOutput,
	{
		self.add_boxed_event_handler(Box::new(handler))
	}

	/// Add a boxed global event handler.
	///
	/// This does the same as [`Self::add_event_handler`],
	/// but doesn't add another layer of boxing if you already have a boxed function.
	pub fn add_boxed_event_handler(
		&mut self,
		handler: Box<dyn FnMut(ContextHandle, &mut crate::Event) -> EventHandlerOutput>
	) {
		self.event_handlers.push(handler)
	}

	/// Add a window-specific event handler.
	pub fn add_window_event_handler<F>(&mut self, window_id: WindowId, handler: F) -> Result<(), InvalidWindowIdError>
	where
		F: 'static + FnMut(WindowHandle, &mut WindowEvent) -> EventHandlerOutput,
	{
		let window = self.windows.iter_mut()
			.find(|x| x.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		window.event_handlers.push(Box::new(handler));
		Ok(())
	}

	/// Add a boxed window-specific event handler.
	///
	/// This does the same as [`Self::add_window_event_handler`],
	/// but doesn't add another layer of boxing if you already have a boxed function.
	pub fn add_boxed_window_event_handler(
		&mut self,
		window_id: WindowId,
		handler: Box<dyn FnMut(WindowHandle, &mut WindowEvent) -> EventHandlerOutput>,
	) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|x| x.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		window.event_handlers.push(handler);
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
			self.handle_event(event, event_loop, control_flow)
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
	pub fn proxy(&self) -> ContextProxy {
		self.context.proxy()
	}

	/// Create a new window.
	pub fn create_window(&mut self, title: impl Into<String>, options: WindowOptions) -> Result<WindowHandle, OsError> {
		let window_id = self.context.create_window(self.event_loop, title, options)?;
		Ok(WindowHandle::new(ContextHandle {
			context: self.context,
			event_loop: self.event_loop,
		}, window_id))
	}

	/// Destroy a window.
	pub fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		self.context.destroy_window(window_id)
	}

	/// Make a window visible or invisible.
	pub fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowIdError> {
		self.context.set_window_visible(window_id, visible)
	}

	/// Set the image to be displayed on a window.
	pub fn set_window_image(&mut self, window_id: WindowId, name: &str, image: &Image) -> Result<(), InvalidWindowIdError> {
		self.context.set_window_image(window_id, name, image)
	}

	/// Add a global event handler.
	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: 'static + FnMut(ContextHandle, &mut crate::Event) -> EventHandlerOutput,
	{
		self.context.add_event_handler(handler);
	}

	/// Add a boxed global event handler.
	///
	/// This does the same as [`Self::add_event_handler`],
	/// but doesn't add another layer of boxing if you already have a boxed function.
	pub fn add_boxed_event_handler(&mut self, handler: Box<dyn FnMut(ContextHandle, &mut crate::Event) -> EventHandlerOutput + 'static>) {
		self.context.add_boxed_event_handler(handler);
	}

	/// Add a window-specific event handler.
	pub fn add_window_event_handler<F>(&mut self, window_id: WindowId, handler: F) -> Result<(), InvalidWindowIdError>
	where
		F: 'static + FnMut(WindowHandle, &mut WindowEvent) -> EventHandlerOutput,
	{
		self.context.add_window_event_handler(window_id, handler)
	}

	/// Add a boxed window-specific event handler.
	///
	/// This does the same as [`Self::add_window_event_handler`],
	/// but doesn't add another layer of boxing if you already have a boxed function.
	pub fn add_boxed_window_event_handler(
		&mut self,
		window_id: WindowId,
		handler: Box<dyn FnMut(WindowHandle, &mut WindowEvent) -> EventHandlerOutput>,
	) -> Result<(), InvalidWindowIdError> {
		self.context.add_boxed_window_event_handler(window_id, handler)
	}
}

impl Context {
	/// Create a window.
	fn create_window(
		&mut self,
		event_loop: &EventLoopWindowTarget,
		title: impl Into<String>,
		options: WindowOptions,
	) -> Result<WindowId, OsError> {
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
		let uniforms = UniformsBuffer::from_value(&self.device, &WindowUniforms::default(), &self.window_bind_group_layout);

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
	fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		let index = self.windows.iter().position(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;
		self.windows.remove(index);
		Ok(())
	}

	/// Make a window visible or invisible.
	fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;
		window.set_visible(visible);
		Ok(())
	}

	/// Set the image to be displayed on a window.
	fn set_window_image(&mut self, window_id: WindowId, name: &str, image: &Image) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		let texture = GpuImage::from_data(&self.device, &self.image_bind_group_layout, name, image);
		window.image = Some(texture);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	/// Resize a window.
	fn resize_window(&mut self, window_id: WindowId, new_size: winit::dpi::PhysicalSize<u32>) -> Result<(), InvalidWindowIdError> {
		let window = self.windows
			.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		window.swap_chain = create_swap_chain(new_size, &window.surface, self.swap_chain_format, &self.device);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	/// Render the contents of a window.
	fn render_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		let image = match &window.image {
			Some(x) => x,
			None => return Ok(()),
		};

		let frame = window.swap_chain
			.get_current_frame()
			.expect("Failed to acquire next swap chain texture");

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

		if window.uniforms.is_dirty() {
			window.uniforms.update_from(&self.device, &mut encoder, &window.calculate_uniforms());
		}

		let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(window.options.background_color),
					store: true,
				},
				attachment: &frame.output.view,
				resolve_target: None,
			}],
			depth_stencil_attachment: None,
		});

		render_pass.set_pipeline(&self.render_pipeline);
		render_pass.set_bind_group(0, window.uniforms.bind_group(), &[]);
		render_pass.set_bind_group(1, image.bind_group(), &[]);
		render_pass.draw(0..6, 0..1);
		drop(render_pass);

		self.queue.submit(std::iter::once(encoder.finish()));
		Ok(())
	}

	/// Handle an event from the event loop.
	fn handle_event(
		&mut self,
		event: Event<ContextFunction>,
		event_loop: &EventLoopWindowTarget,
		control_flow: &mut winit::event_loop::ControlFlow,
	) {
		*control_flow = winit::event_loop::ControlFlow::Poll;

		// Split between Event<ContextFunction> and ContextFunction commands.
		let mut event = match map_nonuser_event(event) {
			Ok(event) => event,
			Err(function) => {
				(function)(&mut ContextHandle::new(self, event_loop));
				return;
			},
		};

		let run_context_handlers = match &mut event {
			Event::WindowEvent { window_id, event } => self.run_window_event_handlers(*window_id, event, event_loop),
			_ => true,
		};

		if run_context_handlers {
			self.run_event_handlers(&mut event, event_loop);
		}

		match event {
			Event::WindowEvent { window_id, event: WindowEvent::Resized(new_size) } => {
				let _  = self.resize_window(window_id, new_size);
			}
			Event::RedrawRequested(window_id) => {
				let _ = self.render_window(window_id);
			}
			Event::WindowEvent { window_id, event: WindowEvent::CloseRequested } => {
				let _ = self.destroy_window(window_id);
			},
			_ => {},
		}
	}

	/// Run global event handlers.
	fn run_event_handlers(&mut self, event: &mut crate::Event, event_loop: &EventLoopWindowTarget) {
		// Event handlers could potentially modify the list of event handlers.
		// Also, even if they couldn't we'd still need borrow self mutably multible times to run the event handlers.
		// That's not allowed, of course, so temporarily swap the event handlers with a new vector.
		// When we've run all handlers, we add the new handlers to the original vector and place it back.
		// https://newfastuff.com/wp-content/uploads/2019/05/dVIkgAf.png
		let mut event_handlers = std::mem::replace(&mut self.event_handlers, Vec::new());

		let mut stop_processing = false;
		event_handlers.retain_mut(|handler| {
			if stop_processing {
				false
			} else {
				let context_handle = ContextHandle::new(self, event_loop);
				let result = (handler)(context_handle, event);
				stop_processing = result.stop_propagation;
				!result.remove_handler
			}
		});

		let new_event_handlers = std::mem::replace(&mut self.event_handlers, Vec::new());
		event_handlers.extend(new_event_handlers);
		self.event_handlers = event_handlers;
	}

	/// Run window-specific event handlers.
	fn run_window_event_handlers(&mut self, window_id: WindowId, event: &mut WindowEvent, event_loop: &EventLoopWindowTarget) -> bool {
		let window_index = match self.windows.iter().position(|x| x.id() == window_id) {
			Some(x) => x,
			None => return true,
		};

		let mut event_handlers = std::mem::replace(&mut self.windows[window_index].event_handlers, Vec::new());

		let mut stop_processing = false;
		event_handlers.retain_mut(|handler| {
			if stop_processing {
				false
			} else {
				let context_handle = ContextHandle::new(self, event_loop);
				let window_handle = WindowHandle::new(context_handle, window_id);
				let result = (handler)(window_handle, event);
				stop_processing = result.stop_propagation;
				!result.remove_handler
			}
		});

		let new_event_handlers = std::mem::replace(&mut self.windows[window_index].event_handlers, Vec::new());
		event_handlers.extend(new_event_handlers);
		self.windows[window_index].event_handlers = event_handlers;

		return !stop_processing;
	}
}

/// Get a wgpu device to use.
async fn get_device(instance: &wgpu::Instance) -> Result<(wgpu::Device, wgpu::Queue), GetDeviceError> {
	// Find a suitable display adapter.
	let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
		power_preference: wgpu::PowerPreference::Default,
		compatible_surface: None, // TODO: can we use a hidden window or something?
	}).await;

	let adapter = adapter.ok_or(NoSuitableAdapterFoundError)?;

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
					min_binding_size: Some(std::num::NonZeroU64::new(std::mem::size_of::<super::window::WindowUniforms>() as u64).unwrap()),
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
			color_blend: wgpu::BlendDescriptor::REPLACE,
			alpha_blend: wgpu::BlendDescriptor::REPLACE,
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
