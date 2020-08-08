use crate::Context;
use crate::InvalidWindowId;
use crate::oneshot;
use winit::window::WindowId;

pub enum ContextCommand<T> {
	CreateWindow(CreateWindow),
	DestroyWindow(DestroyWindow),
	SetWindowImage(SetWindowImage),
	RunContextFunction(RunContextFunction),
	Custom(T),
}

pub struct CreateWindow {
	pub title: String,
	pub preserve_aspect_ratio: bool,
	pub result_tx: oneshot::Sender<Result<WindowId, winit::error::OsError>>

}

pub struct DestroyWindow {
	pub window_id: WindowId,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowId>>
}

pub struct SetWindowImage {
	pub window_id: WindowId,
	pub name: String,
	pub image: image::DynamicImage,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowId>>
}

pub struct RunContextFunction {
	pub function: Box<dyn FnOnce(&Context) + Send>,
}

impl<T> From<CreateWindow> for ContextCommand<T> {
	fn from(other: CreateWindow) -> Self {
		Self::CreateWindow(other)
	}
}

impl<T> From<DestroyWindow> for ContextCommand<T> {
	fn from(other: DestroyWindow) -> Self {
		Self::DestroyWindow(other)
	}
}

impl<T> From<SetWindowImage> for ContextCommand<T> {
	fn from(other: SetWindowImage) -> Self {
		Self::SetWindowImage(other)
	}
}

impl<T> From<RunContextFunction> for ContextCommand<T> {
	fn from(other: RunContextFunction) -> Self {
		Self::RunContextFunction(other)
	}
}
