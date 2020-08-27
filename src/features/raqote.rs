use crate::BoxImage;
use crate::Image;
use crate::ImageInfo;
use crate::PixelFormat;
use crate::error::ImageDataError;

// TODO: support premultiplied alpha in shader, and implement AsImageView too.

fn divide_by_alpha(data: &mut [u8]) {
	for i in 0..(data.len() / 4) {
		let i = i * 4;
		if data[i + 3] != 0  && data[i + 3] != 255 {
			data[i + 0] = (u16::from(data[i + 0]) * 255 / u16::from(data[i + 3])) as u8;
			data[i + 1] = (u16::from(data[i + 1]) * 255 / u16::from(data[i + 3])) as u8;
			data[i + 2] = (u16::from(data[i + 2]) * 255 / u16::from(data[i + 3])) as u8;
		}
	}
}

impl From<raqote::DrawTarget> for Image {
	fn from(other: raqote::DrawTarget) -> Self {
		let info = match draw_target_info(&other) {
			Ok(x) => x,
			Err(e) => return Image::Invalid(e),
		};

		let length = other.get_data_u8().len();
		let buffer = Box::into_raw(other.into_vec().into_boxed_slice()) as *mut u8;
		let mut buffer = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(buffer, length)) };
		divide_by_alpha(&mut buffer);

		BoxImage::new(info, buffer).into()
	}
}

impl From<&raqote::DrawTarget> for Image {
	fn from(other: &raqote::DrawTarget) -> Self {
		let info = match draw_target_info(&other) {
			Ok(x) => x,
			Err(e) => return Image::Invalid(e),
		};

		let mut buffer = Box::from(other.get_data_u8());
		divide_by_alpha(&mut buffer);

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
		let mut buffer = unsafe { Box::from(std::slice::from_raw_parts(buffer, other.data.len() * 4)) };
		divide_by_alpha(&mut buffer);

		BoxImage::new(info, buffer).into()
	}
}

fn draw_target_info(draw_target: &raqote::DrawTarget) -> Result<ImageInfo, ImageDataError> {
	if draw_target.width() < 0 || draw_target.height() < 0 {
		Err(format!("DrawTarget has negative size: [{}, {}]", draw_target.width(), draw_target.height()).into())
	} else {
		Ok(ImageInfo::new(PixelFormat::Bgra8, draw_target.width() as u32, draw_target.height() as u32))
	}
}

fn image_info(&image: &raqote::Image) -> Result<ImageInfo, ImageDataError> {
	if image.width < 0 || image.height < 0 {
		Err(format!("DrawTarget has negative size: [{}, {}]", image.width, image.height).into())
	} else {
		Ok(ImageInfo::new(PixelFormat::Bgra8, image.width as u32, image.height as u32))
	}
}
