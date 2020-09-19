//! Error types for the crate.

use crate::WindowId;

/// An error that can occur while creating a new window.
#[derive(Debug)]
pub enum CreateWindowError {
	/// The underlying call to `winit` reported an error.
	Winit(winit::error::OsError),
}

/// An error that can occur while interpreting image data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ImageDataError {
	/// The image data is not in a supported format.
	UnsupportedImageFormat(UnsupportedImageFormat),

	/// An other error occured.
	Other(String),
}

/// An error indicating that the image data is not in a supported format.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnsupportedImageFormat {
	/// The unsupported format.
	pub format: String,
}

/// The window ID is not valid.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InvalidWindowId {
	/// The invalid window ID.
	pub window_id: WindowId,
}

/// An error that can occur when setting the image of a window.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SetImageError {
	InvalidWindowId(InvalidWindowId),
	ImageDataError(ImageDataError),
}

/// An error occured trying to find a usable graphics device.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GetDeviceError {
	/// No suitable video adapter was found.
	NoSuitableAdapterFound(NoSuitableAdapterFound),

	/// No suitable graphics device was found.
	NoSuitableDeviceFound(wgpu::RequestDeviceError),
}

/// No suitable video adapter was found.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoSuitableAdapterFound;

impl From<winit::error::OsError> for CreateWindowError {
	fn from(other: winit::error::OsError) -> Self {
		Self::Winit(other)
	}
}

impl From<ImageDataError> for SetImageError {
	fn from(other: ImageDataError) -> Self {
		Self::ImageDataError(other)
	}
}

impl From<InvalidWindowId> for SetImageError {
	fn from(other: InvalidWindowId) -> Self {
		Self::InvalidWindowId(other)
	}
}

impl From<UnsupportedImageFormat> for ImageDataError {
	fn from(other: UnsupportedImageFormat) -> Self {
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

impl From<NoSuitableAdapterFound> for GetDeviceError {
	fn from(other: NoSuitableAdapterFound) -> Self {
		Self::NoSuitableAdapterFound(other)
	}
}

impl From<wgpu::RequestDeviceError> for GetDeviceError {
	fn from(other: wgpu::RequestDeviceError) -> Self {
		Self::NoSuitableDeviceFound(other)
	}
}

impl std::error::Error for CreateWindowError {}
impl std::error::Error for ImageDataError {}
impl std::error::Error for UnsupportedImageFormat {}
impl std::error::Error for InvalidWindowId {}
impl std::error::Error for SetImageError {}
impl std::error::Error for GetDeviceError {}
impl std::error::Error for NoSuitableAdapterFound {}

impl std::fmt::Display for CreateWindowError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Winit(e) => write!(f, "{}", e),
		}
	}
}

impl std::fmt::Display for ImageDataError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::UnsupportedImageFormat(e) => write!(f, "{}", e),
			Self::Other(e) => write!(f, "{}", e),
		}
	}
}

impl std::fmt::Display for UnsupportedImageFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "unsupported image format: {}", self.format)
	}
}

impl std::fmt::Display for InvalidWindowId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "invalid window ID: {:?}", self.window_id)
	}
}

impl std::fmt::Display for SetImageError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::InvalidWindowId(e) => write!(f, "{}", e),
			Self::ImageDataError(e) => write!(f, "{}", e),
		}
	}
}

impl std::fmt::Display for GetDeviceError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::NoSuitableAdapterFound(e) => write!(f, "{}", e),
			Self::NoSuitableDeviceFound(e) => write!(f, "{}", e),
		}
	}
}

impl std::fmt::Display for NoSuitableAdapterFound {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "no suitable graphics adapter found")
	}
}
