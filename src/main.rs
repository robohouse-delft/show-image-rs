use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowId;

pub mod error;

mod buffer;
mod proxy;
mod texture;
mod uniforms_buffer;

pub use proxy::ContextProxy;

use error::InvalidWindowIdError;
use error::NoSuitableAdapterFoundError;
use proxy::ContextCommand;
use texture::Texture;
use uniforms_buffer::UniformsBuffer;

pub mod oneshot;
pub use wgpu::Color;

macro_rules! include_spirv {
	($path:literal) => {{
		let bytes = include_bytes!($path);
		wgpu::read_spirv(std::io::Cursor::new(&bytes[..])).unwrap()
	}};
}

fn main() {
	let args : Vec<_> = std::env::args().collect();
	let image = image::open(args.get(1).unwrap()).unwrap();

	let context = Context::new(wgpu::TextureFormat::Bgra8UnormSrgb).unwrap();
	let proxy = context.proxy();

	std::thread::spawn(move || fake_main(image, proxy));
	context.run(|_context, _command: ()| ());
}

fn fake_main(image: image::DynamicImage, proxy: ContextProxy<()>) {
	let window = proxy.create_window("Show Image").unwrap();
	window.set_image("image", image).unwrap();
}

pub struct Context<CustomEvent: 'static> {
	event_loop: Option<EventLoop<ContextCommand<CustomEvent>>>,
	proxy: ContextProxy<CustomEvent>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	swap_chain_format: wgpu::TextureFormat,
	window_bind_group_layout: wgpu::BindGroupLayout,
	image_bind_group_layout: wgpu::BindGroupLayout,
	render_pipeline: wgpu::RenderPipeline,

	windows: Vec<Window>,
}

pub struct ContextHandle<'a, CustomEvent: 'static> {
	context: &'a mut Context<CustomEvent>,
	event_loop: &'a winit::event_loop::EventLoopWindowTarget<ContextCommand<CustomEvent>>,
}

pub struct Window {
	window: winit::window::Window,
	options: WindowOptions,
	surface: wgpu::Surface,
	swap_chain: wgpu::SwapChain,
	uniforms: UniformsBuffer<WindowUniforms>,
	image: Option<Texture>,
	load_texture: Option<wgpu::CommandBuffer>,
}

#[derive(Debug, Clone)]
pub struct WindowOptions {
	preserve_aspect_ratio: bool,
	background_color: Color,
	start_hidden: bool,
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
	fn id(&self) -> WindowId {
		self.window.id()
	}

	fn calculate_uniforms(&self) -> WindowUniforms {
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
struct WindowUniforms {
	scale: [f32; 2],
}

impl Default for WindowUniforms {
	fn default() -> Self {
		Self {
			scale: [1.0, 1.0],
		}
	}
}

impl<CustomEvent> Context<CustomEvent> {
	fn new(swap_chain_format: wgpu::TextureFormat) -> Result<Self, NoSuitableAdapterFoundError> {
		let event_loop = EventLoop::with_user_event();
		let proxy = ContextProxy::new(event_loop.create_proxy());

		let (device, queue) = futures::executor::block_on(get_device())?;

		let window_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
		});

		let image_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("image_bind_group_layout"),
			bindings: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStage::FRAGMENT,
					ty: wgpu::BindingType::SampledTexture {
						multisampled: false,
						dimension: wgpu::TextureViewDimension::D2,
						component_type: wgpu::TextureComponentType::Uint,
					},
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStage::FRAGMENT,
					ty: wgpu::BindingType::Sampler {
						comparison: false,
					},
				},
			],
		});

		let vertex_shader = device.create_shader_module(&include_spirv!("../shaders/shader.vert.spv"));
		let fragment_shader = device.create_shader_module(&include_spirv!("../shaders/shader.frag.spv"));

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			bind_group_layouts: &[
				&window_bind_group_layout,
				&image_bind_group_layout,
			],
		});

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			layout: &pipeline_layout,
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
		});

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
		})
	}

	pub fn proxy(&self) -> ContextProxy<CustomEvent> {
		self.proxy.clone()
	}

	pub fn create_window(
		&mut self,
		event_loop: &winit::event_loop::EventLoopWindowTarget<ContextCommand<CustomEvent>>,
		title: impl Into<String>,
		options: WindowOptions,
	) -> Result<WindowId, winit::error::OsError> {
		let window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_visible(!options.start_hidden)
			.build(event_loop)?;

		let surface = wgpu::Surface::create(&window);
		let swap_chain = make_swap_chain(window.inner_size(), &surface, self.swap_chain_format, &self.device);
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

	pub fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		let index = self.windows.iter().position(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;
		self.windows.remove(index);
		Ok(())
	}

	pub fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;
		window.window.set_visible(visible);
		Ok(())
	}

	pub fn set_window_image(&mut self, window_id: WindowId, name: &str, image: &image::DynamicImage) -> Result<(), InvalidWindowIdError> {
		let window = self.windows.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		let (texture, load_commands) = Texture::from_image(&self.device, &self.image_bind_group_layout, name, image);
		window.load_texture = Some(load_commands);
		window.image = Some(texture);
		window.uniforms.mark_dirty(true);
		Ok(())
	}

	fn resize_window(&mut self, window_id: WindowId, new_size: winit::dpi::PhysicalSize<u32>) -> Result<(), InvalidWindowIdError> {
		let window = self.windows
			.iter_mut()
			.find(|w| w.id() == window_id)
			.ok_or_else(|| InvalidWindowIdError { window_id })?;

		window.swap_chain = make_swap_chain(new_size, &window.surface, self.swap_chain_format, &self.device);
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

	fn run<CustomHandler>(
		mut self,
		mut custom_handler: CustomHandler
	) -> !
	where
		CustomHandler: 'static + FnMut(&mut Self, CustomEvent),
	{
		let event_loop = self.event_loop.take().unwrap();
		event_loop.run(move |event, event_loop, control_flow| {
			*control_flow = ControlFlow::Poll;
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
				Event::UserEvent(command) => {
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
						ContextCommand::ExecuteFunction(command) => {
							(command.function)(ContextHandle::new(&mut self, event_loop));
						},
						ContextCommand::Custom(command) => {
							custom_handler(&mut self, command);
						},
					}
				}
				_ => {},
			}
		});
	}
}

impl<'a, CustomEvent: 'static> ContextHandle<'a, CustomEvent> {
	pub fn new(
		context: &'a mut Context<CustomEvent>,
		event_loop: &'a winit::event_loop::EventLoopWindowTarget<ContextCommand<CustomEvent>>,
	) -> Self {
		Self { context, event_loop }
	}

	pub fn proxy(&self) -> ContextProxy<CustomEvent> {
		self.context.proxy()
	}

	pub fn create_window(&mut self, title: impl Into<String>, options: WindowOptions) -> Result<WindowId, winit::error::OsError> {
		self.context.create_window(self.event_loop, title, options)
	}

	pub fn destroy_window(&mut self, window_id: WindowId) -> Result<(), InvalidWindowIdError> {
		self.context.destroy_window(window_id)
	}

	pub fn set_window_visible(&mut self, window_id: WindowId, visible: bool) -> Result<(), InvalidWindowIdError> {
		self.context.set_window_visible(window_id, visible)
	}

	pub fn set_window_image(&mut self, window_id: WindowId, name: &str, image: &image::DynamicImage) -> Result<(), InvalidWindowIdError> {
		self.context.set_window_image(window_id, name, image)
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

fn make_swap_chain(size: winit::dpi::PhysicalSize<u32>, surface: &wgpu::Surface, format: wgpu::TextureFormat, device: &wgpu::Device) -> wgpu::SwapChain {
	let swap_chain_desc = wgpu::SwapChainDescriptor {
		usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
		format,
		width: size.width,
		height: size.height,
		present_mode: wgpu::PresentMode::Mailbox,
	};

	device.create_swap_chain(&surface, &swap_chain_desc)
}
