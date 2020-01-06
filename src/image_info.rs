/// Information describing the binary data of an image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageInfo {
	/// The width of the image in pixels.
	pub width: usize,

	/// The height of the image in pixels.
	pub height: usize,

	/// The row stride of the image data in bytes.
	///
	/// The data is assumed to be stored row-major.
	/// The stride is the byte offset between two rows in the data.
	pub row_stride: usize,

	/// The pixel format of the image data.
	pub pixel_format: PixelFormat,
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

	/// 8-bit grayscale data.
	Mono8,
}

impl ImageInfo {
	/// Create a new info struct with the given format, width and height.
	///
	/// The row stride is automatically calculated based on the image width and pixel format.
	/// If you wish to use a different row stride, construct the struct directly.
	pub fn new(pixel_format: PixelFormat, width: usize, height: usize) -> Self {
		let row_stride = usize::from(pixel_format.bytes_per_pixel()) * width;
		Self { pixel_format, width, height, row_stride}
	}

	/// Create a new 8-bit RGB info struct with the given width and height.
	pub fn rgb8(width: usize, height: usize) -> Self {
		Self::new(PixelFormat::Rgb8, width, height)
	}

	/// Create a new 8-bit RGBA info struct with the given width and height.
	pub fn rgba8(width: usize, height: usize) -> Self {
		Self::new(PixelFormat::Rgba8, width, height)
	}

	/// Create a new 8-bit BGR info struct with the given width and height.
	pub fn bgr8(width: usize, height: usize) -> Self {
		Self::new(PixelFormat::Bgr8, width, height)
	}

	/// Create a new 8-bit BGRA info struct with the given width and height.
	pub fn bgra8(width: usize, height: usize) -> Self {
		Self::new(PixelFormat::Bgra8, width, height)
	}

	/// Create a new 8-bit grayscale info struct with the given width and height.
	pub fn mono8(width: usize, height: usize) -> Self {
		Self::new(PixelFormat::Mono8, width, height)
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
