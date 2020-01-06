use crate::ImageInfo;
use crate::PixelFormat;
use crate::ImageData;

/// Wrapper for [`tch::Tensor`] that implements [`ImageData`].
pub struct TchImage<'a> {
	tensor: &'a tch::Tensor,
	info: ImageInfo,
	planar: bool,
}

/// The pixel format of a tensor, or a color format to guess the pixel format.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TensorPixelFormat {
	Planar(PixelFormat),
	Interlaced(PixelFormat),
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

pub trait TensorAsImage {
	/// Wrap the tensor in a [`TchImage`] that implements [`ImageData`].
	///
	/// This function requires you to specify the pixel format of the tensor,
	/// or a preferred color format to have the library guess based on the tensor shape.
	///
	/// See the other functions in the trait for easier shorthands.
	fn as_image<'a>(&'a self, pixel_format: TensorPixelFormat) -> Result<TchImage<'a>, String>;

	/// Wrap the tensor with a known pixel format in a [`TchImage`], assuming it holds interlaced pixel data.
	fn as_interlaced<'a>(&'a self, pixel_format: PixelFormat) -> Result<TchImage<'a>, String> {
		self.as_image(TensorPixelFormat::Interlaced(pixel_format))
	}

	/// Wrap the tensor with a known pixel format in a [`TchImage`], assuming it holds planaer pixel data.
	fn as_planar<'a>(&'a self, pixel_format: PixelFormat) -> Result<TchImage<'a>, String> {
		self.as_image(TensorPixelFormat::Planar(pixel_format))
	}

	/// Wrap the tensor in a [`TchImage`].
	///
	/// The pixel format of the tensor will be guessed based on the shape.
	/// The `color_format` argument determines if tensors with 3 or 4 channels are interpreted as RGB or BGR.
	fn as_image_guess<'a>(&'a self, color_format: ColorFormat) -> Result<TchImage<'a>, String> {
		self.as_image(TensorPixelFormat::Guess(color_format))
	}

	/// Wrap the tensor in a [`TchImage`].
	///
	/// The pixel format of the tensor will be guessed based on the shape.
	/// Tensors with 3 or 4 channels will be interpreted as RGB.
	fn as_image_guess_rgb<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_image_guess(ColorFormat::Rgb)
	}

	/// Wrap the tensor in a [`TchImage`].
	///
	/// The pixel format of the tensor will be guessed based on the shape.
	/// Tensors with 3 or 4 channels will be interpreted as BGR.
	fn as_image_guess_bgr<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_image_guess(ColorFormat::Bgr)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds monochrome data.
	fn as_mono8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_interlaced(PixelFormat::Mono8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds interlaced RGB data.
	fn as_interlaced_rgb8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_interlaced(PixelFormat::Rgb8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds interlaced RGBA data.
	fn as_interlaced_rgba8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_interlaced(PixelFormat::Rgba8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds interlaced BGR data.
	fn as_interlaced_bgr8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_interlaced(PixelFormat::Bgr8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds interlaced BGRA data.
	fn as_interlaced_bgra8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_interlaced(PixelFormat::Bgra8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds planar RGB data.
	fn as_planar_rgb8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_planar(PixelFormat::Rgb8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds planar RGBA data.
	fn as_planar_rgba8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_planar(PixelFormat::Rgba8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds planar BGR data.
	fn as_planar_bgr8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_planar(PixelFormat::Bgr8)
	}

	/// Wrap the tensor in a [`TchImage`], assuming it holds planar BGRA data.
	fn as_planar_bgra8<'a>(&'a self) -> Result<TchImage<'a>, String> {
		self.as_planar(PixelFormat::Bgra8)
	}
}

impl TensorAsImage for tch::Tensor {
	fn as_image(&self, pixel_format: TensorPixelFormat) -> Result<TchImage, String> {
		let (planar, info) = match pixel_format {
			TensorPixelFormat::Planar(pixel_format)     => tensor_info(self, pixel_format, true)?,
			TensorPixelFormat::Interlaced(pixel_format) => tensor_info(self, pixel_format, false)?,
			TensorPixelFormat::Guess(color_format)      => guess_tensor_info(self, color_format)?,
		};
		Ok(TchImage { tensor: self, info, planar })
	}
}

impl ImageData for TchImage<'_> {
	fn data(self) -> Box<[u8]> {
		if self.planar {
			Vec::<u8>::from(self.tensor.permute(&[1, 2, 0])).into_boxed_slice()
		} else {
			Vec::<u8>::from(self.tensor).into_boxed_slice()
		}
	}

	fn info(&self) -> Result<ImageInfo, String> {
		Ok(self.info.clone())
	}
}

impl ImageData for Result<TchImage<'_>, String> {
	fn data(self) -> Box<[u8]> {
		self.expect("ImageData::data called on an Err variant").data()
	}

	fn info(&self) -> Result<ImageInfo, String> {
		self.as_ref().map_err(|x| x.clone()).and_then(|x| x.info())
	}
}

/// Compute the image info of a tensor, given a known pixel format.
fn tensor_info(tensor: &tch::Tensor, pixel_format: PixelFormat, planar: bool) -> Result<(bool, ImageInfo), String> {
	let expected_channels = pixel_format.channels();
	let dimensions = tensor.dim();

	if dimensions == 3 {
		let shape = tensor.size3().unwrap();
		if planar {
			let (channels, height, width) = shape;
			if channels != i64::from(expected_channels) {
				Err(format!("expected shape ({}, height, width), found {:?}", expected_channels, shape))
			} else {
				Ok((false, ImageInfo::new(pixel_format, width as usize, height as usize)))
			}
		} else {
			let (height, width, channels) = shape;
			if channels != i64::from(expected_channels) {
				Err(format!("expected shape (height, width, {}), found {:?}", expected_channels, shape))
			} else {
				Ok((false, ImageInfo::new(pixel_format, width as usize, height as usize)))
			}
		}
	} else if dimensions == 2 && expected_channels == 1 {
		let (height, width) = tensor.size2().unwrap();
		Ok((false, ImageInfo::new(pixel_format, width as usize, height as usize)))
	} else {
		Err(format!("wrong number of dimensions ({}) for format ({:?})", dimensions, pixel_format))
	}
}

/// Guess the image info of a tensor.
fn guess_tensor_info(tensor: &tch::Tensor, color_format: ColorFormat) -> Result<(bool, ImageInfo), String> {
	let dimensions = tensor.dim();

	if dimensions == 2 {
		let (height, width) = tensor.size2().unwrap();
		Ok((false, ImageInfo::mono8(width as usize, height as usize)))
	} else if dimensions == 3 {
		let shape = tensor.size3().unwrap();
		match (shape.0 as usize, shape.1 as usize, shape.2 as usize, color_format) {
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
		Err(format!("unable to guess pixel format for tensor with {} dimensions, expected 2 or 3 dimensions", dimensions))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn guess_tensor_info() {
		let data = tch::Tensor::of_slice(&(0..120).collect::<Vec<u8>>());

		// Guess monochromo from compatible data.
		assert_eq!(data.reshape(&[12, 10, 1]).as_image_guess_bgr().info(), Ok(ImageInfo::mono8(10, 12)));
		assert_eq!(data.reshape(&[1, 12, 10]).as_image_guess_bgr().info(), Ok(ImageInfo::mono8(10, 12)));
		assert_eq!(data.reshape(&[12, 10]).as_image_guess_bgr().info(), Ok(ImageInfo::mono8(10, 12)));

		// Guess RGB[A]/BGR[A] from interlaced data.
		assert_eq!(data.reshape(&[8, 5, 3]).as_image_guess_rgb().info(), Ok(ImageInfo::rgb8(5, 8)));
		assert_eq!(data.reshape(&[8, 5, 3]).as_image_guess_bgr().info(), Ok(ImageInfo::bgr8(5, 8)));
		assert_eq!(data.reshape(&[5, 6, 4]).as_image_guess_rgb().info(), Ok(ImageInfo::rgba8(6, 5)));
		assert_eq!(data.reshape(&[5, 6, 4]).as_image_guess_bgr().info(), Ok(ImageInfo::bgra8(6, 5)));

		// Guess RGB[A]/BGR[A] from planar data.
		assert_eq!(data.reshape(&[3, 8, 5]).as_image_guess_rgb().info(), Ok(ImageInfo::rgb8(5, 8)));
		assert_eq!(data.reshape(&[3, 8, 5]).as_image_guess_bgr().info(), Ok(ImageInfo::bgr8(5, 8)));
		assert_eq!(data.reshape(&[4, 5, 6]).as_image_guess_rgb().info(), Ok(ImageInfo::rgba8(6, 5)));
		assert_eq!(data.reshape(&[4, 5, 6]).as_image_guess_bgr().info(), Ok(ImageInfo::bgra8(6, 5)));

		// Fail to guess on other dimensions
		data.reshape(&[120]).as_image_guess_rgb().info().unwrap_err();
		data.reshape(&[2, 10, 6]).as_image_guess_rgb().info().unwrap_err();
		data.reshape(&[6, 10, 2]).as_image_guess_rgb().info().unwrap_err();
		data.reshape(&[8, 5, 3, 1]).as_image_guess_rgb().info().unwrap_err();
		data.reshape(&[4, 5, 6, 1]).as_image_guess_rgb().info().unwrap_err();
	}

	#[test]
	fn tensor_info_interlaced_with_known_format() {
		let data = tch::Tensor::of_slice(&(0..60).collect::<Vec<u8>>());

		// Monochrome
		assert_eq!(data.reshape(&[12, 5, 1]).as_mono8().info(), Ok(ImageInfo::mono8(5, 12)));
		assert_eq!(data.reshape(&[12, 5]).as_mono8().info(), Ok(ImageInfo::mono8(5, 12)));
		data.reshape(&[12, 5, 1, 1]).as_mono8().info().unwrap_err();
		data.reshape(&[6, 5, 2]).as_mono8().info().unwrap_err();
		data.reshape(&[3, 5, 4]).as_mono8().info().unwrap_err();
		data.reshape(&[4, 5, 3]).as_mono8().info().unwrap_err();
		data.reshape(&[60]).as_mono8().info().unwrap_err();

		// RGB/BGR
		assert_eq!(data.reshape(&[4, 5, 3]).as_interlaced_rgb8().info(), Ok(ImageInfo::rgb8(5, 4)));
		assert_eq!(data.reshape(&[4, 5, 3]).as_interlaced_bgr8().info(), Ok(ImageInfo::bgr8(5, 4)));
		data.reshape(&[4, 5, 3, 1]).as_interlaced_bgr8().info().unwrap_err();
		data.reshape(&[4, 5, 3, 1]).as_interlaced_bgr8().info().unwrap_err();
		data.reshape(&[3, 5, 4]).as_interlaced_bgr8().info().unwrap_err();
		data.reshape(&[3, 5, 4]).as_interlaced_bgr8().info().unwrap_err();
		data.reshape(&[15, 4]).as_interlaced_rgb8().info().unwrap_err();
		data.reshape(&[15, 4]).as_interlaced_rgb8().info().unwrap_err();

		// RGBA/BGRA
		assert_eq!(data.reshape(&[3, 5, 4]).as_interlaced_rgba8().info(), Ok(ImageInfo::rgba8(5, 3)));
		assert_eq!(data.reshape(&[3, 5, 4]).as_interlaced_bgra8().info(), Ok(ImageInfo::bgra8(5, 3)));
		data.reshape(&[3, 5, 4, 1]).as_interlaced_rgba8().info().unwrap_err();
		data.reshape(&[3, 5, 4, 1]).as_interlaced_bgra8().info().unwrap_err();
		data.reshape(&[4, 5, 3]).as_interlaced_rgba8().info().unwrap_err();
		data.reshape(&[4, 5, 3]).as_interlaced_bgra8().info().unwrap_err();
		data.reshape(&[15, 4]).as_interlaced_rgba8().info().unwrap_err();
		data.reshape(&[15, 4]).as_interlaced_bgra8().info().unwrap_err();
	}

	#[test]
	fn tensor_info_planar_with_known_format() {
		let data = tch::Tensor::of_slice(&(0..60).collect::<Vec<u8>>());

		// RGB/BGR
		assert_eq!(data.reshape(&[3, 4, 5]).as_planar_rgb8().info(), Ok(ImageInfo::rgb8(5, 4)));
		assert_eq!(data.reshape(&[3, 4, 5]).as_planar_bgr8().info(), Ok(ImageInfo::bgr8(5, 4)));
		data.reshape(&[4, 5, 3, 1]).as_planar_bgr8().info().unwrap_err();
		data.reshape(&[4, 5, 3, 1]).as_planar_bgr8().info().unwrap_err();
		data.reshape(&[4, 5, 3]).as_planar_bgr8().info().unwrap_err();
		data.reshape(&[4, 5, 3]).as_planar_bgr8().info().unwrap_err();
		data.reshape(&[15, 4]).as_planar_rgb8().info().unwrap_err();
		data.reshape(&[15, 4]).as_planar_rgb8().info().unwrap_err();

		// RGBA/BGRA
		assert_eq!(data.reshape(&[4, 3, 5]).as_planar_rgba8().info(), Ok(ImageInfo::rgba8(5, 3)));
		assert_eq!(data.reshape(&[4, 3, 5]).as_planar_bgra8().info(), Ok(ImageInfo::bgra8(5, 3)));
		data.reshape(&[3, 5, 4, 1]).as_planar_rgba8().info().unwrap_err();
		data.reshape(&[3, 5, 4, 1]).as_planar_bgra8().info().unwrap_err();
		data.reshape(&[3, 5, 4]).as_planar_rgba8().info().unwrap_err();
		data.reshape(&[3, 5, 4]).as_planar_bgra8().info().unwrap_err();
		data.reshape(&[15, 4]).as_planar_rgba8().info().unwrap_err();
		data.reshape(&[15, 4]).as_planar_bgra8().info().unwrap_err();
	}
}
