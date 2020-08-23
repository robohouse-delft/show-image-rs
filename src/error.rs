use crate::WindowId;

pub use winit::error::OsError;

/// An error that can occur when setting the image of a window.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SetImageError {
	InvalidWindowId(InvalidWindowIdError),
	ImageDataError(ImageDataError),
}

/// An error that can occur while interpreting image data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ImageDataError {
	/// The image data is not in a supported format.
	UnsupportedImageFormat(UnsupportedImageFormatError),

	/// An other error occured.
	Other(String),
}

/// An error indicating that the image data is not in a supported format.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnsupportedImageFormatError {
	/// The unsupported format.
	pub format: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidWindowIdError {
	pub window_id: WindowId,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoSuitableAdapterFoundError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GetDeviceError {
	NoSuitableAdapterFound(NoSuitableAdapterFoundError),
	NoSuitableDeviceFound(wgpu::RequestDeviceError),
}

/// The context event loop was closed.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EventLoopClosedError;

/// An error that can occur while creating a window through a proxy object.
#[derive(Debug)]
pub enum ProxyCreateWindowError {
	EventLoopClosed(EventLoopClosedError),
	Os(OsError),
}

/// An error that can occur while creating a window through a proxy object.
#[derive(Debug, Eq, PartialEq)]
pub enum ProxyWindowOperationError {
	EventLoopClosed(EventLoopClosedError),
	InvalidWindowId(InvalidWindowIdError),
}

/// An error that can occur while creating a window through a proxy object.
#[derive(Debug, Eq, PartialEq)]
pub enum ProxySetImageError {
	EventLoopClosed(EventLoopClosedError),
	SetImageError(SetImageError),
}

impl From<ImageDataError> for SetImageError {
	fn from(other: ImageDataError) -> Self {
		Self::ImageDataError(other)
	}
}

impl From<InvalidWindowIdError> for SetImageError {
	fn from(other: InvalidWindowIdError) -> Self {
		Self::InvalidWindowId(other)
	}
}

impl From<UnsupportedImageFormatError> for ImageDataError {
	fn from(other: UnsupportedImageFormatError) -> Self {
		Self::UnsupportedImageFormat(other)
	}
}

impl From<String> for ImageDataError {
	fn from(other: String) -> Self {
		Self::Other(other)
	}
}

impl<'a> From<&'a str> for ImageDataError {
	fn from(other: &'a str) -> Self {
		Self::Other(other.to_string())
	}
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

impl From<EventLoopClosedError> for ProxySetImageError {
	fn from(other: EventLoopClosedError) -> Self {
		Self::EventLoopClosed(other)
	}
}

impl From<SetImageError> for ProxySetImageError {
	fn from(other: SetImageError) -> Self {
		Self::SetImageError(other)
	}
}
