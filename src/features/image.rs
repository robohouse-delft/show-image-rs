use crate::ImageData;
use crate::ImageInfo;
use crate::PixelFormat;

use image::DynamicImage;
use image::GenericImageView;
use std::ops::Deref;

fn as_bytes<P: 'static + image::Pixel<Subpixel = u8>>(buffer: &image::ImageBuffer<P, Vec<u8>>) -> &[u8] {
	buffer.deref()
}

impl ImageData for DynamicImage {
	fn data(&self) -> &[u8] {
		match self {
			Self::ImageLuma8(x)  => as_bytes(x),
			Self::ImageLumaA8(x) => as_bytes(x),
			Self::ImageRgb8(x)   => as_bytes(x),
			Self::ImageRgba8(x)  => as_bytes(x),
			Self::ImageBgr8(x)   => as_bytes(x),
			Self::ImageBgra8(x)  => as_bytes(x),
		}
	}

	fn info(&self) -> Result<ImageInfo, String> {
		let (pixel_format, layout) = match self {
			Self::ImageLuma8(x)  => Ok((PixelFormat::Mono8, x.sample_layout())),
			Self::ImageLumaA8(_) => Err("8-bit mono with alpha channel is not supported"),
			Self::ImageRgb8(x)   => Ok((PixelFormat::Rgb8,  x.sample_layout())),
			Self::ImageRgba8(x)  => Ok((PixelFormat::Rgba8, x.sample_layout())),
			Self::ImageBgr8(x)   => Ok((PixelFormat::Bgr8,  x.sample_layout())),
			Self::ImageBgra8(x)  => Ok((PixelFormat::Bgra8, x.sample_layout())),
		}?;

		Ok(ImageInfo {
			pixel_format,
			width: self.width() as usize,
			height: self.height() as usize,
			row_stride: layout.height_stride,
		})
	}
}
