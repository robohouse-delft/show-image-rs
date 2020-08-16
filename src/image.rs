use std::sync::Arc;

use crate::ImageInfo;

#[derive(Debug, Clone)]
pub enum Image<'a> {
	Ref(RefImage<'a>),
	Box(BoxImage),
	Arc(ArcImage),
}

#[derive(Debug, Copy, Clone)]
pub struct RefImage<'a> {
	info: ImageInfo,
	buffer: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct BoxImage {
	info: ImageInfo,
	buffer: Box<[u8]>,
}

#[derive(Debug, Clone)]
pub struct ArcImage {
	info: ImageInfo,
	buffer: Arc<[u8]>,
}

impl Image<'_> {
	pub fn into_owned(self) -> Image<'static> {
		match self {
			Image::Ref(x) => Image::Box(x.to_box()),
			Image::Box(x) => Image::Box(x),
			Image::Arc(x) => Image::Arc(x),
		}
	}

	pub fn into_arc(self) -> ArcImage {
		match self {
			Self::Ref(x) => x.to_arc(),
			Self::Box(x) => ArcImage::from(x),
			Self::Arc(x) => x,
		}
	}

	pub fn as_ref<'a>(&'a self) -> RefImage<'a> {
		match self {
			Self::Ref(x) => *x,
			Self::Box(x) => x.into(),
			Self::Arc(x) => x.into(),
		}
	}

	pub fn info(&self) -> &ImageInfo {
		match self {
			Self::Ref(x) => &x.info,
			Self::Box(x) => &x.info,
			Self::Arc(x) => &x.info,
		}
	}

	pub fn buffer(&self) -> &[u8] {
		match self {
			Self::Ref(x) => x.buffer,
			Self::Box(x) => &x.buffer,
			Self::Arc(x) => &x.buffer,
		}
	}
}

impl<'a> RefImage<'a> {
	pub fn new(info: ImageInfo, buffer: &'a [u8]) -> Self {
		Self { info, buffer }
	}

	pub fn to_box(self) -> BoxImage {
		BoxImage {
			info: self.info,
			buffer: self.buffer.into(),
		}
	}

	pub fn to_arc(self) -> ArcImage {
		ArcImage {
			info: self.info,
			buffer: self.buffer.into(),
		}
	}

	pub fn info(&self) -> &ImageInfo {
		&self.info
	}

	pub fn buffer(&self) -> &[u8] {
		self.buffer
	}
}

impl BoxImage {
	pub fn new(info: ImageInfo, buffer: Box<[u8]>) -> Self {
		Self { info, buffer }
	}

	pub fn info(&self) -> &ImageInfo {
		&self.info
	}

	pub fn buffer(&self) -> &[u8] {
		&self.buffer
	}
}

impl ArcImage {
	pub fn new(info: ImageInfo, buffer: Arc<[u8]>) -> Self {
		Self { info, buffer }
	}

	pub fn info(&self) -> &ImageInfo {
		&self.info
	}

	pub fn buffer(&self) -> &[u8] {
		&self.buffer
	}
}

impl<'a> From<RefImage<'a>> for Image<'a> {
	fn from(other: RefImage<'a>) -> Self {
		Self::Ref(other)
	}
}

impl<'a> From<&'a RefImage<'a>> for Image<'a> {
	fn from(other: &'a RefImage<'a>) -> Self {
		Self::Ref(*other)
	}
}

impl From<BoxImage> for Image<'_> {
	fn from(other: BoxImage) -> Self {
		Self::Box(other)
	}
}

impl From<ArcImage> for Image<'_> {
	fn from(other: ArcImage) -> Self {
		Self::Arc(other)
	}
}

impl<'a> From<&'a Image<'_>> for RefImage<'a> {
	fn from(other: &'a Image) -> Self {
		other.as_ref()
	}
}

impl<'a> From<&'a BoxImage> for RefImage<'a> {
	fn from(other: &'a BoxImage) -> Self {
		Self {
			info: other.info,
			buffer: other.buffer.as_ref(),
		}
	}
}

impl<'a> From<&'a ArcImage> for RefImage<'a> {
	fn from(other: &'a ArcImage) -> Self {
		Self {
			info: other.info,
			buffer: other.buffer.as_ref(),
		}
	}
}

impl From<BoxImage> for ArcImage {
	fn from(other: BoxImage) -> Self {
		Self {
			info: other.info,
			buffer: other.buffer.into()
		}
	}
}
