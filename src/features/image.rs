use crate::ImageData;
use crate::ImageInfo;
use crate::PixelFormat;

impl ImageData for image::DynamicImage {
	fn data(self) -> Box<[u8]> {
		match self {
			image::DynamicImage::ImageLuma8(x)  => into_bytes(x),
			image::DynamicImage::ImageLumaA8(x) => into_bytes(x),
			image::DynamicImage::ImageRgb8(x)   => into_bytes(x),
			image::DynamicImage::ImageRgba8(x)  => into_bytes(x),
			image::DynamicImage::ImageBgr8(x)   => into_bytes(x),
			image::DynamicImage::ImageBgra8(x)  => into_bytes(x),
		}
	}

	fn info(&self) -> Result<ImageInfo, String> {
		match self {
			image::DynamicImage::ImageLuma8(x)  => info(x),
			image::DynamicImage::ImageLumaA8(x) => info(x),
			image::DynamicImage::ImageRgb8(x)   => info(x),
			image::DynamicImage::ImageRgba8(x)  => info(x),
			image::DynamicImage::ImageBgr8(x)   => info(x),
			image::DynamicImage::ImageBgra8(x)  => info(x),
		}
	}
}

impl ImageData for &'_ image::DynamicImage {
	fn data(self) -> Box<[u8]> {
		match self {
			image::DynamicImage::ImageLuma8(x)  => Box::from(as_bytes(x)),
			image::DynamicImage::ImageLumaA8(x) => Box::from(as_bytes(x)),
			image::DynamicImage::ImageRgb8(x)   => Box::from(as_bytes(x)),
			image::DynamicImage::ImageRgba8(x)  => Box::from(as_bytes(x)),
			image::DynamicImage::ImageBgr8(x)   => Box::from(as_bytes(x)),
			image::DynamicImage::ImageBgra8(x)  => Box::from(as_bytes(x)),
		}
	}

	fn info(&self) -> Result<ImageInfo, String> {
		(*self).info()
	}
}

impl<P> ImageData for image::ImageBuffer<P, Vec<u8>>
where
	P: image::Pixel<Subpixel = u8> + 'static,
{
	fn data(self) -> Box<[u8]> {
		self.into_raw().into_boxed_slice()
	}

	fn info(&self) -> Result<ImageInfo, String> {
		info(self)
	}
}

impl<P> ImageData for &'_ image::ImageBuffer<P, Vec<u8>>
where
	P: image::Pixel<Subpixel = u8> + 'static,
{
	fn data(self) -> Box<[u8]> {
		Box::from(as_bytes(self))
	}

	fn info(&self) -> Result<ImageInfo, String> {
		(*self).info()
	}
}

/// Consume an [`image::ImageBuffer`] and return the pixel data as boxed slice.
fn into_bytes<P: 'static + image::Pixel<Subpixel = u8>>(buffer: image::ImageBuffer<P, Vec<u8>>) -> Box<[u8]> {
	buffer.into_raw().into_boxed_slice()
}

/// Copy the pixel data of an [`image::ImageBuffer`] to a boxed slice.
fn as_bytes<P: 'static + image::Pixel<Subpixel = u8>>(buffer: &image::ImageBuffer<P, Vec<u8>>) -> &[u8] {
	&*buffer
}

/// Extract the [`ImageInfo`] from an [`image::ImageBuffer`].
fn info<P, C>(image: &image::ImageBuffer<P, C>) -> Result<ImageInfo, String>
where
	P: image::Pixel<Subpixel = u8> + 'static,
	C: std::ops::Deref<Target = [u8]>,
{
	Ok(ImageInfo {
		pixel_format: pixel_format::<P>()?,
		width: image.width() as usize,
		height: image.height() as usize,
		row_stride: image.sample_layout().height_stride,
	})
}

/// Extract the PixelFormat from an [`image::Pixel`].
fn pixel_format<P: image::Pixel>() -> Result<PixelFormat, String> {
	match P::COLOR_TYPE {
		image::ColorType::BGR(8)  => Ok(PixelFormat::Bgr8),
		image::ColorType::BGRA(8) => Ok(PixelFormat::Bgra8),
		image::ColorType::RGB(8)  => Ok(PixelFormat::Rgb8),
		image::ColorType::RGBA(8) => Ok(PixelFormat::Rgba8),
		image::ColorType::Gray(8) => Ok(PixelFormat::Mono8),
		x  => Err(format!("unsupported color type: {:?}", x)),
	}
}
