use crate::BoxImage;
use crate::Image;
use crate::ImageView;
use crate::AsImageView;
use crate::ImageInfo;
use crate::PixelFormat;
use crate::error::ImageDataError;

impl AsImageView for image::DynamicImage {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		let info = dynamic_image_info(self)?;
		let data = dynamic_image_as_bytes(self);
		Ok(ImageView::new(info, data))
	}
}

impl AsImageView for &'_ image::DynamicImage {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		(*self).as_image_view()
	}
}

impl From<image::DynamicImage> for Image {
	fn from(other: image::DynamicImage) -> Self {
		let info = match dynamic_image_info(&other) {
			Ok(x) => x,
			Err(e) => return Self::Invalid(e),
		};
		let data = dynamic_image_into_bytes(other);
		BoxImage::new(info, data).into()
	}
}

impl<P> AsImageView for image::ImageBuffer<P, Vec<u8>>
where
	P: image::Pixel<Subpixel = u8> + 'static,
{
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		let info = info(self)?;
		let data = as_bytes(self);
		Ok(ImageView::new(info, data))
	}
}

impl<P> AsImageView for &'_ image::ImageBuffer<P, Vec<u8>>
where
	P: image::Pixel<Subpixel = u8> + 'static,
{
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		(*self).as_image_view()
	}
}

impl<P> From<image::ImageBuffer<P, Vec<u8>>> for Image
where
	P: image::Pixel<Subpixel = u8> + 'static,
{
	fn from(other: image::ImageBuffer<P, Vec<u8>>) -> Self {
		let info = match info(&other) {
			Ok(x) => x,
			Err(e) => return Self::Invalid(e),
		};
		let data = into_bytes(other);
		BoxImage::new(info, data).into()
	}
}

/// Consume an [`image::ImageBuffer`] and return the pixel data as boxed slice.
fn into_bytes<P: 'static + image::Pixel<Subpixel = u8>>(buffer: image::ImageBuffer<P, Vec<u8>>) -> Box<[u8]> {
	buffer.into_raw().into_boxed_slice()
}

fn dynamic_image_into_bytes(image: image::DynamicImage) -> Box<[u8]> {
	match image {
		image::DynamicImage::ImageLuma8(x)   => into_bytes(x),
		image::DynamicImage::ImageLumaA8(x)  => into_bytes(x),
		image::DynamicImage::ImageLuma16(_)  => panic!("unsupported pixel format: Luma16"),
		image::DynamicImage::ImageLumaA16(_) => panic!("unsupported pixel format: LumaA16"),
		image::DynamicImage::ImageRgb8(x)    => into_bytes(x),
		image::DynamicImage::ImageRgba8(x)   => into_bytes(x),
		image::DynamicImage::ImageRgb16(_)   => panic!("unsupported pixel format: Rgb16"),
		image::DynamicImage::ImageRgba16(_)  => panic!("unsupported pixel format: Rgba16"),
		image::DynamicImage::ImageBgr8(x)    => into_bytes(x),
		image::DynamicImage::ImageBgra8(x)   => into_bytes(x),
	}
}

/// Get the pixel data of an [`image::ImageBuffer`] to as a byte slice.
fn as_bytes<P: 'static + image::Pixel<Subpixel = u8>>(buffer: &image::ImageBuffer<P, Vec<u8>>) -> &[u8] {
	&*buffer
}

fn dynamic_image_as_bytes(image: &image::DynamicImage) -> &[u8] {
	match image {
		image::DynamicImage::ImageLuma8(x)   => as_bytes(x),
		image::DynamicImage::ImageLumaA8(x)  => as_bytes(x),
		image::DynamicImage::ImageLuma16(_)  => panic!("unsupported pixel format: Luma16"),
		image::DynamicImage::ImageLumaA16(_) => panic!("unsupported pixel format: LumaA16"),
		image::DynamicImage::ImageRgb8(x)    => as_bytes(x),
		image::DynamicImage::ImageRgba8(x)   => as_bytes(x),
		image::DynamicImage::ImageRgb16(_)   => panic!("unsupported pixel format: Rgb16"),
		image::DynamicImage::ImageRgba16(_)  => panic!("unsupported pixel format: Rgba16"),
		image::DynamicImage::ImageBgr8(x)    => as_bytes(x),
		image::DynamicImage::ImageBgra8(x)   => as_bytes(x),
	}
}

/// Extract the [`ImageInfo`] from an [`image::ImageBuffer`].
fn info<P, C>(image: &image::ImageBuffer<P, C>) -> Result<ImageInfo, ImageDataError>
where
	P: image::Pixel<Subpixel = u8> + 'static,
	C: std::ops::Deref<Target = [u8]>,
{
	Ok(ImageInfo {
		pixel_format: pixel_format::<P>()?,
		width: image.width(),
		height: image.height(),
		stride_x: image.sample_layout().width_stride as u32,
		stride_y: image.sample_layout().height_stride as u32,
	})
}

fn dynamic_image_info(image: &image::DynamicImage) -> Result<ImageInfo, ImageDataError> {
	match image {
		image::DynamicImage::ImageLuma8(x)   => info(x),
		image::DynamicImage::ImageLumaA8(x)  => info(x),
		image::DynamicImage::ImageLuma16(_)  => Err("unsupported pixel format: Luma16".into()),
		image::DynamicImage::ImageLumaA16(_) => Err("unsupported pixel format: LumaA16".into()),
		image::DynamicImage::ImageRgb8(x)    => info(x),
		image::DynamicImage::ImageRgba8(x)   => info(x),
		image::DynamicImage::ImageRgb16(_)   => Err("unsupported pixel format: Rgb16".into()),
		image::DynamicImage::ImageRgba16(_)  => Err("unsupported pixel format: Rgba16".into()),
		image::DynamicImage::ImageBgr8(x)    => info(x),
		image::DynamicImage::ImageBgra8(x)   => info(x),
	}
}

/// Extract the PixelFormat from an [`image::Pixel`].
fn pixel_format<P: image::Pixel>() -> Result<PixelFormat, ImageDataError> {
	match P::COLOR_TYPE {
		image::ColorType::Bgr8  => Ok(PixelFormat::Bgr8),
		image::ColorType::Bgra8 => Ok(PixelFormat::Bgra8),
		image::ColorType::Rgb8  => Ok(PixelFormat::Rgb8),
		image::ColorType::Rgba8 => Ok(PixelFormat::Rgba8),
		image::ColorType::L8    => Ok(PixelFormat::Mono8),
		x  => Err(format!("unsupported color type: {:?}", x).into()),
	}
}
