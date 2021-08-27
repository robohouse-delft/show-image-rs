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
//!   * [`tch::Tensor`][::tch::Tensor] (requires the `"tch"` feature).
//!   * [`raqote::DrawTarget`][::raqote::DrawTarget] and [`raqote::Image`][::raqote::Image] (requires the `"raqote"` feature).
//!
//! If you think support for a some data type is missing,
//! feel free to send a PR or create an issue on GitHub.
//!
//! # Global context and threading
//! The library uses a global context that runs an event loop.
//! This context must be initialized before any `show-image` functions can be used.
//! Additionally, some platforms require the event loop to be run in the main thread.
//! To ensure portability, the same restriction is enforced on all platforms.
//!
//! The easiest way to initialize the global context and run the event loop in the main thread
//! is to use the [`main`] attribute macro on your main function.
//! If you want to run some code in the main thread before the global context takes over,
//! you can use the [`run_context()`] function or one of it's variations instead.
//! Note that you must still call those functions from the main thread,
//! and they do not return control back to the caller.
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
//! If the `save` feature is enabled, windows allow the displayed image to be saved using `Ctrl+S` or `Ctrl+Shift+S`.
//! The first shortcut will open a file dialog to save the currently displayed image.
//! The second shortcut will directly save the image in the current working directory using the name of the image.
//!
//! The image is saved without any overlays.
//! To save an image including overlays, add `Alt` to the shortcut: `Ctrl+Alt+S` and `Ctrl+Alt+Shift+S`.
//!
//! Note that images are saved in a background thread.
//! To ensure that no data loss occurs, call [`exit()`] to terminate the process rather than [`std::process::exit()`].
//! That will ensure that the background threads are joined before the process is terminated.
//!
//! # Example 1: Showing an image.
//! ```no_run
//! # use image;
//! use show_image::{ImageView, ImageInfo, create_window};
//!
//! #[show_image::main]
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//! # let pixel_data = &[0u8][..];
//!   let image = ImageView::new(ImageInfo::rgb8(1920, 1080), pixel_data);
//!
//!   // Create a window with default options and display the image.
//!   let window = create_window("image", Default::default())?;
//!   window.set_image("image-001", image)?;
//!
//!   Ok(())
//! }
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
//!
//! # Back-end and GPU selection
//!
//! This crate uses [`wgpu`] for rendering.
//! You can force the selection of a specfic WGPU backend by setting the `WGPU_BACKEND` environment variable to one of the supported values:
//!
//! * `primary`: Use the primary backend for the platform (the default).
//! * `vulkan`: Use the vulkan back-end.
//! * `metal`: Use the metal back-end.
//! * `dx12`: Use the DirectX 12 back-end.
//! * `dx11`: Use the DirectX 11 back-end.
//! * `gl`: Use the OpenGL back-end.
//! * `webgpu`: Use the browser WebGPU back-end.
//!
//! You can also influence the GPU selection by setting the `WGPU_POWER_PREF` environment variable:
//!
//! * `low`: Prefer a low power GPU (the default).
//! * `high`: Prefer a high performance GPU.

#![cfg_attr(feature = "nightly", feature(doc_cfg))]
#![cfg_attr(feature = "nightly", feature(termination_trait_lib))]
#![warn(missing_docs)]

mod backend;
mod background_thread;
pub mod error;
pub mod event;
mod features;
mod image;
mod image_info;
mod oneshot;
mod rectangle;

pub use self::backend::*;
pub use self::features::*;
pub use self::image::*;
pub use self::image_info::*;
pub use self::rectangle::Rectangle;

pub use winit;
pub use winit::window::WindowId;

pub use glam;

/// An RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Color {
	/// The red component in the range 0 to 1.
	pub red: f64,

	/// The green component in the range 0 to 1.
	pub green: f64,

	/// The blue component in the range 0 to 1.
	pub blue: f64,

	/// The alpha component in the range 0 to 1.
	pub alpha: f64,
}

impl Color {
	/// Create a new fully opaque color from the RGB components.
	pub const fn rgb(red: f64, green: f64, blue: f64) -> Self {
		Self::rgba(red, green, blue, 1.0)
	}

	/// Create a new color from the RGBA components.
	pub const fn rgba(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
		Self { red, green, blue, alpha }
	}

	/// Get a color representing fully opaque black.
	pub const fn black() -> Self {
		Self::rgb(0.0, 0.0, 0.0)
	}

	/// Get a color representing fully opaque white.
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
	size: glam::UVec2,
	row_stride: u32,
) -> Result<(), error::SaveImageError> {
	let path = path.as_ref();

	let file = std::fs::File::create(path)?;

	let mut encoder = png::Encoder::new(file, size.x, size.y);
	encoder.set_color(png::ColorType::Rgba);
	encoder.set_depth(png::BitDepth::Eight);

	let mut writer = encoder.write_header()?;

	if row_stride == size.x * 4 {
		Ok(writer.write_image_data(data)?)
	} else {
		use std::io::Write;

		let mut writer = writer.into_stream_writer()?;
		for row in data.chunks(row_stride as usize) {
			let row = &row[..size.x as usize * 4];
			writer.write_all(row)?;
		}
		writer.finish()?;
		Ok(())
	}
}
