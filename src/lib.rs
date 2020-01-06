//! `show-image` is a library for quickly displaying images.
//! It is intended as a debugging aid for writing image processing code.
//! The library is not inteded to be used for writing full-featured GUIs,
//! although you can process keyboard events from the created windows.
//!
//! It is the goal of the library to support as many different data types used to represent images.
//! To prevent dependency bloat and unreasonable compile times, feature flags can be used to enable support for third party libraries.
//! If you think support for a specific data type is missing, feel free to send a PR or create an issue on GitHub.
//!
//! # Global context or manually created context.
//! The library uses a [`Context`] object to manage an event loop running in a background thread.
//! You can manually create such a context, or you can use the global functions [`make_window`] and [`make_window_full`].
//! These free functions will use a global context that is initialized when needed.
//!
//! Only one [`Context`] object can ever be created, so you can not mix the free functions with a manually created context.
//!
//! # Keyboard events.
//! You can handle keyboard events for windows.
//! You can use [`Window::wait_key`] or [`Window::wait_key_deadline`] to wait for key press events.
//! Alternatively you can use [`Window::events`] to get direct access to the channel where all keyboard events are sent (including key release events).
//!
//! Keyboard events are reported using types re-exported from the `keyboard-types` crate for easy interoperability with other crates.
//!
//!
//! # Example 1: Using the global context.
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
//! window.set_image(image)?;
//!
//! # Result::<(), String>::Ok(())
//! ```
//!
//! # Example 2: Using a manually created context.
//!
//! Alternatively, you can manually create a [`Context`] and use that to create a window.
//! This avoids using global state, but since you can only create one context,
//! you will have to pass the context everywhere in your code.
//!
//! ```no_run
//! use show_image::Context;
//! # use show_image::ImageInfo;
//!
//! # let image = (&[0u8][..], ImageInfo::rgb8(1920, 1080));
//! let context = Context::new()?;
//! let window = context.make_window("image")?;
//! window.set_image(&image)?;
//! # Result::<(), String>::Ok(())
//! ```
//!
//! # Example 3: Handling keyboard events.
//! ```no_run
//! # use std::time::Duration;
//! # use show_image::ImageInfo;
//! use show_image::{KeyCode, make_window};
//!
//! # let image = (&[0u8][..], ImageInfo::rgb8(1920, 1080));
//! #
//! // Create a window and display the image.
//! let window = make_window("image")?;
//! window.set_image(&image)?;
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
//! # Result::<(), String>::Ok(())
//! ```

pub use keyboard_types::Code as ScanCode;
pub use keyboard_types::Key as KeyCode;
pub use keyboard_types::KeyState;
pub use keyboard_types::KeyboardEvent;
pub use keyboard_types::Location as KeyLocation;
pub use keyboard_types::Modifiers as KeyModifiers;

mod backend;
mod features;
mod image_info;
mod oneshot;

pub use backend::*;
pub use features::*;
pub use image_info::*;

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
/// Amonst others, that includes `&[u8]`, `Box<[u8]>`, `Vec<u8>`.
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

	/// Preserve the aspact ratio
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

	/// Make the window resiable or not.
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
