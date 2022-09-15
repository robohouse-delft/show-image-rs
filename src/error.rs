//! Error types for the crate.

use crate::WindowId;

/// An error that can occur while creating a new window.
#[derive(Debug)]
pub enum CreateWindowError {
	/// The underlying call to `winit` reported an error.
	Winit(winit::error::OsError),

	/// Failed to get a suitable GPU device.
	GetDevice(GetDeviceError),
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
	/// The window ID is invalid.
	InvalidWindowId(InvalidWindowId),

	/// The image data is not supported.
	ImageDataError(ImageDataError),
}

/// The specified overlay was not found on the window.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnknownOverlay {
	/// The name of the overlay.
	pub name: String,
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

/// An error occured trying to save an image.
#[derive(Debug)]
pub enum SaveImageError {
	/// An I/O error occured.
	IoError(std::io::Error),

	/// An error occured encoding the PNG image.
	#[cfg(feature = "png")]
	PngError(png::EncodingError),
}

impl From<winit::error::OsError> for CreateWindowError {
	fn from(other: winit::error::OsError) -> Self {
		Self::Winit(other)
	}
}

impl From<GetDeviceError> for CreateWindowError {
	fn from(other: GetDeviceError) -> Self {
		Self::GetDevice(other)
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

impl From<std::io::Error> for SaveImageError {
	fn from(other: std::io::Error) -> Self {
		Self::IoError(other)
	}
}

#[cfg(feature = "png")]
impl From<png::EncodingError> for SaveImageError {
	fn from(other: png::EncodingError) -> Self {
		match other {
			png::EncodingError::IoError(e) => Self::IoError(e),
			e => Self::PngError(e),
		}
	}
}

impl std::error::Error for CreateWindowError {}
impl std::error::Error for ImageDataError {}
impl std::error::Error for UnsupportedImageFormat {}
impl std::error::Error for InvalidWindowId {}
impl std::error::Error for SetImageError {}
impl std::error::Error for UnknownOverlay {}
impl std::error::Error for GetDeviceError {}
impl std::error::Error for NoSuitableAdapterFound {}
impl std::error::Error for SaveImageError {}

impl std::fmt::Display for CreateWindowError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Winit(e) => write!(f, "{}", e),
			Self::GetDevice(e) => write!(f, "{}", e),
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

impl std::fmt::Display for UnknownOverlay {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "unknown overlay: {}", self.name)
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

impl std::fmt::Display for SaveImageError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::IoError(e) => write!(f, "{}", e),
			#[cfg(feature = "png")]
			Self::PngError(e) => write!(f, "{}", e),
		}
	}
}
