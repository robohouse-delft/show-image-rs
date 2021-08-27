/// Reinterpret an object as bytes.
unsafe fn as_bytes<T>(value: &T) -> &[u8] {
	std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of_val(value))
}

/// Create a [`wgpu::Buffer`] with an arbitrary object as contents.
pub fn create_buffer_with_value<T: Copy>(device: &wgpu::Device, label: Option<&str>, value: &T, usage: wgpu::BufferUsages) -> wgpu::Buffer {
	use wgpu::util::DeviceExt;
	unsafe {
		let contents = as_bytes(value);
		device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label, contents, usage })
	}
}
