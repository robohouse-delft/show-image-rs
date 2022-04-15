//! Support for the [`image`][::image] crate.

use std::ops::Deref;

use crate::error::ImageDataError;
use crate::Alpha;
use crate::AsImageView;
use crate::BoxImage;
use crate::Image;
use crate::ImageInfo;
use crate::ImageView;
use crate::PixelFormat;
use crate::error::UnsupportedImageFormat;

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

impl<P, Container> AsImageView for image::ImageBuffer<P, Container>
where
	P: image::Pixel<Subpixel = u8> + image::PixelWithColorType,
	Container: Deref<Target = [u8]>,
{
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		let info = info(self)?;
		let data = as_bytes(self);
		Ok(ImageView::new(info, data))
	}
}

impl<P, Container> AsImageView for &'_ image::ImageBuffer<P, Container>
where
	P: image::Pixel<Subpixel = u8> + image::PixelWithColorType,
	Container: Deref<Target = [u8]>,
{
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		(*self).as_image_view()
	}
}

impl<P, Container> From<image::ImageBuffer<P, Container>> for Image
where
	P: image::Pixel<Subpixel = u8> + image::PixelWithColorType,
	Container: Deref<Target = [u8]>,
{
	fn from(other: image::ImageBuffer<P, Container>) -> Self {
		let info = match info(&other) {
			Ok(x) => x,
			Err(e) => return Self::Invalid(e),
		};
		let data = into_bytes(other);
		BoxImage::new(info, data).into()
	}
}

/// Consume an [`image::ImageBuffer`] and return the pixel data as boxed slice.
fn into_bytes<P, Container>(buffer: image::ImageBuffer<P, Container>) -> Box<[u8]>
where
	P: image::Pixel<Subpixel = u8> + image::PixelWithColorType,
	Container: Deref<Target = [u8]>,
{
	// TODO: Specialize this for Vec<u8> to avoid copying when
	// https://github.com/rust-lang/rust/issues/31844 lands in stable.
	Box::from(buffer.into_raw().deref())
}

fn dynamic_image_into_bytes(image: image::DynamicImage) -> Box<[u8]> {
	match image {
		image::DynamicImage::ImageLuma8(x) => into_bytes(x),
		image::DynamicImage::ImageLumaA8(x) => into_bytes(x),
		image::DynamicImage::ImageLuma16(_) => panic!("unsupported pixel format: Luma16"),
		image::DynamicImage::ImageLumaA16(_) => panic!("unsupported pixel format: LumaA16"),
		image::DynamicImage::ImageRgb8(x) => into_bytes(x),
		image::DynamicImage::ImageRgba8(x) => into_bytes(x),
		image::DynamicImage::ImageRgb16(_) => panic!("unsupported pixel format: Rgb16"),
		image::DynamicImage::ImageRgba16(_) => panic!("unsupported pixel format: Rgba16"),
		image::DynamicImage::ImageRgb32F(_) => panic!("unsupported pixel format: Rgb32F"),
		image::DynamicImage::ImageRgba32F(_) => panic!("unsupported pixel format: Rgba32F"),
		x => panic!("unsupported pixel format: {:?}", x),
	}
}

/// Get the pixel data of an [`image::ImageBuffer`] to as a byte slice.
fn as_bytes<P, Container>(buffer: &image::ImageBuffer<P, Container>) -> &[u8]
where
	P: image::Pixel<Subpixel = u8> + image::PixelWithColorType,
	Container: Deref<Target = [u8]>,
{
	&*buffer
}

fn dynamic_image_as_bytes(image: &image::DynamicImage) -> &[u8] {
	match image {
		image::DynamicImage::ImageLuma8(x) => as_bytes(x),
		image::DynamicImage::ImageLumaA8(x) => as_bytes(x),
		image::DynamicImage::ImageLuma16(_) => panic!("unsupported pixel format: Luma16"),
		image::DynamicImage::ImageLumaA16(_) => panic!("unsupported pixel format: LumaA16"),
		image::DynamicImage::ImageRgb8(x) => as_bytes(x),
		image::DynamicImage::ImageRgba8(x) => as_bytes(x),
		image::DynamicImage::ImageRgb16(_) => panic!("unsupported pixel format: Rgb16"),
		image::DynamicImage::ImageRgba16(_) => panic!("unsupported pixel format: Rgba16"),
		image::DynamicImage::ImageRgb32F(_) => panic!("unsupported pixel format: Rgb32F"),
		image::DynamicImage::ImageRgba32F(_) => panic!("unsupported pixel format: Rgba32F"),
		x => panic!("unsupported pixel format: {:?}", x),
	}
}

/// Extract the [`ImageInfo`] from an [`image::ImageBuffer`].
fn info<P, C>(image: &image::ImageBuffer<P, C>) -> Result<ImageInfo, ImageDataError>
where
	P: image::Pixel<Subpixel = u8> + image::PixelWithColorType,
	C: std::ops::Deref<Target = [u8]>,
{
	Ok(ImageInfo {
		pixel_format: pixel_format::<P>()?,
		size: glam::UVec2::new(image.width(), image.height()),
		stride: glam::UVec2::new(
			image.sample_layout().width_stride as u32,
			image.sample_layout().height_stride as u32,
		),
	})
}

fn dynamic_image_info(image: &image::DynamicImage) -> Result<ImageInfo, ImageDataError> {
	match image {
		image::DynamicImage::ImageLuma8(x) => info(x),
		image::DynamicImage::ImageLumaA8(x) => info(x),
		image::DynamicImage::ImageRgb8(x) => info(x),
		image::DynamicImage::ImageRgba8(x) => info(x),
		x => Err(UnsupportedImageFormat { format: format!("{:?}", x) }.into()),
	}
}

/// Extract the PixelFormat from an [`image::Pixel`].
fn pixel_format<P: image::PixelWithColorType>() -> Result<PixelFormat, ImageDataError> {
	match P::COLOR_TYPE {
		image::ColorType::L8 => Ok(PixelFormat::Mono8),
		image::ColorType::La8 => Ok(PixelFormat::MonoAlpha8(Alpha::Unpremultiplied)),
		image::ColorType::Rgb8 => Ok(PixelFormat::Rgb8),
		image::ColorType::Rgba8 => Ok(PixelFormat::Rgba8(Alpha::Unpremultiplied)),
		x => Err(UnsupportedImageFormat { format: format!("{:?}", x) }.into()),
	}
}
