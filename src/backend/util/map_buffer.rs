use std::sync::{Arc, Mutex};

/// Synchronously wait for a buffer to be mappable.
fn wait_for_buffer(
	device: &wgpu::Device,
	buffer: wgpu::BufferSlice<'_>,
	map_mode: wgpu::MapMode,
) -> Result<(), wgpu::BufferAsyncError> {
	let result = Arc::new(Mutex::new(None));
	buffer.map_async(map_mode, {
		let result = result.clone();
		move |new_result| {
			*result.lock().unwrap() = Some(new_result);
		}
	});

	loop {
		device.poll(wgpu::Maintain::Wait);
		if let Some(result) = result.lock().unwrap().take() {
			return result;
		}
	}
}

/// Synchronously map a buffer for read access.
///
/// This will internally call [`wgpu::Device::poll()`] until the buffer is ready, and then map it.
#[allow(unused)]
pub fn map_buffer<'a>(device: &wgpu::Device, buffer: wgpu::BufferSlice<'a>) -> Result<wgpu::BufferView<'a>, wgpu::BufferAsyncError> {
	wait_for_buffer(device, buffer, wgpu::MapMode::Read)?;
	Ok(buffer.get_mapped_range())
}

/// Synchronously map a buffer for write access.
///
/// This will internally call [`wgpu::Device::poll()`] until the buffer is ready, and then map it.
#[allow(unused)]
pub fn map_buffer_mut<'a>(device: &wgpu::Device, buffer: wgpu::BufferSlice<'a>) -> Result<wgpu::BufferViewMut<'a>, wgpu::BufferAsyncError> {
	wait_for_buffer(device, buffer, wgpu::MapMode::Write)?;
	Ok(buffer.get_mapped_range_mut())
}
