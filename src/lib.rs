//! `show-image` is a library for quickly displaying images.
//! It is intended as a debugging aid for writing image processing code.
//! The library is not intended for making full-featured GUIs,
//! but you can process keyboard events from the created windows.
//!
//! # Supported image types.
//! The library aims to support as many different data types used to represent images.
//! To keep the dependency graph as small as possible,
//! support for third party libraries must be enabled explicitly with feature flags.
//!
//! Currently, the following types are supported:
//!   * Tuples of binary data and [`ImageInfo`].
//!   * [`image::DynamicImage`] and [`image::ImageBuffer`] with the `image` feature.
//!   * [`tch::Tensor`](::tch::Tensor) with the `tch` feature.
//!
//! If you think support for a some data type is missing,
//! feel free to send a PR or create an issue on GitHub.
//!
//! # Event handling.
//! You can receive events using [`Window::events`].
//! This is a general channel on which all events for that window are sent.
//! Alternatively you can use [`Window::add_event_handler`] to register an asynchronous event handler.
//! This event handler will run in the context thread, so shouldn't block for too long.
//!
//! You can also handle keyboard events for windows using [`Window::wait_key`] or [`Window::wait_key_deadline`].
//! These functions will wait for key press events while discarding key up events.
//!
//! # Saving displayed images.
//! If the `save` feature is enabled, windows allow the displayed image to be saved using `Ctrl+S`.
//! This will open a file dialog to save the currently displayed image.
//!
//! Note that images are saved in a background thread.
//! To ensure that no data loss occurs, call [`stop`] to gracefully stop and join the background thread.
//!
//! # Example 1: Showing an image.
//! This example uses a tuple of `(&[u8], `[`ImageInfo`]`)` as image,
//! but any type that implements [`ImageData`] will do.
//! ```no_run
//! # use image;
//! # use std::time::Duration;
//! # let pixel_data = &[0u8][..];
//! use show_image::{ImageInfo, make_window};
//!
//! let image = (pixel_data, ImageInfo::rgb8(1920, 1080));
//!
//! // Create a window and display the image.
//! let window = make_window("image")?;
//! window.set_image(image, "image-001")?;
//!
//! # Result::<(), String>::Ok(())
//! ```
//!
//! # Example 2: Handling keyboard events.
//! ```no_run
//! # use std::time::Duration;
//! # use show_image::ImageInfo;
//! use show_image::{KeyCode, make_window};
//!
//! // Create a window and display the image.
//! # let image = (&[0u8][..], ImageInfo::rgb8(1920, 1080));
//! let window = make_window("image")?;
//! window.set_image(&image, "image-001")?;
//!
//! // Print keyboard events until Escape is pressed, then exit.
//! // If the user closes the window, wait_key() will return an error and the loop also exits.
//! while let Ok(event) = window.wait_key(Duration::from_millis(100)) {
//!     if let Some(event) = event {
//!         println!("{:#?}", event);
//!         if event.key == KeyCode::Escape {
//!             break;
//!         }
//!     }
//! }
//!
//! // Make sure all background tasks are stopped cleanly.
//! show_image::stop()?;
//! # Result::<(), String>::Ok(())
//! ```

pub use keyboard_types::Code as ScanCode;
pub use keyboard_types::Key as KeyCode;
pub use keyboard_types::KeyState;
pub use keyboard_types::KeyboardEvent;
pub use keyboard_types::Location as KeyLocation;
pub use keyboard_types::Modifiers as KeyModifiers;

mod backend;
mod background_thread;
mod features;
mod image_info;
mod oneshot;
mod event;

pub use backend::*;
pub use features::*;
pub use image_info::*;
pub use event::*;

/// Error that can occur while waiting for a key press.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WaitKeyError {
	/// The window is closed.
	///
	/// No further key events will happen,
	/// and any loop waiting for keys should terminate.
	WindowClosed,
}

/// Allows a type to be displayed as an image.
///
/// This trait is implemented for tuples of `(Data, ImageInfo)` if `Data` can be converted into a `Box<[u8]>`,
/// and for `&(Data, ImageInfo)` if `Data` is `AsRef<[u8]>`.
/// Amongst others, that includes `&[u8]`, `Box<[u8]>`, `Vec<u8>`.
///
/// Implementations for types from third-party libraries can be enabled using feature flags.
pub trait ImageData {
	/// Get the image data as boxed slice.
	///
	/// This function takes self by value to prevent copying if possible.
	/// If the data can not be moved into a box, consider implementing the trait for references.
	fn data(self) -> Box<[u8]>;

	/// Get the [`ImageInfo`] describing the binary data.
	///
	/// This function may fail at runtime if the data can not be described properly.
	fn info(&self) -> Result<ImageInfo, String>;
}

/// A rectangle.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Rectangle {
	x: i32,
	y: i32,
	width: u32,
	height: u32
}

impl Rectangle {
	pub fn from_xywh(x: i32, y: i32, width: u32, height: u32) -> Self {
		Self { x, y, width, height }
	}

	pub fn x(&self) -> i32 {
		self.x
	}

	pub fn y(&self) -> i32 {
		self.y
	}

	pub fn width(&self) -> u32 {
		self.width
	}

	pub fn height(&self) -> u32 {
		self.height
	}
}

/// Options for creating a window.
#[derive(Clone, Debug, Eq, PartialEq)]
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

	/// Preserve the aspect ratio
	pub preserve_aspect_ratio: bool,
}

impl std::error::Error for WaitKeyError {}

impl std::fmt::Display for WaitKeyError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			WaitKeyError::WindowClosed => write!(f, "window closed"),
		}
	}
}

impl Default for WindowOptions {
	fn default() -> Self {
		Self {
			name: String::from("image"),
			size: [800, 600],
			resizable: true,
			preserve_aspect_ratio: true,
		}
	}
}

impl WindowOptions {
	/// Set the name of the window.
	///
	/// This function consumed and returns `self` to allow daisy chaining.
	pub fn set_name(mut self, name: String) -> Self {
		self.name = name;
		self
	}

	/// Set the initial size of the window.
	///
	/// This property may be ignored by a window manager.
	///
	/// This function consumed and returns `self` to allow daisy chaining.
	pub fn set_size(mut self, size: [u32; 2]) -> Self {
		self.size = size;
		self
	}

	/// Set the initial width of the window.
	///
	/// This property may be ignored by a window manager.
	///
	/// This function consumed and returns `self` to allow daisy chaining.
	pub fn set_width(mut self, width: u32) -> Self {
		self.size[0] = width;
		self
	}

	/// Set the initial height of the window.
	///
	/// This property may be ignored by a window manager.
	///
	/// This function consumed and returns `self` to allow daisy chaining.
	pub fn set_height(mut self, height: u32) -> Self {
		self.size[1] = height;
		self
	}

	/// Make the window resizable or not.
	///
	/// This property may be ignored by a window manager.
	///
	/// This function consumed and returns `self` to allow daisy chaining.
	pub fn set_resizable(mut self, resizable: bool) -> Self {
		self.resizable = resizable;
		self
	}

	/// Preserve the aspect ratio of displayed images, or not.
	///
	/// This function consumed and returns `self` to allow daisy chaining.
	pub fn set_preserve_aspect_ratio(mut self, preserve_aspect_ratio: bool) -> Self {
		self.preserve_aspect_ratio = preserve_aspect_ratio;
		self
	}
}

/// Save an image to the given path.
#[cfg(feature = "save")]
pub fn save_image(path: &std::path::Path, data: &[u8], info: ImageInfo) -> Result<(), String> {
	let color_type = match info.pixel_format {
		PixelFormat::Mono8 => image::ColorType::L8,
		PixelFormat::Rgb8  => image::ColorType::Rgb8,
		PixelFormat::Rgba8 => image::ColorType::Rgba8,
		PixelFormat::Bgr8  => image::ColorType::Bgr8,
		PixelFormat::Bgra8 => image::ColorType::Bgra8,
	};

	let bytes_per_pixel = usize::from(info.pixel_format.bytes_per_pixel());

	if info.row_stride == info.width * bytes_per_pixel {
		image::save_buffer(path, data, info.width as u32, info.height as u32, color_type)
			.map_err(|e| format!("failed to save image: {}", e))
	} else {
		let mut packed = Vec::with_capacity(info.width * info.height * bytes_per_pixel);
		for row in 0..info.height {
			packed.extend_from_slice(&data[info.row_stride * row..][..info.width * bytes_per_pixel]);
		}
		image::save_buffer(path, &packed, info.width as u32, info.height as u32, color_type)
			.map_err(|e| format!("failed to save image: {}", e))
	}
}

/// Prompt the user to save an image.
///
/// The name hint is used as initial path for the prompt.
#[cfg(feature = "save")]
pub fn prompt_save_image(name_hint: &str, data: &[u8], info: ImageInfo) -> Result<(), String> {
	let path = match tinyfiledialogs::save_file_dialog("Save image", name_hint) {
		Some(x) => x,
		None => return Ok(()),
	};

	save_image(path.as_ref(), &data, info)
}

impl<Container> ImageData for (Container, ImageInfo)
where
	Box<[u8]>: From<Container>,
{
	fn data(self) -> Box<[u8]> {
		Box::from(self.0)
	}

	fn info(&self) -> Result<ImageInfo, String> {
		Ok(self.1.clone())
	}
}

impl<Container> ImageData for (Container, &ImageInfo)
where
	Box<[u8]>: From<Container>,
{
	fn data(self) -> Box<[u8]> {
		Box::from(self.0)
	}

	fn info(&self) -> Result<ImageInfo, String> {
		Ok(self.1.clone())
	}
}

impl<Container> ImageData for &(Container, ImageInfo)
where
	Container: AsRef<[u8]>,
{
	fn data(self) -> Box<[u8]> {
		Box::from(self.0.as_ref())
	}

	fn info(&self) -> Result<ImageInfo, String> {
		Ok(self.1.clone())
	}
}

impl<Container> ImageData for &(Container, &ImageInfo)
where
	Container: AsRef<[u8]>,
{
	fn data(self) -> Box<[u8]> {
		Box::from(self.0.as_ref())
	}

	fn info(&self) -> Result<ImageInfo, String> {
		Ok(self.1.clone())
	}
}
