use crate::ContextProxy;
use crate::EventHandlerOutput;
use crate::Image;
use crate::Window;
use crate::WindowId;
use crate::WindowOptions;
use crate::backend::event::downgrade_event;
use crate::backend::proxy::ContextCommand;
use crate::backend::proxy::ContextEvent;
use crate::backend::util::RetainMut;
use crate::backend::util::Texture;
use crate::backend::util::UniformsBuffer;
use crate::backend::window::WindowUniforms;
use crate::error::InvalidWindowIdError;
use crate::error::NoSuitableAdapterFoundError;
use crate::error::OsError;
use crate::event::Event;

macro_rules! include_spirv {
	($path:literal) => {{
		let bytes = include_bytes!($path);
		wgpu::read_spirv(std::io::Cursor::new(&bytes[..])).unwrap()
	}};
}

type EventLoop<UserEvent> = winit::event_loop::EventLoop<ContextEvent<UserEvent>>;
type EventLoopWindowTarget<UserEvent> = winit::event_loop::EventLoopWindowTarget<ContextEvent<UserEvent>>;

pub struct Context<UserEvent: 'static> {
	event_loop: Option<EventLoop<UserEvent>>,
	proxy: ContextProxy<UserEvent>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	swap_chain_format: wgpu::TextureFormat,
	window_bind_group_layout: wgpu::BindGroupLayout,
	image_bind_group_layout: wgpu::BindGroupLayout,
	render_pipeline: wgpu::RenderPipeline,

	windows: Vec<Window>,
	event_handlers: Vec<Box<dyn FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + 'static>>,
}

pub struct ContextHandle<'a, UserEvent: 'static> {
	context: &'a mut Context<UserEvent>,
	event_loop: &'a EventLoopWindowTarget<UserEvent>,
}

impl<UserEvent> Context<UserEvent> {
	pub fn new(swap_chain_format: wgpu::TextureFormat) -> Result<Self, NoSuitableAdapterFoundError> {
		let event_loop = EventLoop::with_user_event();
		let proxy = ContextProxy::new(event_loop.create_proxy());

		let (device, queue) = futures::executor::block_on(get_device())?;

		let window_bind_group_layout = create_window_bind_group_layout(&device);
		let image_bind_group_layout = create_image_bind_group_layout(&device);

		let vertex_shader = device.create_shader_module(&include_spirv!("../../shaders/shader.vert.spv"));
		let fragment_shader = device.create_shader_module(&include_spirv!("../../shaders/shader.frag.spv"));

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			bind_group_layouts: &[&window_bind_group_layout, &image_bind_group_layout],
		});

		let render_pipeline = create_render_pipeline(&device, &pipeline_layout, &vertex_shader, &fragment_shader, swap_chain_format);

		Ok(Self {
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

	pub fn proxy(&self) -> ContextProxy<UserEvent> {
		self.proxy.clone()
	}

	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + 'static,
	{
		self.add_boxed_event_handler(Box::new(handler))
	}

	pub fn add_boxed_event_handler(&mut self, handler: Box<dyn FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + 'static>) {
		self.event_handlers.push(handler)
	}

	pub fn run(mut self) -> ! {
		let event_loop = self.event_loop.take().unwrap();
		event_loop.run(move |event, event_loop, control_flow| {
			self.handle_event(event, event_loop, control_flow)
		});
	}
}

impl<'a, UserEvent: 'static> ContextHandle<'a, UserEvent> {
	fn new(
		context: &'a mut Context<UserEvent>,
		event_loop: &'a EventLoopWindowTarget<UserEvent>,
	) -> Self {
		Self { context, event_loop }
	}

	pub fn proxy(&self) -> ContextProxy<UserEvent> {
		self.context.proxy()
	}

	pub fn create_window(&mut self, title: impl Into<String>, options: WindowOptions) -> Result<WindowId, OsError> {
		self.context.create_window(self.event_loop, title, options)
	}

	pub fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		self.context.destroy_window(window_id)
	}

	pub fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowIdError> {
		self.context.set_window_visible(window_id, visible)
	}

	pub fn set_window_image(&mut self, window_id: WindowId, name: &str, image: &Image) -> Result<(), InvalidWindowIdError> {
		self.context.set_window_image(window_id, name, image)
	}

	pub fn add_event_handler<F>(&mut self, handler: F)
	where
		F: FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + 'static,
	{
		self.context.add_event_handler(handler);
	}

	pub fn add_boxed_event_handler(&mut self, handler: Box<dyn FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + 'static>) {
		self.context.add_boxed_event_handler(handler);
	}
}

impl<UserEvent> Context<UserEvent> {
	fn create_window(
		&mut self,
		event_loop: &EventLoopWindowTarget<UserEvent>,
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

		let surface = wgpu::Surface::create(&window);
		let swap_chain = create_swap_chain(window.inner_size(), &surface, self.swap_chain_format, &self.device);
		let uniforms = UniformsBuffer::from_value(&self.device, &WindowUniforms::default(), &self.window_bind_group_layout);

		let window = Window {
			window,
			options,
			surface,
			swap_chain,
			uniforms,
			image: None,
			load_texture: None,
		};

		let window_id = window.id();
		self.windows.push(window);
		Ok(window_id)
	}

	fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		let index = self.windows.iter().position(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;
		self.windows.remove(index);
		Ok(())
	}

	fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;
		window.set_visible(visible);
		Ok(())
	}

	fn set_window_image(&mut self, window_id: WindowId, name: &str, image: &Image) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		let texture = Texture::from_data(&self.device, &self.image_bind_group_layout, name, image);
		window.image = Some(texture);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	fn resize_window(&mut self, window_id: WindowId, new_size: winit::dpi::PhysicalSize<u32>) -> Result<(), InvalidWindowIdError> {
		let window = self.windows
			.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		window.swap_chain = create_swap_chain(new_size, &window.surface, self.swap_chain_format, &self.device);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	fn render_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		let image = match &window.image {
			Some(x) => x,
			None => return Ok(()),
		};

		let frame = window.swap_chain
			.get_next_texture()
			.expect("Failed to acquire next swap chain texture");

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

		if window.uniforms.is_dirty() {
			window.uniforms.update_from(&self.device, &mut encoder, &window.calculate_uniforms());
		}

		let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
				load_op: wgpu::LoadOp::Clear,
				store_op: wgpu::StoreOp::Store,
				clear_color: window.options.background_color,
				attachment: &frame.view,
				resolve_target: None,
			}],
			depth_stencil_attachment: None,
		});

		render_pass.set_pipeline(&self.render_pipeline);
		render_pass.set_bind_group(0, window.uniforms.bind_group(), &[]);
		render_pass.set_bind_group(1, image.bind_group(), &[]);
		render_pass.draw(0..6, 0..1);
		drop(render_pass);

		let render_window = encoder.finish();

		if let Some(load_texture) = window.load_texture.take() {
			self.queue.submit(&[load_texture, render_window]);
		} else {
			self.queue.submit(&[render_window]);
		}

		Ok(())
	}

	fn handle_event(
		&mut self,
		event: Event<ContextEvent<UserEvent>>,
		event_loop: &EventLoopWindowTarget<UserEvent>,
		control_flow: &mut winit::event_loop::ControlFlow,
	) {
		use winit::event::WindowEvent;

		*control_flow = winit::event_loop::ControlFlow::Poll;

		let mut event = match downgrade_event(event) {
			Ok(event) => event,
			Err(command) => return self.handle_command(command, event_loop),
		};

		self.run_event_handlers(&mut event, event_loop);

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

	fn run_event_handlers(&mut self, event: &mut Event<UserEvent>, event_loop: &EventLoopWindowTarget<UserEvent>) {
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
				stop_processing = result.stop_processing;
				!result.remove_handler
			}
		});
		let new_event_handlers = std::mem::replace(&mut self.event_handlers, Vec::new());
		event_handlers.extend(new_event_handlers);
		self.event_handlers = event_handlers;
	}

	fn handle_command(
		&mut self,
		command: ContextCommand<UserEvent>,
		event_loop: &EventLoopWindowTarget<UserEvent>,
	) {
		match command {
			ContextCommand::CreateWindow(command) => {
				let _ = command.result_tx.send(self.create_window(event_loop, command.title, command.options));
			},
			ContextCommand::DestroyWindow(command) => {
				let _ = command.result_tx.send(self.destroy_window(command.window_id));
			},
			ContextCommand::SetWindowVisible(command) => {
				let _ = command.result_tx.send(self.set_window_visible(command.window_id, command.visible));
			}
			ContextCommand::SetWindowImage(command) => {
				let _ = command.result_tx.send(self.set_window_image(command.window_id, &command.name, &command.image));
			}
			ContextCommand::AddContextEventHandler(command) => {
				self.event_handlers.push(command.handler);
			}
			ContextCommand::ExecuteFunction(command) => {
				(command.function)(ContextHandle::new(self, event_loop));
			},
		}
	}
}

async fn get_device() -> Result<(wgpu::Device, wgpu::Queue), NoSuitableAdapterFoundError> {
	// Find a suitable display adapter.
	let adapter = wgpu::Adapter::request(
		&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::Default,
			compatible_surface: None, // TODO: can we use a hidden window or something?
		},
		wgpu::BackendBit::PRIMARY
	).await;

	let adapter = adapter.ok_or(NoSuitableAdapterFoundError)?;

	// Create the logical device and command queue
	let (device, queue) = adapter.request_device(
		&wgpu::DeviceDescriptor {
			limits: wgpu::Limits::default(),
			extensions: wgpu::Extensions::default(),
		},
	).await;

	Ok((device, queue))
}

fn create_window_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("window_bind_group_layout"),
		bindings: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStage::VERTEX,
				ty: wgpu::BindingType::UniformBuffer {
					dynamic: false,
				},
			},
		],
	})
}

fn create_image_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
	device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("image_bind_group_layout"),
		bindings: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStage::FRAGMENT,
				ty: wgpu::BindingType::UniformBuffer {
					dynamic: false,
				},
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStage::FRAGMENT,
				ty: wgpu::BindingType::StorageBuffer {
					readonly: true,
					dynamic: false,
				},
			},
		],
	})
}

fn create_render_pipeline(
	device: &wgpu::Device,
	layout: &wgpu::PipelineLayout,
	vertex_shader: &wgpu::ShaderModule,
	fragment_shader: &wgpu::ShaderModule,
	swap_chain_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
	device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		layout,
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
