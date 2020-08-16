use crate::ImageData;
use crate::ImageInfo;
use crate::PixelFormat;
use crate::Image;
use crate::RefImage;
use crate::BoxImage;

impl ImageData for image::DynamicImage {
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		let info = dynamic_image_info(self)?;
		let buffer = dynamic_image_as_bytes(self);
		Ok(Image::Ref(RefImage::new(info, buffer)))
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		let info = dynamic_image_info(&self)?;
		let buffer = dynamic_image_into_bytes(self);
		Ok(BoxImage::new(info, buffer).into())
	}
}

impl ImageData for &'_ image::DynamicImage {
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		let info = dynamic_image_info(self)?;
		let buffer = dynamic_image_as_bytes(self);
		Ok(Image::Ref(RefImage::new(info, buffer)))
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		Ok(self.image()?.into_owned())
	}
}

impl<P> ImageData for image::ImageBuffer<P, Vec<u8>>
where
	P: image::Pixel<Subpixel = u8> + 'static,
{
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		let info = info(self)?;
		let buffer = as_bytes(self);
		Ok(Image::Ref(RefImage::new(info, buffer)))
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		let info = info(&self)?;
		let buffer = into_bytes(self);
		Ok(BoxImage::new(info, buffer).into())
	}
}

impl<P> ImageData for &'_ image::ImageBuffer<P, Vec<u8>>
where
	P: image::Pixel<Subpixel = u8> + 'static,
{
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		let info = info(self)?;
		let buffer = as_bytes(self);
		Ok(Image::Ref(RefImage::new(info, buffer)))
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		Ok(self.image()?.into_owned())
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
fn info<P, C>(image: &image::ImageBuffer<P, C>) -> Result<ImageInfo, String>
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

fn dynamic_image_info(image: &image::DynamicImage) -> Result<ImageInfo, String> {
	match image {
		image::DynamicImage::ImageLuma8(x)   => info(x),
		image::DynamicImage::ImageLumaA8(x)  => info(x),
		image::DynamicImage::ImageLuma16(_)  => Err(String::from("unsupported pixel format: Luma16")),
		image::DynamicImage::ImageLumaA16(_) => Err(String::from("unsupported pixel format: LumaA16")),
		image::DynamicImage::ImageRgb8(x)    => info(x),
		image::DynamicImage::ImageRgba8(x)   => info(x),
		image::DynamicImage::ImageRgb16(_)   => Err(String::from("unsupported pixel format: Rgb16")),
		image::DynamicImage::ImageRgba16(_)  => Err(String::from("unsupported pixel format: Rgba16")),
		image::DynamicImage::ImageBgr8(x)    => info(x),
		image::DynamicImage::ImageBgra8(x)   => info(x),
	}
}

/// Extract the PixelFormat from an [`image::Pixel`].
fn pixel_format<P: image::Pixel>() -> Result<PixelFormat, String> {
	match P::COLOR_TYPE {
		image::ColorType::Bgr8  => Ok(PixelFormat::Bgr8),
		image::ColorType::Bgra8 => Ok(PixelFormat::Bgra8),
		image::ColorType::Rgb8  => Ok(PixelFormat::Rgb8),
		image::ColorType::Rgba8 => Ok(PixelFormat::Rgba8),
		image::ColorType::L8    => Ok(PixelFormat::Mono8),
		x  => Err(format!("unsupported color type: {:?}", x)),
	}
}
