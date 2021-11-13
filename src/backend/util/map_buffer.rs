use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::RawWaker;
use std::task::RawWakerVTable;
use std::task::Waker;

/// A vtable with all no-ops.
#[rustfmt::skip]
static NULL_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
	|_| raw_null_waker(),
	|_| (),
	|_| (),
	|_| (),
);

/// Create a raw null waker that does nothing.
fn raw_null_waker() -> RawWaker {
	RawWaker::new(std::ptr::null(), &NULL_WAKER_VTABLE)
}

/// Create a null waker that does nothing.
fn null_waker() -> Waker {
	unsafe { Waker::from_raw(raw_null_waker()) }
}

/// Synchronously wait for a buffer to be mappable.
fn wait_for_buffer(
	device: &wgpu::Device,
	buffer: wgpu::BufferSlice<'_>,
	map_mode: wgpu::MapMode,
) -> Result<(), wgpu::BufferAsyncError> {
	let mut future = buffer.map_async(map_mode);
	let waker = null_waker();

	loop {
		let future = Pin::new(&mut future);
		match future.poll(&mut Context::from_waker(&waker)) {
			Poll::Ready(x) => return x,
			Poll::Pending => device.poll(wgpu::Maintain::Wait),
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
