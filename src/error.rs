use crate::WindowId;

pub use winit::error::OsError;

#[derive(Debug, Clone)]
pub struct InvalidWindowIdError {
	pub window_id: WindowId,
}

#[derive(Debug, Clone)]
pub struct NoSuitableAdapterFoundError;

#[derive(Debug, Clone)]
pub enum GetDeviceError {
	NoSuitableAdapterFound(NoSuitableAdapterFoundError),
	NoSuitableDeviceFound(wgpu::RequestDeviceError),
}

/// The context event loop was closed.
#[derive(Debug, Clone)]
pub struct EventLoopClosedError;

/// An error that can occur while creating a window through a proxy object.
#[derive(Debug)]
pub enum ProxyCreateWindowError {
	EventLoopClosed(EventLoopClosedError),
	Os(OsError),
}

/// An error that can occur while creating a window through a proxy object.
#[derive(Debug)]
pub enum ProxyWindowOperationError {
	EventLoopClosed(EventLoopClosedError),
	InvalidWindowId(InvalidWindowIdError),
}

impl From<NoSuitableAdapterFoundError> for GetDeviceError {
	fn from(other: NoSuitableAdapterFoundError) -> Self {
		Self::NoSuitableAdapterFound(other)
	}
}

impl From<wgpu::RequestDeviceError> for GetDeviceError {
	fn from(other: wgpu::RequestDeviceError) -> Self {
		Self::NoSuitableDeviceFound(other)
	}
}

impl From<EventLoopClosedError> for ProxyCreateWindowError {
	fn from(other: EventLoopClosedError) -> Self {
		Self::EventLoopClosed(other)
	}
}

impl From<OsError> for ProxyCreateWindowError {
	fn from(other: OsError) -> Self {
		Self::Os(other)
	}
}

impl From<EventLoopClosedError> for ProxyWindowOperationError {
	fn from(other: EventLoopClosedError) -> Self {
		Self::EventLoopClosed(other)
	}
}

impl From<InvalidWindowIdError> for ProxyWindowOperationError {
	fn from(other: InvalidWindowIdError) -> Self {
		Self::InvalidWindowId(other)
	}
}
