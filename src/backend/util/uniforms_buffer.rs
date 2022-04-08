use super::buffer::create_buffer_with_value;

/// Trait for data in Std140 compatible layout.
///
/// # Safety
/// Implementing this trait indicates that the data is in Std140 compatible layout.
/// If that is not true, the GPU may perform illegal memory access.
pub unsafe trait ToStd140 {
	type Output: Copy;

	const STD140_SIZE: u64 = std::mem::size_of::<Self::Output>() as u64;

	fn to_std140(&self) -> Self::Output;
}

/// A buffer holding uniform data and matching bind group.
///
/// The buffer can be marked as dirty to indicate the contents need to be updated.
/// The contents can be updated with [`Self::update_from`],
/// which will also clear the dirty flag.
pub struct UniformsBuffer<T> {
	buffer: wgpu::Buffer,
	bind_group: wgpu::BindGroup,
	dirty: bool,
	_phantom: std::marker::PhantomData<fn(&T)>,
}

impl<T: ToStd140> UniformsBuffer<T> {
	/// Create a new UniformsBuffer from the given value and bind group layout.
	///
	/// The bind group layout must have exactly 1 binding for a buffer at index 0.
	pub fn from_value(device: &wgpu::Device, value: &T, layout: &wgpu::BindGroupLayout) -> Self {
		let buffer = create_buffer_with_value(device, None, &value.to_std140(), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("uniforms_bind_group"),
			layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
					buffer: &buffer,
					offset: 0,
					size: None, // Use entire buffer.
				}),
			}],
		});

		Self {
			buffer,
			bind_group,
			dirty: false,
			_phantom: std::marker::PhantomData,
		}
	}

	/// Get the bind group for the uniforms.
	pub fn bind_group(&self) -> &wgpu::BindGroup {
		&self.bind_group
	}

	/// Check if the uniforms are marked as dirty.
	pub fn is_dirty(&self) -> bool {
		self.dirty
	}

	/// Mark the uniforms as dirty.
	pub fn mark_dirty(&mut self, dirty: bool) {
		self.dirty = dirty;
	}

	/// Update the buffer contents using the provided command encoder and clear the dirty flag.
	pub fn update_from(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, value: &T) {
		let buffer = create_buffer_with_value(device, None, &value.to_std140(), wgpu::BufferUsages::COPY_SRC);
		encoder.copy_buffer_to_buffer(&buffer, 0, &self.buffer, 0, T::STD140_SIZE as wgpu::BufferAddress);
		self.mark_dirty(false);
	}
}
