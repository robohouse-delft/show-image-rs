/// Information describing the binary data of an image.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ImageInfo {
	/// The pixel format of the image data.
	pub pixel_format: PixelFormat,

	/// The size of the image in pixels
	pub size: glam::UVec2,

	/// The stride of the image data in bytes for both X and Y.
	pub stride: glam::UVec2,
}

/// Supported pixel formats.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PixelFormat {
	/// 8-bit monochrome data.
	Mono8,

	/// 8-bit monochrome data with alpha.
	MonoAlpha8(Alpha),

	/// Interlaced 8-bit BGR data.
	Bgr8,

	/// Interlaced 8-bit BGRA data.
	Bgra8(Alpha),

	/// Interlaced 8-bit RGB data.
	Rgb8,

	/// Interlaced 8-bit RGBA data.
	Rgba8(Alpha),
}

/// Possible alpha representations.
///
/// See also: <https://en.wikipedia.org/wiki/Alpha_compositing#Straight_versus_premultiplied>
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Alpha {
	/// The alpha channel is encoded only in the alpha component of the pixel.
	Unpremultiplied,

	/// The alpha channel is also premultiplied into the other components of the pixel.
	Premultiplied,
}

impl ImageInfo {
	/// Create a new info struct with the given format, width and height.
	///
	/// The row stride is automatically calculated based on the image width and pixel format.
	/// If you wish to use a different row stride, construct the struct directly.
	pub fn new(pixel_format: PixelFormat, width: u32, height: u32) -> Self {
		let stride_x = u32::from(pixel_format.bytes_per_pixel());
		let stride_y = stride_x * width;
		Self {
			pixel_format,
			size: glam::UVec2::new(width, height),
			stride: glam::UVec2::new(stride_x, stride_y),
		}
	}

	/// Create a new info struct for an 8-bit monochrome image with the given width and height.
	pub fn mono8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Mono8, width, height)
	}

	/// Create a new info struct for an 8-bit monochrome image with with alpha channel and the given width and height.
	pub fn mono_alpha8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::MonoAlpha8(Alpha::Unpremultiplied), width, height)
	}

	/// Create a new info struct for an 8-bit monochrome image with premultiplied alpha channel and the given width and height.
	pub fn mono_alpha8_premultiplied(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::MonoAlpha8(Alpha::Premultiplied), width, height)
	}

	/// Create a new info struct for an 8-bit BGR image with the given width and height.
	pub fn bgr8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Bgr8, width, height)
	}

	/// Create a new info struct for an 8-bit BGRA image with the given width and height.
	pub fn bgra8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Bgra8(Alpha::Unpremultiplied), width, height)
	}

	/// Create a new info struct for an 8-bit BGRA image with premultiplied alpha channel and the given width and height.
	pub fn bgra8_premultiplied(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Bgra8(Alpha::Premultiplied), width, height)
	}

	/// Create a new info struct for an 8-bit RGB image with the given width and height.
	pub fn rgb8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Rgb8, width, height)
	}

	/// Create a new info struct for an 8-bit RGBA image with the given width and height.
	pub fn rgba8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Rgba8(Alpha::Unpremultiplied), width, height)
	}

	/// Create a new info struct for an 8-bit RGBA image with premultiplied alpha channel and the given width and height.
	pub fn rgba8_premultiplied(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Rgba8(Alpha::Premultiplied), width, height)
	}

	/// Get the image size in bytes.
	pub fn byte_size(self) -> u64 {
		if self.stride.y >= self.stride.x {
			u64::from(self.stride.y) * u64::from(self.size.y)
		} else {
			u64::from(self.stride.x) * u64::from(self.size.x)
		}
	}
}

impl PixelFormat {
	/// Get the number of channels.
	pub fn channels(self) -> u8 {
		match self {
			PixelFormat::Mono8 => 1,
			PixelFormat::MonoAlpha8(_) => 1,
			PixelFormat::Bgr8 => 3,
			PixelFormat::Bgra8(_) => 4,
			PixelFormat::Rgb8 => 3,
			PixelFormat::Rgba8(_) => 4,
		}
	}

	/// Get the bytes per channel.
	const fn byte_depth(self) -> u8 {
		1
	}

	/// Get the bytes per pixel.
	pub fn bytes_per_pixel(self) -> u8 {
		self.byte_depth() * self.channels()
	}

	/// Get the alpha representation of the pixel format.
	///
	/// Returns [`None`], if the pixel format has no alpha channel.
	pub fn alpha(self) -> Option<Alpha> {
		match self {
			PixelFormat::Mono8 => None,
			PixelFormat::MonoAlpha8(a) => Some(a),
			PixelFormat::Bgr8 => None,
			PixelFormat::Bgra8(a) => Some(a),
			PixelFormat::Rgb8 => None,
			PixelFormat::Rgba8(a) => Some(a),
		}
	}
}
