use crate::ImageView;
use crate::PixelFormat;
use super::buffer::create_buffer_with_value;

/// A GPU image buffer ready to be used with the rendering pipeline.
pub struct GpuImage {
	size: [u32; 2],
	bind_group: wgpu::BindGroup,
	_uniforms: wgpu::Buffer,
	_data: wgpu::Buffer,
}

/// The uniforms associated with a [`GpuImage`].
#[derive(Debug, Copy, Clone)]
pub struct GpuImageUniforms {
	format: u32,
	width: u32,
	height: u32,
	stride_x: u32,
	stride_y: u32,
}

impl GpuImage {
	/// Create a [`GpuImage`] from an image buffer.
	pub fn from_data(
		device: &wgpu::Device,
		bind_group_layout: &wgpu::BindGroupLayout,
		name: &str,
		image: ImageView,
	) -> Self {
		let format = match image.info().pixel_format {
			PixelFormat::Mono8 => 0,
			PixelFormat::Bgr8 => 1,
			PixelFormat::Bgra8 => 2,
			PixelFormat::Rgb8 => 3,
			PixelFormat::Rgba8 => 4,
		};

		let uniforms = GpuImageUniforms {
			format,
			width: image.info().width,
			height: image.info().height,
			stride_x: image.info().stride_x,
			stride_y: image.info().stride_y,
		};

		let uniforms = create_buffer_with_value(device, Some(&format!("{}_uniforms_buffer", name)), &uniforms, wgpu::BufferUsage::UNIFORM);

		use wgpu::util::DeviceExt;
		let data = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some(&format!("{}_image_buffer", name)),
			contents: image.data(),
			usage: wgpu::BufferUsage::STORAGE,
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(&format!("{}_bind_group", name)),
			layout: &bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Buffer(uniforms.slice(..)),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Buffer(data.slice(..)),
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

	/// Get the dimensions of the image.
	pub fn size(&self) -> [u32; 2] {
		self.size
	}

	/// Get the width of the image.
	pub fn width(&self) -> u32 {
		self.size[0]
	}

	/// Get the height of the image.
	pub fn height(&self) -> u32 {
		self.size[1]
	}

	/// Get the bind group that should be used to render the image with the rendering pipeline.
	pub fn bind_group(&self) -> &wgpu::BindGroup {
		&self.bind_group
	}
}
