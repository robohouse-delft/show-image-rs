//! Support for the [`tch`][::tch] crate.
//!
//! This module adds support for displaying [`tch::Tensor`] as images.
//! The main interface is provided by an extension trait [`TensorAsImage`],
//! which allows you to wrap a tensor in a [`TensorImage`].
//! The wrapper struct adds some required meta-data for interpreting the tensor data as an image.
//!
//! The meta-data has to be supplied by the user, or it can be guessed automatically based on the tensor shape.
//! When guessing, you do need to specify if you want to interpret multi-channel tensors as RGB or BGR.
//! An extension trait [`TensorAsImage`] is provided to construct the wrapper with the proper meta-data.
//!
//! It is not always possible to interpret a tensor as the requested image format,
//! so all function in the extension trait return a [`Result`].
//! The [`Into<Image>`] trait is implemented for [`TensorImage`] and for [`Result`]`<`[`TensorImage`]`, `[`ImageDataError`]`>`,
//! so you can directly pass use the result to so set the image of a window directly.
//!
//! Both planar and interlaced tensors are supported.
//! If you specify the format manually, you must also specify if the tensor contains interlaced or planar data.
//! If you let the library guess, it will try to deduce it automatically based on the tensor shape.
//!
//! # Example
//! ```no_run
//! use show_image::{create_window, WindowOptions};
//! use show_image::tch::TensorAsImage;
//!
//! let tensor = tch::vision::imagenet::load_image("/path/to/image.png").unwrap();
//! let window = create_window("image", WindowOptions::default())?;
//! window.set_image("image-001", tensor.as_image_guess_rgb())?;
//! # Result::<(), Box<dyn std::error::Error>>::Ok(())
//! ```

use crate::error::ImageDataError;
use crate::Alpha;
use crate::BoxImage;
use crate::Image;
use crate::ImageInfo;
use crate::PixelFormat;

/// Wrapper for [`tch::Tensor`] that implements `Into<Image>`.
pub struct TensorImage<'a> {
	tensor: &'a tch::Tensor,
	info: ImageInfo,
	planar: bool,
}

/// The pixel format of a tensor, or a color format to guess the pixel format.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TensorPixelFormat {
	/// The tensor has planar pixel data.
	Planar(PixelFormat),

	/// The tensor has interlaced pixel data.
	Interlaced(PixelFormat),

	/// The library should guess if the pixel data is planar or interlaced.
	Guess(ColorFormat),
}

/// A preferred color format for guessing the pixel format of a tensor.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ColorFormat {
	/// Interpret 3 or 4 channel tensors as RGB or RGBA.
	Rgb,

	/// Interpret 3 or 4 channel tensors as BGR or BGRA.
	Bgr,
}

/// Extension trait to allow displaying tensors as image.
///
/// The tensor data will always be copied.
/// Additionally, the data will be converted to 8 bit integers,
/// and planar data will be converted to interlaced data.
///
/// The original tensor is unaffected, but the conversion can be expensive.
/// If you also need to convert the tensor, consider doing so before displaying it.
#[allow(clippy::needless_lifetimes)]
pub trait TensorAsImage {
	/// Wrap the tensor in a [`TensorImage`] that implements `Into<Image>`.
	///
	/// This function requires you to specify the pixel format of the tensor,
	/// or a preferred color format to have the library guess based on the tensor shape.
	///
	/// See the other functions in the trait for easier shorthands.
	fn as_image<'a>(&'a self, pixel_format: TensorPixelFormat) -> Result<TensorImage<'a>, ImageDataError>;

	/// Wrap the tensor with a known pixel format in a [`TensorImage`], assuming it holds interlaced pixel data.
	fn as_interlaced<'a>(&'a self, pixel_format: PixelFormat) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_image(TensorPixelFormat::Interlaced(pixel_format))
	}

	/// Wrap the tensor with a known pixel format in a [`TensorImage`], assuming it holds planar pixel data.
	fn as_planar<'a>(&'a self, pixel_format: PixelFormat) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_image(TensorPixelFormat::Planar(pixel_format))
	}

	/// Wrap the tensor in a [`TensorImage`].
	///
	/// The pixel format of the tensor will be guessed based on the shape.
	/// The `color_format` argument determines if tensors with 3 or 4 channels are interpreted as RGB or BGR.
	fn as_image_guess<'a>(&'a self, color_format: ColorFormat) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_image(TensorPixelFormat::Guess(color_format))
	}

	/// Wrap the tensor in a [`TensorImage`].
	///
	/// The pixel format of the tensor will be guessed based on the shape.
	/// Tensors with 3 or 4 channels will be interpreted as RGB.
	fn as_image_guess_rgb<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_image_guess(ColorFormat::Rgb)
	}

	/// Wrap the tensor in a [`TensorImage`].
	///
	/// The pixel format of the tensor will be guessed based on the shape.
	/// Tensors with 3 or 4 channels will be interpreted as BGR.
	fn as_image_guess_bgr<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_image_guess(ColorFormat::Bgr)
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds monochrome data.
	fn as_mono8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_interlaced(PixelFormat::Mono8)
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds interlaced RGB data.
	fn as_interlaced_rgb8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_interlaced(PixelFormat::Rgb8)
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds interlaced RGBA data.
	fn as_interlaced_rgba8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_interlaced(PixelFormat::Rgba8(Alpha::Unpremultiplied))
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds interlaced BGR data.
	fn as_interlaced_bgr8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_interlaced(PixelFormat::Bgr8)
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds interlaced BGRA data.
	fn as_interlaced_bgra8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_interlaced(PixelFormat::Bgra8(Alpha::Unpremultiplied))
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds planar RGB data.
	fn as_planar_rgb8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_planar(PixelFormat::Rgb8)
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds planar RGBA data.
	fn as_planar_rgba8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_planar(PixelFormat::Rgba8(Alpha::Unpremultiplied))
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds planar BGR data.
	fn as_planar_bgr8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_planar(PixelFormat::Bgr8)
	}

	/// Wrap the tensor in a [`TensorImage`], assuming it holds planar BGRA data.
	fn as_planar_bgra8<'a>(&'a self) -> Result<TensorImage<'a>, ImageDataError> {
		self.as_planar(PixelFormat::Bgra8(Alpha::Unpremultiplied))
	}
}

impl TensorAsImage for tch::Tensor {
	fn as_image(&self, pixel_format: TensorPixelFormat) -> Result<TensorImage, ImageDataError> {
		let (planar, info) = match pixel_format {
			TensorPixelFormat::Planar(pixel_format) => (true, tensor_info(self, pixel_format, true)?),
			TensorPixelFormat::Interlaced(pixel_format) => (false, tensor_info(self, pixel_format, false)?),
			TensorPixelFormat::Guess(color_format) => guess_tensor_info(self, color_format)?,
		};
		Ok(TensorImage {
			tensor: self,
			info,
			planar,
		})
	}
}

impl<'a> From<TensorImage<'a>> for Image {
	fn from(other: TensorImage<'a>) -> Self {
		let data: Vec<u8> = match other.planar {
			true => other.tensor.permute(&[1, 2, 0]).into(),
			false => other.tensor.into(),
		};

		BoxImage::new(other.info, data.into_boxed_slice()).into()
	}
}

impl<'a> From<Result<TensorImage<'a>, ImageDataError>> for Image {
	fn from(other: Result<TensorImage<'a>, ImageDataError>) -> Self {
		match other {
			Ok(x) => x.into(),
			Err(e) => Image::Invalid(e),
		}
	}
}

/// Compute the image info of a tensor, given a known pixel format.
#[allow(clippy::branches_sharing_code)] // Stop lying, clippy.
fn tensor_info(tensor: &tch::Tensor, pixel_format: PixelFormat, planar: bool) -> Result<ImageInfo, String> {
	let expected_channels = pixel_format.channels();
	let dimensions = tensor.dim();

	if dimensions == 3 {
		let shape = tensor.size3().unwrap();
		if planar {
			let (channels, height, width) = shape;
			if channels != i64::from(expected_channels) {
				Err(format!("expected shape ({}, height, width), found {:?}", expected_channels, shape))
			} else {
				Ok(ImageInfo::new(pixel_format, width as u32, height as u32))
			}
		} else {
			let (height, width, channels) = shape;
			if channels != i64::from(expected_channels) {
				Err(format!("expected shape (height, width, {}), found {:?}", expected_channels, shape))
			} else {
				Ok(ImageInfo::new(pixel_format, width as u32, height as u32))
			}
		}
	} else if dimensions == 2 && expected_channels == 1 {
		let (height, width) = tensor.size2().unwrap();
		Ok(ImageInfo::new(pixel_format, width as u32, height as u32))
	} else {
		Err(format!(
			"wrong number of dimensions ({}) for format ({:?})",
			dimensions, pixel_format
		))
	}
}

/// Guess the image info of a tensor.
fn guess_tensor_info(tensor: &tch::Tensor, color_format: ColorFormat) -> Result<(bool, ImageInfo), String> {
	let dimensions = tensor.dim();

	if dimensions == 2 {
		let (height, width) = tensor.size2().unwrap();
		Ok((false, ImageInfo::mono8(width as u32, height as u32)))
	} else if dimensions == 3 {
		let shape = tensor.size3().unwrap();
		match (shape.0 as u32, shape.1 as u32, shape.2 as u32, color_format) {
			(h, w, 1, _) => Ok((false, ImageInfo::mono8(w, h))),
			(1, h, w, _) => Ok((false, ImageInfo::mono8(w, h))), // "planar" doesn't do anything here, so call it interlaced
			(h, w, 3, ColorFormat::Rgb) => Ok((false, ImageInfo::rgb8(w, h))),
			(h, w, 3, ColorFormat::Bgr) => Ok((false, ImageInfo::bgr8(w, h))),
			(3, h, w, ColorFormat::Rgb) => Ok((true, ImageInfo::rgb8(w, h))),
			(3, h, w, ColorFormat::Bgr) => Ok((true, ImageInfo::bgr8(w, h))),
			(h, w, 4, ColorFormat::Rgb) => Ok((false, ImageInfo::rgba8(w, h))),
			(h, w, 4, ColorFormat::Bgr) => Ok((false, ImageInfo::bgra8(w, h))),
			(4, h, w, ColorFormat::Rgb) => Ok((true, ImageInfo::rgba8(w, h))),
			(4, h, w, ColorFormat::Bgr) => Ok((true, ImageInfo::bgra8(w, h))),
			_ => Err(format!("unable to guess pixel format for tensor with shape {:?}, expected (height, width) or (height, width, channels) or (channels, height, width) where channels is either 1, 3 or 4", shape))
		}
	} else {
		Err(format!(
			"unable to guess pixel format for tensor with {} dimensions, expected 2 or 3 dimensions",
			dimensions
		))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn guess_tensor_info() {
		let data = tch::Tensor::of_slice(&(0..120).collect::<Vec<u8>>());

		// Guess monochrome from compatible data.
		assert!(data.reshape(&[12, 10, 1]).as_image_guess_bgr().map(|x| x.info) == Ok(ImageInfo::mono8(10, 12)));
		assert!(data.reshape(&[1, 12, 10]).as_image_guess_bgr().map(|x| x.info) == Ok(ImageInfo::mono8(10, 12)));
		assert!(data.reshape(&[12, 10]).as_image_guess_bgr().map(|x| x.info) == Ok(ImageInfo::mono8(10, 12)));

		// Guess RGB[A]/BGR[A] from interlaced data.
		assert!(data.reshape(&[8, 5, 3]).as_image_guess_rgb().map(|x| x.info) == Ok(ImageInfo::rgb8(5, 8)));
		assert!(data.reshape(&[8, 5, 3]).as_image_guess_bgr().map(|x| x.info) == Ok(ImageInfo::bgr8(5, 8)));
		assert!(data.reshape(&[5, 6, 4]).as_image_guess_rgb().map(|x| x.info) == Ok(ImageInfo::rgba8(6, 5)));
		assert!(data.reshape(&[5, 6, 4]).as_image_guess_bgr().map(|x| x.info) == Ok(ImageInfo::bgra8(6, 5)));

		// Guess RGB[A]/BGR[A] from planar data.
		assert!(data.reshape(&[3, 8, 5]).as_image_guess_rgb().map(|x| x.info) == Ok(ImageInfo::rgb8(5, 8)));
		assert!(data.reshape(&[3, 8, 5]).as_image_guess_bgr().map(|x| x.info) == Ok(ImageInfo::bgr8(5, 8)));
		assert!(data.reshape(&[4, 5, 6]).as_image_guess_rgb().map(|x| x.info) == Ok(ImageInfo::rgba8(6, 5)));
		assert!(data.reshape(&[4, 5, 6]).as_image_guess_bgr().map(|x| x.info) == Ok(ImageInfo::bgra8(6, 5)));

		// Fail to guess on other dimensions
		assert!(let Err(_) = data.reshape(&[120]).as_image_guess_rgb().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[2, 10, 6]).as_image_guess_rgb().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[6, 10, 2]).as_image_guess_rgb().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[8, 5, 3, 1]).as_image_guess_rgb().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 6, 1]).as_image_guess_rgb().map(|x| x.info));
	}

	#[test]
	fn tensor_info_interlaced_with_known_format() {
		let data = tch::Tensor::of_slice(&(0..60).collect::<Vec<u8>>());

		// Monochrome
		assert!(data.reshape(&[12, 5, 1]).as_mono8().map(|x| x.info) == Ok(ImageInfo::mono8(5, 12)));
		assert!(data.reshape(&[12, 5]).as_mono8().map(|x| x.info) == Ok(ImageInfo::mono8(5, 12)));
		assert!(let Err(_) = data.reshape(&[12, 5, 1, 1]).as_mono8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[6, 5, 2]).as_mono8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[3, 5, 4]).as_mono8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 3]).as_mono8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[60]).as_mono8().map(|x| x.info));

		// RGB/BGR
		assert!(data.reshape(&[4, 5, 3]).as_interlaced_rgb8().map(|x| x.info) == Ok(ImageInfo::rgb8(5, 4)));
		assert!(data.reshape(&[4, 5, 3]).as_interlaced_bgr8().map(|x| x.info) == Ok(ImageInfo::bgr8(5, 4)));
		assert!(let Err(_) = data.reshape(&[4, 5, 3, 1]).as_interlaced_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 3, 1]).as_interlaced_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[3, 5, 4]).as_interlaced_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[3, 5, 4]).as_interlaced_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_interlaced_rgb8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_interlaced_rgb8().map(|x| x.info));

		// RGBA/BGRA
		assert!(data.reshape(&[3, 5, 4]).as_interlaced_rgba8().map(|x| x.info) == Ok(ImageInfo::rgba8(5, 3)));
		assert!(data.reshape(&[3, 5, 4]).as_interlaced_bgra8().map(|x| x.info) == Ok(ImageInfo::bgra8(5, 3)));
		assert!(let Err(_) = data.reshape(&[3, 5, 4, 1]).as_interlaced_rgba8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[3, 5, 4, 1]).as_interlaced_bgra8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 3]).as_interlaced_rgba8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 3]).as_interlaced_bgra8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_interlaced_rgba8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_interlaced_bgra8().map(|x| x.info));
	}

	#[test]
	fn tensor_info_planar_with_known_format() {
		let data = tch::Tensor::of_slice(&(0..60).collect::<Vec<u8>>());

		// RGB/BGR
		assert!(data.reshape(&[3, 4, 5]).as_planar_rgb8().map(|x| x.info) == Ok(ImageInfo::rgb8(5, 4)));
		assert!(data.reshape(&[3, 4, 5]).as_planar_bgr8().map(|x| x.info) == Ok(ImageInfo::bgr8(5, 4)));
		assert!(let Err(_) = data.reshape(&[4, 5, 3, 1]).as_planar_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 3, 1]).as_planar_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 3]).as_planar_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[4, 5, 3]).as_planar_bgr8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_planar_rgb8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_planar_rgb8().map(|x| x.info));

		// RGBA/BGRA
		assert!(data.reshape(&[4, 3, 5]).as_planar_rgba8().map(|x| x.info) == Ok(ImageInfo::rgba8(5, 3)));
		assert!(data.reshape(&[4, 3, 5]).as_planar_bgra8().map(|x| x.info) == Ok(ImageInfo::bgra8(5, 3)));
		assert!(let Err(_) = data.reshape(&[3, 5, 4, 1]).as_planar_rgba8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[3, 5, 4, 1]).as_planar_bgra8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[3, 5, 4]).as_planar_rgba8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[3, 5, 4]).as_planar_bgra8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_planar_rgba8().map(|x| x.info));
		assert!(let Err(_) = data.reshape(&[15, 4]).as_planar_bgra8().map(|x| x.info));
	}
}
