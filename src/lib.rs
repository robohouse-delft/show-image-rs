mod backend;
mod features;
mod oneshot;

pub use keyboard_types::Code as ScanCode;
pub use keyboard_types::Key as KeyCode;
pub use keyboard_types::KeyState;
pub use keyboard_types::KeyboardEvent;
pub use keyboard_types::Location as KeyLocation;
pub use keyboard_types::Modifiers as KeyModifiers;

pub use backend::Context;
pub use backend::Window;

/// Supported pixel formats.
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

/// Information describing the binary data of an image.
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

/// Allows a type to be displayed as an image.
pub trait ImageData {
	fn data(&self) -> &[u8];
	fn info(&self) -> Result<ImageInfo, String>;
}

/// Options for creating a window.
pub struct WindowOptions {
	/// The name of the window.
	pub name: String,

	/// The initial size of the window in pixel.
	///
	/// This may be ignored by a window manager.
	pub size: [u32; 2],

	/// If true allow the window to be resized.
	///
	/// This may be ignored by a window manager.
	pub resizable: bool,

	/// Preserve the aspact ratio
	pub preserve_aspect_ratio: bool,
}
