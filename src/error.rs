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

/// An error that can occur when setting the image of a window.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SetImageError {
	InvalidWindowId(InvalidWindowIdError),
	ImageDataError(ImageDataError),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GetDeviceError {
	NoSuitableAdapterFound(NoSuitableAdapterFoundError),
	NoSuitableDeviceFound(wgpu::RequestDeviceError),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NoSuitableAdapterFoundError;

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

impl std::error::Error for CreateWindowError {}
impl std::error::Error for ImageDataError {}
impl std::error::Error for UnsupportedImageFormatError {}
impl std::error::Error for InvalidWindowIdError {}
impl std::error::Error for SetImageError {}
impl std::error::Error for GetDeviceError {}
impl std::error::Error for NoSuitableAdapterFoundError {}

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

impl std::fmt::Display for UnsupportedImageFormatError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "unsupported image format: {}", self.format)
	}
}

impl std::fmt::Display for InvalidWindowIdError {
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

impl std::fmt::Display for NoSuitableAdapterFoundError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "no suitable graphics adapter found")
	}
}
