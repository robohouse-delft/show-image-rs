unsafe fn as_bytes<T>(value: &T) -> &[u8] {
	std::slice::from_raw_parts(value as *const T as *const u8, std::mem::size_of_val(value))
}

pub fn create_buffer_with_value<T>(device: &wgpu::Device, value: &T, usage: wgpu::BufferUsage) -> wgpu::Buffer {
	unsafe {
		let bytes = as_bytes(value);
		device.create_buffer_with_data(bytes, usage)
	}
}
