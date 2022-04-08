use crate::ImageInfo;
use crate::ImageView;
use crate::{Alpha, PixelFormat};
use super::buffer::create_buffer_with_value;

/// A GPU image buffer ready to be used with the rendering pipeline.
pub struct GpuImage {
	name: String,
	info: ImageInfo,
	bind_group: wgpu::BindGroup,
	_uniforms: wgpu::Buffer,
	_data: wgpu::Buffer,
}

/// The uniforms associated with a [`GpuImage`].
#[derive(Debug, Copy, Clone)]
#[allow(unused)] // All fields are used by the GPU.
pub struct GpuImageUniforms {
	format: u32,
	width: u32,
	height: u32,
	stride_x: u32,
	stride_y: u32,
}

impl GpuImage {
	/// Create a [`GpuImage`] from an image buffer.
	pub fn from_data(name: String, device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout, image: &ImageView) -> Self {
		let info = image.info();

		let format = match info.pixel_format {
			PixelFormat::Mono8 => 0,
			PixelFormat::MonoAlpha8(Alpha::Unpremultiplied) => 1,
			PixelFormat::MonoAlpha8(Alpha::Premultiplied) => 2,
			PixelFormat::Bgr8 => 3,
			PixelFormat::Bgra8(Alpha::Unpremultiplied) => 4,
			PixelFormat::Bgra8(Alpha::Premultiplied) => 5,
			PixelFormat::Rgb8 => 6,
			PixelFormat::Rgba8(Alpha::Unpremultiplied) => 7,
			PixelFormat::Rgba8(Alpha::Premultiplied) => 8,
		};

		let uniforms = GpuImageUniforms {
			format,
			width: info.size.x,
			height: info.size.y,
			stride_x: info.stride.x,
			stride_y: info.stride.y,
		};

		let uniforms = create_buffer_with_value(
			device,
			Some(&format!("{}_uniforms_buffer", name)),
			&uniforms,
			wgpu::BufferUsages::UNIFORM,
		);

		use wgpu::util::DeviceExt;
		let data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some(&format!("{}_image_buffer", name)),
			contents: image.data(),
			usage: wgpu::BufferUsages::STORAGE,
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(&format!("{}_bind_group", name)),
			layout: bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
						buffer: &uniforms,
						offset: 0,
						size: None, // Use entire buffer.
					}),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
						buffer: &data,
						offset: 0,
						size: None, // Use entire buffer.
					}),
				},
			],
		});

		Self {
			name,
			info,
			bind_group,
			_uniforms: uniforms,
			_data: data,
		}
	}

	/// Get the name of the image.
	#[allow(unused)]
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Get the image info.
	pub fn info(&self) -> &ImageInfo {
		&self.info
	}

	/// Get the bind group that should be used to render the image with the rendering pipeline.
	pub fn bind_group(&self) -> &wgpu::BindGroup {
		&self.bind_group
	}
}
