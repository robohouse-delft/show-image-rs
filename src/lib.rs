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
//!   * The [`Image`] and [`ImageView`] types from this crate.
//!   * [`image::DynamicImage`][::image::DynamicImage] and [`image::ImageBuffer`][::image::ImageBuffer] (requires the `"image"` feature).
//!   * [`tch::Tensor`](::tch::Tensor) (requires the `"tch"` feature).
//!   * [`raqote::DrawTarget`][::raqote::DrawTarget] and [`raqote::Image`][::raqote::Image] (requires the `"raqote"` feature).
//!
//! If you think support for a some data type is missing,
//! feel free to send a PR or create an issue on GitHub.
//!
//! # Event handling.
//! You can register an event handler to run in the global context thread using [`WindowProxy::add_event_handler()`] or some of the similar functions.
//! You can also register an event handler directly with the context to handle global events (including all window events).
//! Since these event handlers run in the event loop, they should not block for any significant time.
//!
//! You can also receive events using [`WindowProxy::event_channel()`] or [`ContextProxy::event_channel()`].
//! These functions create a new channel for receiving window events or global events, respectively.
//! As long as you're receiving the events in your own thread, you can block as long as you like.
//!
//! # Saving displayed images.
//! If the `save` feature is enabled, windows allow the displayed image to be saved using `Ctrl+S`.
//! This will open a file dialog to save the currently displayed image.
//!
//! Note that images are saved in a background thread.
//! To ensure that no data loss occurs, call [`stop`] to gracefully stop and join the background thread.
//!
//! # Example 1: Showing an image.
//! ```no_run
//! # use image;
//! # let pixel_data = &[0u8][..];
//! use show_image::{ImageView, ImageInfo, create_window};
//!
//! let image = ImageView::new(ImageInfo::rgb8(1920, 1080), pixel_data);
//!
//! // Create a window with default options and display the image.
//! let window = create_window("image", Default::default())?;
//! window.set_image("image-001", image)?;
//!
//! # Result::<(), Box<dyn std::error::Error>>::Ok(())
//! ```
//!
//! # Example 2: Handling keyboard events using an event channel.
//! ```no_run
//! # use show_image::{ImageInfo, ImageView};
//! use show_image::{event, create_window};
//!
//! // Create a window and display the image.
//! # let image = ImageView::new(ImageInfo::rgb8(1920, 1080), &[0u8][..]);
//! let window = create_window("image", Default::default())?;
//! window.set_image("image-001", &image)?;
//!
//! // Print keyboard events until Escape is pressed, then exit.
//! // If the user closes the window, the channel is closed and the loop also exits.
//! for event in window.event_channel()? {
//!   if let event::WindowEvent::KeyboardInput(event) = event {
//!         println!("{:#?}", event);
//!         if event.input.key_code == Some(event::VirtualKeyCode::Escape) && event.input.state.is_pressed() {
//!             break;
//!         }
//!     }
//! }
//!
//! # Result::<(), Box<dyn std::error::Error>>::Ok(())
//! ```

#![cfg_attr(feature = "nightly", feature(doc_cfg))]

mod backend;
mod background_thread;
pub mod error;
pub mod event;
mod features;
mod image;
mod image_info;
mod oneshot;

pub use self::backend::*;
pub use self::features::*;
pub use self::image::*;
pub use self::image_info::*;

pub use winit;
pub use winit::window::WindowId;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Color {
	pub red: f64,
	pub green: f64,
	pub blue: f64,
	pub alpha: f64,
}

impl Color {
	pub const fn rgb(red: f64, green: f64, blue: f64) -> Self {
		Self::rgba(red, green, blue, 1.0)
	}

	pub const fn rgba(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
		Self { red, green, blue, alpha }
	}

	pub const fn black() -> Self {
		Self::rgb(0.0, 0.0, 0.0)
	}

	pub const fn white() -> Self {
		Self::rgb(1.0, 1.0, 1.0)
	}
}

pub mod termination;

#[cfg(feature = "macros")]
pub use show_image_macros::main;

/// Save an image to the given path.
#[cfg(feature = "save")]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "save")))]
fn save_rgba8_image(
	path: impl AsRef<std::path::Path>,
	data: &[u8],
	width: u32,
	height: u32,
	row_stride: u32,
) -> Result<(), String> {
	let path = path.as_ref();

	let file = std::fs::File::create(path)
		.map_err(|e| format!("failed to create {}: {}", path.display(), e))?;
	let file = std::io::BufWriter::new(file);

	let mut encoder = png::Encoder::new(file, width, height);
	encoder.set_color(png::ColorType::RGBA);
	encoder.set_depth(png::BitDepth::Eight);

	let mut writer = encoder.write_header()
		.map_err(|e| format!("failed to write PNG header: {}", e))?;

	if row_stride == width * 4 {
		writer.write_image_data(data).map_err(|e| format!("failed to write image data: {}", e))
	} else {
		use std::io::Write;

		let mut writer = writer.into_stream_writer();
		for row in data.chunks(row_stride as usize) {
			let row = &row[..width as usize * 4];
			writer.write_all(row)
				.map_err(|e| format!("failed to write image data: {}", e))?;
		}
		writer.finish()
			.map_err(|e| format!("failed to write image data: {}", e))?;
		Ok(())
	}
}

/// Prompt the user to save an image.
///
/// The name hint is used as initial path for the prompt.
#[cfg(feature = "save")]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "save")))]
fn prompt_save_rgba8_image(
	name_hint: &str,
	data: &[u8],
	width: u32,
	height: u32,
	row_stride: u32,
) -> Result<(), String> {
	let path = match tinyfiledialogs::save_file_dialog("Save image", name_hint) {
		Some(x) => x,
		None => return Ok(()),
	};

	save_rgba8_image(path, &data, width, height, row_stride)
}
