mod features;
mod backend;

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
