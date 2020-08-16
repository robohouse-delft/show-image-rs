use crate::BoxImage;
use crate::Image;
use crate::ImageData;
use crate::ImageInfo;
use crate::PixelFormat;

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

impl ImageData for raqote::DrawTarget {
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		let info = draw_target_info(self)?;

		let mut buffer = Box::from(self.get_data_u8());
		divide_by_alpha(&mut buffer);

		Ok(BoxImage::new(info, buffer).into())
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		let info = draw_target_info(&self)?;

		let length = self.get_data_u8().len();
		let buffer = Box::into_raw(self.into_vec().into_boxed_slice()) as *mut u8;
		let mut buffer = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(buffer, length)) };
		divide_by_alpha(&mut buffer);

		Ok(BoxImage::new(info, buffer).into())
	}
}

impl ImageData for &'_ raqote::DrawTarget {
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		(*self).image()
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		Ok(self.image()?.into_owned())
	}
}

impl<'a> ImageData for raqote::Image<'a> {
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		let info = image_info(self)?;

		let buffer = self.data.as_ptr() as *const u8;
		let mut buffer = unsafe { Box::from(std::slice::from_raw_parts(buffer, self.data.len() * 4)) };
		divide_by_alpha(&mut buffer);

		Ok(BoxImage::new(info, buffer).into())
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		Ok(self.image()?.into_owned())
	}
}

impl<'a> ImageData for &'_ raqote::Image<'a> {
	type Error = String;

	fn image(&self) -> Result<Image, String> {
		(*self).image()
	}

	fn into_image(self) -> Result<Image<'static>, String> {
		Ok(self.image()?.into_owned())
	}
}

fn draw_target_info(draw_target: &raqote::DrawTarget) -> Result<ImageInfo, String> {
	if draw_target.width() < 0 || draw_target.height() < 0 {
		Err(format!("DrawTarget has negative size: [{}, {}]", draw_target.width(), draw_target.height()))
	} else {
		Ok(ImageInfo::new(PixelFormat::Bgra8, draw_target.width() as u32, draw_target.height() as u32))
	}
}

fn image_info(&image: &raqote::Image) -> Result<ImageInfo, String> {
	if image.width < 0 || image.height < 0 {
		Err(format!("DrawTarget has negative size: [{}, {}]", image.width, image.height))
	} else {
		Ok(ImageInfo::new(PixelFormat::Bgra8, image.width as u32, image.height as u32))
	}
}
