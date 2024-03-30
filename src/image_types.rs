use std::sync::Arc;

use crate::error::ImageDataError;
use crate::ImageInfo;

/// Trait for borrowing image data from a struct.
pub trait AsImageView {
	/// Get an image view for the object.
	fn as_image_view(&self) -> Result<ImageView, ImageDataError>;
}

/// Get the image info of an object that implements [`AsImageView`].
pub fn image_info(image: &impl AsImageView) -> Result<ImageInfo, ImageDataError> {
	Ok(image.as_image_view()?.info())
}

/// Borrowed view of image data,
#[derive(Debug, Copy, Clone)]
pub struct ImageView<'a> {
	info: ImageInfo,
	data: &'a [u8],
}

impl<'a> ImageView<'a> {
	/// Create a new image view from image information and a data slice.
	pub fn new(info: ImageInfo, data: &'a [u8]) -> Self {
		Self { info, data }
	}

	/// Get the image information.
	pub fn info(&self) -> ImageInfo {
		self.info
	}

	/// Get the image data as byte slice.
	pub fn data(&self) -> &[u8] {
		self.data
	}
}

impl<'a> AsImageView for ImageView<'a> {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		Ok(*self)
	}
}

/// Owning image that can be sent to another thread.
///
/// The image is backed by either a [`Box`] or [`Arc`].
/// It can either directly own the data or through a [`dyn AsImageView`].
pub enum Image {
	/// An image backed by a `Box<[u8]>`.
	Box(BoxImage),

	/// An image backed by an `Arc<[u8]>`.
	Arc(ArcImage),

	/// An image backed by a `Box<dyn AsImageView>`.
	BoxDyn(Box<dyn AsImageView + Send>),

	/// An image backed by an `Arc<dyn AsImageView>`.
	ArcDyn(Arc<dyn AsImageView + Sync + Send>),

	/// An invalid image that will always fail the conversion to [`ImageView`].
	Invalid(ImageDataError),
}

impl Clone for Image {
	fn clone(&self) -> Self {
		match self {
			Self::Box(x) => Self::Box(x.clone()),
			Self::Arc(x) => Self::Arc(x.clone()),
			// We can not clone Box<dyn AsImageView> directly, but we can clone the data or the error.
			Self::BoxDyn(x) => match x.as_image_view() {
				Ok(view) => Self::Box(BoxImage::new(view.info, view.data.into())),
				Err(error) => Self::Invalid(error),
			},
			Self::ArcDyn(x) => Self::ArcDyn(x.clone()),
			Self::Invalid(x) => Self::Invalid(x.clone()),
		}
	}
}

impl<T: AsImageView> AsImageView for Box<T> {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		self.as_ref().as_image_view()
	}
}

impl<T: AsImageView> AsImageView for Arc<T> {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		self.as_ref().as_image_view()
	}
}

/// Image backed by a `Box<[u8]>`.
#[derive(Debug, Clone)]
pub struct BoxImage {
	info: ImageInfo,
	data: Box<[u8]>,
}

/// Image backed by an `Arc<[u8]>`.
#[derive(Debug, Clone)]
pub struct ArcImage {
	info: ImageInfo,
	data: Arc<[u8]>,
}

impl Image {
	/// Get a non-owning view of the image data.
	pub fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		match self {
			Self::Box(x) => Ok(x.as_view()),
			Self::Arc(x) => Ok(x.as_view()),
			Self::BoxDyn(x) => x.as_image_view(),
			Self::ArcDyn(x) => x.as_image_view(),
			Self::Invalid(e) => Err(e.clone()),
		}
	}
}

impl AsImageView for Image {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		self.as_image_view()
	}
}

impl BoxImage {
	/// Create a new image from image information and a boxed slice.
	pub fn new(info: ImageInfo, data: Box<[u8]>) -> Self {
		Self { info, data }
	}

	/// Get a non-owning view of the image data.
	pub fn as_view(&self) -> ImageView {
		ImageView::new(self.info, &self.data)
	}

	/// Get the image information.
	pub fn info(&self) -> ImageInfo {
		self.info
	}

	/// Get the image data as byte slice.
	pub fn data(&self) -> &[u8] {
		&self.data
	}
}

impl AsImageView for BoxImage {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		Ok(self.as_view())
	}
}

impl ArcImage {
	/// Create a new image from image information and a Arc-wrapped slice.
	pub fn new(info: ImageInfo, data: Arc<[u8]>) -> Self {
		Self { info, data }
	}

	/// Get a non-owning view of the image data.
	pub fn as_view(&self) -> ImageView {
		ImageView::new(self.info, &self.data)
	}

	/// Get the image information.
	pub fn info(&self) -> ImageInfo {
		self.info
	}

	/// Get the image data as byte slice.
	pub fn data(&self) -> &[u8] {
		&self.data
	}
}

impl AsImageView for ArcImage {
	fn as_image_view(&self) -> Result<ImageView, ImageDataError> {
		Ok(self.as_view())
	}
}

impl From<ImageView<'_>> for BoxImage {
	fn from(other: ImageView) -> Self {
		Self {
			info: other.info,
			data: other.data.into(),
		}
	}
}

impl From<&'_ ImageView<'_>> for BoxImage {
	fn from(other: &ImageView) -> Self {
		Self {
			info: other.info,
			data: other.data.into(),
		}
	}
}

impl From<ImageView<'_>> for ArcImage {
	fn from(other: ImageView) -> Self {
		Self {
			info: other.info,
			data: other.data.into(),
		}
	}
}

impl From<&'_ ImageView<'_>> for ArcImage {
	fn from(other: &ImageView) -> Self {
		Self {
			info: other.info,
			data: other.data.into(),
		}
	}
}

impl From<ImageView<'_>> for Image {
	fn from(other: ImageView) -> Self {
		Self::Box(BoxImage::from(other))
	}
}

impl From<&'_ ImageView<'_>> for Image {
	fn from(other: &ImageView) -> Self {
		Self::Box(BoxImage::from(other))
	}
}

impl From<BoxImage> for ArcImage {
	fn from(other: BoxImage) -> Self {
		Self {
			info: other.info,
			data: other.data.into(),
		}
	}
}

impl From<BoxImage> for Image {
	fn from(other: BoxImage) -> Self {
		Self::Box(other)
	}
}

impl From<ArcImage> for Image {
	fn from(other: ArcImage) -> Self {
		Self::Arc(other)
	}
}

impl From<Box<dyn AsImageView + Send>> for Image {
	fn from(other: Box<dyn AsImageView + Send>) -> Self {
		Self::BoxDyn(other)
	}
}

impl From<Arc<dyn AsImageView + Sync + Send>> for Image {
	fn from(other: Arc<dyn AsImageView + Sync + Send>) -> Self {
		Self::ArcDyn(other)
	}
}

impl<T> From<Box<T>> for Image
where
	T: AsImageView + Send + 'static,
{
	fn from(other: Box<T>) -> Self {
		Self::BoxDyn(other)
	}
}

impl<T> From<Arc<T>> for Image
where
	T: AsImageView + Send + Sync + 'static,
{
	fn from(other: Arc<T>) -> Self {
		Self::ArcDyn(other)
	}
}
