/// Information describing the binary data of an image.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ImageInfo {
	/// The pixel format of the image data.
	pub pixel_format: PixelFormat,

	/// The width of the image in pixels.
	pub width: u32,

	/// The height of the image in pixels.
	pub height: u32,

	/// The X stride of the image data in bytes.
	pub stride_x: u32,

	/// The Y stride of the image data in bytes.
	pub stride_y: u32,
}

/// Supported pixel formats.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PixelFormat {
	/// Interlaced 8-bit RGB data.
	Rgb8,

	/// Interlaced 8-bit RGBA data.
	Rgba8,

	/// Interlaced 8-bit BGR data.
	Bgr8,

	/// Interlaced 8-bit BGRA data.
	Bgra8,

	/// 8-bit monochrome data.
	Mono8,
}

impl ImageInfo {
	/// Create a new info struct with the given format, width and height.
	///
	/// The row stride is automatically calculated based on the image width and pixel format.
	/// If you wish to use a different row stride, construct the struct directly.
	pub fn new(pixel_format: PixelFormat, width: u32, height: u32) -> Self {
		let stride_x = u32::from(pixel_format.bytes_per_pixel());
		let stride_y = stride_x * width;
		Self { pixel_format, width, height, stride_x, stride_y }
	}

	/// Create a new 8-bit RGB info struct with the given width and height.
	pub fn rgb8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Rgb8, width, height)
	}

	/// Create a new 8-bit RGBA info struct with the given width and height.
	pub fn rgba8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Rgba8, width, height)
	}

	/// Create a new 8-bit BGR info struct with the given width and height.
	pub fn bgr8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Bgr8, width, height)
	}

	/// Create a new 8-bit BGRA info struct with the given width and height.
	pub fn bgra8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Bgra8, width, height)
	}

	/// Create a new 8-bit monochrome info struct with the given width and height.
	pub fn mono8(width: u32, height: u32) -> Self {
		Self::new(PixelFormat::Mono8, width, height)
	}

	/// Get the image size in bytes.
	pub fn byte_size(self) -> u64 {
		if self.stride_y >= self.stride_x {
			u64::from(self.stride_y) * u64::from(self.height)
		} else {
			u64::from(self.stride_x) * u64::from(self.width)
		}
	}
}

impl PixelFormat {
	/// Get the number of channels.
	pub fn channels(self) -> u8 {
		match self {
			PixelFormat::Bgr8  => 3,
			PixelFormat::Bgra8 => 4,
			PixelFormat::Rgb8  => 3,
			PixelFormat::Rgba8 => 4,
			PixelFormat::Mono8 => 1,
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
}
