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

pub enum PixelFormat {
	Rgb8,
	Rgba8,
	Bgr8,
	Bgra8,
	Mono8,
}

pub struct ImageInfo {
	width: usize,
	height: usize,
	row_stride: usize,
	pixel_format: PixelFormat,
}

pub trait ImageData {
	fn data(&self) -> &[u8];
	fn info(&self) -> Result<ImageInfo, String>;
}
