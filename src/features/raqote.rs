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
	fn info(&self) -> Result<ImageInfo, String> {
		if self.width() < 0 || self.height() < 0 {
			Err(format!("DrawTarget has negative size: [{}, {}]", self.width(), self.height()))
		} else {
			Ok(ImageInfo::new(PixelFormat::Bgra8, self.width() as usize, self.height() as usize))
		}
	}

	fn data(self) -> Box<[u8]> {
		let length = self.get_data_u8().len();
		let data = Box::into_raw(self.into_vec().into_boxed_slice()) as *mut u8;
		unsafe {
			let mut data = Box::from_raw(std::slice::from_raw_parts_mut(data, length));
			divide_by_alpha(&mut data);
			data
		}
	}
}

impl ImageData for &'_ raqote::DrawTarget {
	fn info(&self) -> Result<ImageInfo, String> {
		(*self).info()
	}

	fn data(self) -> Box<[u8]> {
		let mut data = Box::from(self.get_data_u8());
		divide_by_alpha(&mut data);
		data
	}
}

impl<'a> ImageData for raqote::Image<'a> {
	fn info(&self) -> Result<ImageInfo, String> {
		if self.width < 0 || self.height < 0 {
			Err(format!("image has negative size: [{}, {}]", self.width, self.height))
		} else {
			Ok(ImageInfo::new(PixelFormat::Bgra8, self.width as usize, self.height as usize))
		}
	}

	fn data(self) -> Box<[u8]> {
		let data = self.data.as_ptr() as *const u8;
		unsafe {
			let mut data = Box::from(std::slice::from_raw_parts(data, self.data.len() * 4));
			divide_by_alpha(&mut data);
			data
		}
	}
}

impl<'a> ImageData for &'_ raqote::Image<'a> {
	fn info(&self) -> Result<ImageInfo, String> {
		(*self).info()
	}

	fn data(self) -> Box<[u8]> {
		(*self).data()
	}
}
