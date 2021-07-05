//! Support for the [`raqote`][::raqote] crate.

use crate::error::ImageDataError;
use crate::BoxImage;
use crate::Image;
use crate::ImageInfo;

impl From<raqote::DrawTarget> for Image {
	fn from(other: raqote::DrawTarget) -> Self {
		let info = match draw_target_info(&other) {
			Ok(x) => x,
			Err(e) => return Image::Invalid(e),
		};

		let length = other.get_data_u8().len();
		let buffer = Box::into_raw(other.into_vec().into_boxed_slice()) as *mut u8;
		let buffer = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(buffer, length)) };

		BoxImage::new(info, buffer).into()
	}
}

impl From<&raqote::DrawTarget> for Image {
	fn from(other: &raqote::DrawTarget) -> Self {
		let info = match draw_target_info(other) {
			Ok(x) => x,
			Err(e) => return Image::Invalid(e),
		};

		let buffer = Box::from(other.get_data_u8());

		BoxImage::new(info, buffer).into()
	}
}

impl From<raqote::Image<'_>> for Image {
	fn from(other: raqote::Image) -> Self {
		let info = match image_info(&other) {
			Ok(x) => x,
			Err(e) => return Image::Invalid(e),
		};

		let buffer = other.data.as_ptr() as *const u8;
		let buffer = unsafe { Box::from(std::slice::from_raw_parts(buffer, other.data.len() * 4)) };

		BoxImage::new(info, buffer).into()
	}
}

fn draw_target_info(draw_target: &raqote::DrawTarget) -> Result<ImageInfo, ImageDataError> {
	if draw_target.width() < 0 || draw_target.height() < 0 {
		Err(format!("DrawTarget has negative size: [{}, {}]", draw_target.width(), draw_target.height()).into())
	} else {
		Ok(ImageInfo::bgra8_premultiplied(
			draw_target.width() as u32,
			draw_target.height() as u32,
		))
	}
}

fn image_info(&image: &raqote::Image) -> Result<ImageInfo, ImageDataError> {
	if image.width < 0 || image.height < 0 {
		Err(format!("DrawTarget has negative size: [{}, {}]", image.width, image.height).into())
	} else {
		Ok(ImageInfo::bgra8_premultiplied(image.width as u32, image.height as u32))
	}
}
