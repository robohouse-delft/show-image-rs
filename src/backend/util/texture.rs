use crate::Image;
use crate::PixelFormat;
use super::buffer::create_buffer_with_value;

pub struct Texture {
	size: [u32; 2],
	bind_group: wgpu::BindGroup,
	_uniforms: wgpu::Buffer,
	_data: wgpu::Buffer,
}

#[derive(Debug, Copy, Clone)]
pub struct TextureUniforms {
	format: u32,
	width: u32,
	height: u32,
	stride_x: u32,
	stride_y: u32,
}

impl Texture {
	pub fn from_data(
		device: &wgpu::Device,
		bind_group_layout: &wgpu::BindGroupLayout,
		name: &str,
		image: &Image,
	) -> Self {

		let format = match image.info().pixel_format {
			PixelFormat::Mono8 => 0,
			PixelFormat::Bgr8 => 1,
			PixelFormat::Bgra8 => 2,
			PixelFormat::Rgb8 => 3,
			PixelFormat::Rgba8 => 4,
		};

		let uniforms = TextureUniforms {
			format,
			width: image.info().width,
			height: image.info().height,
			stride_x: image.info().stride_x,
			stride_y: image.info().stride_y,
		};
		let uniforms = create_buffer_with_value(device, &uniforms, wgpu::BufferUsage::UNIFORM);

		let data = device.create_buffer_with_data(image.buffer(), wgpu::BufferUsage::STORAGE_READ);

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(&format!("{}_bind_group", name)),
			layout: &bind_group_layout,
			bindings: &[
				wgpu::Binding {
					binding: 0,
					resource: wgpu::BindingResource::Buffer {
						buffer: &uniforms,
						range: 0..std::mem::size_of::<TextureUniforms>() as u64,
					},
				},
				wgpu::Binding {
					binding: 1,
					resource: wgpu::BindingResource::Buffer {
						buffer: &data,
						range: 0..image.info().byte_size(),
					},
				},
			],
		});

		Self {
			size: [image.info().width, image.info().height],
			bind_group,
			_uniforms: uniforms,
			_data: data,
		}
	}

	pub fn size(&self) -> [u32; 2] {
		self.size
	}

	pub fn width(&self) -> u32 {
		self.size[0]
	}

	pub fn height(&self) -> u32 {
		self.size[1]
	}

	pub fn bind_group(&self) -> &wgpu::BindGroup {
		&self.bind_group
	}
}
