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
//! window.set_image("image-001", image)?;
//!
//! # Result::<(), String>::Ok(())
//! ```
//!
//! # Example 2: Handling keyboard events.
//! ```no_run
//! # use std::time::Duration;
//! # use show_image::ImageInfo;
//! use show_image::{Event, KeyCode, make_window};
//!
//! // Create a window and display the image.
//! # let image = (&[0u8][..], ImageInfo::rgb8(1920, 1080));
//! let window = make_window("image")?;
//! window.set_image("image-001", &image)?;
//!
//! // Print keyboard events until Escape is pressed, then exit.
//! // If the user closes the window, the channel is closed and the loop also exits.
//! for event in window.events()? {
//!     if let Event::KeyboardEvent(event) = event {
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

mod backend;
mod error;
mod event_handler;
mod features;
mod image;
mod image_info;
mod oneshot;

pub use self::error::*;
pub use self::backend::*;
pub use self::features::*;
pub use self::image::*;
pub use self::image_info::*;
pub use self::event_handler::*;

pub use wgpu::Color;
pub use winit;
pub use winit::event;
pub use winit::window::WindowId;

#[cfg(feature = "macros")]
pub use show_image_macros::main;

/// The event type that can be handled by event handlers.
///
/// Note that the user event from [`winit::event::Event`] is used internally.
/// User event handlers will never see a `Event::UserEvent`.
///
/// When the `never` type is stabalized, this type alias will change to [`winit::event::Event<!>`].
/// Do not worry, the library will receive a semver bump when that happens.
pub type Event<'a> = winit::event::Event<'a, AllWindowsClosed>;

/// Save an image to the given path.
#[cfg(feature = "save")]
pub fn save_image(path: &std::path::Path, data: &[u8], info: ImageInfo) -> Result<(), String> {
	let color_type = match info.pixel_format {
		PixelFormat::Mono8 => ::image::ColorType::L8,
		PixelFormat::Rgb8  => ::image::ColorType::Rgb8,
		PixelFormat::Rgba8 => ::image::ColorType::Rgba8,
		PixelFormat::Bgr8  => ::image::ColorType::Bgr8,
		PixelFormat::Bgra8 => ::image::ColorType::Bgra8,
	};

	let bytes_per_pixel = u32::from(info.pixel_format.bytes_per_pixel());

	if info.stride_x == info.width * bytes_per_pixel && info.stride_y == bytes_per_pixel {
		::image::save_buffer(path, data, info.width, info.height, color_type)
			.map_err(|e| format!("failed to save image: {}", e))

	} else {
		let bytes_per_pixel = bytes_per_pixel as usize;
		let stride_x = info.stride_x as usize;
		let stride_y = info.stride_y as usize;
		let width = info.width as usize;
		let height = info.height as usize;

		let mut packed = Vec::with_capacity(width * height * bytes_per_pixel);
		if stride_y == bytes_per_pixel {
			for row in 0..height {
				packed.extend_from_slice(&data[stride_x * row..][..width * bytes_per_pixel]);
			}
		} else if stride_x > stride_y {
			for x in 0..width {
				for y in 0..height {
					packed.extend_from_slice(&data[stride_x * x + stride_y * y..][..bytes_per_pixel])
				}
			}
		} else {
			for y in 0..height {
				for x in 0..width {
					packed.extend_from_slice(&data[stride_x * x + stride_y * y..][..bytes_per_pixel])
				}
			}
		}

		::image::save_buffer(path, &packed, info.width as u32, info.height as u32, color_type)
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
