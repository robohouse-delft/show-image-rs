pub struct Texture {
	pub texture: wgpu::Texture,
	pub sampler: wgpu::Sampler,
	pub bind_group: wgpu::BindGroup,
}

impl Texture {
	pub fn from_image(device: &wgpu::Device, bind_group_layout: &wgpu::BindGroupLayout, name: &str, image: &image::DynamicImage) -> (Self, wgpu::CommandBuffer) {
		let image = image.to_bgra();
		let size = wgpu::Extent3d {
			width: image.width(),
			height: image.height(),
			depth: 1,
		};

		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label: Some(name),
			size,
			array_layer_count: 1,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8UnormSrgb,
			usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
		});

		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			lod_min_clamp: -100.0,
			lod_max_clamp: 100.0,
			compare: wgpu::CompareFunction::Always,
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some(&format!("{}_bind_group", name)),
			layout: &bind_group_layout,
			bindings: &[
				wgpu::Binding {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&texture.create_default_view()),
				},
				wgpu::Binding {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&sampler),
				},
			],
		});

		// Copy the texture data.
		let buffer = device.create_buffer_with_data(&image, wgpu::BufferUsage::COPY_SRC);
		let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("copy_image")
		});

		encoder.copy_buffer_to_texture(
			wgpu::BufferCopyView {
				buffer: &buffer,
				offset: 0,
				bytes_per_row: 4 * image.width(),
				rows_per_image: image.height(),
			},
			wgpu::TextureCopyView {
				texture: &texture,
				mip_level: 0,
				array_layer: 0,
				origin: wgpu::Origin3d::ZERO,
			},
			size,
		);
		let commands = encoder.finish();

		let result = Self {
			texture,
			sampler,
			bind_group,
		};

		(result, commands)
	}
}
