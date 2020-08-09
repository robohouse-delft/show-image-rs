use crate::Context;
use crate::error::EventLoopClosedError;
use crate::error::InvalidWindowIdError;
use crate::error::ProxyError;
use crate::error::TimeoutError;
use crate::oneshot;
use std::time::Duration;
use winit::window::WindowId;
use winit::event_loop::EventLoopProxy;

pub struct ContextProxy<CustomEvent: 'static> {
	event_loop: EventLoopProxy<ContextCommand<CustomEvent>>,
}

#[derive(Clone)]
pub struct WindowProxy<CustomEvent: 'static> {
	window_id: WindowId,
	context_proxy: ContextProxy<CustomEvent>,
}

impl<CustomEvent: 'static> Clone for ContextProxy<CustomEvent> {
	fn clone(&self) -> Self {
		Self { event_loop: self.event_loop.clone() }
	}
}

pub enum ContextCommand<CustomEvent: 'static> {
	CreateWindow(CreateWindow),
	DestroyWindow(DestroyWindow),
	SetWindowImage(SetWindowImage),
	RunFunction(RunFunction<CustomEvent>),
	Custom(CustomEvent),
}

impl<CustomEvent> ContextProxy<CustomEvent> {
	pub(crate) fn new(event_loop: EventLoopProxy<ContextCommand<CustomEvent>>) -> Self {
		Self { event_loop }
	}

	pub fn create_window(
		&self,
		title: impl Into<String>,
		preserve_aspect_ratio: bool,
	) -> Result<WindowProxy<CustomEvent>, ProxyError<winit::error::OsError>> {
		let title = title.into();

		let (result_tx, mut result_rx) = oneshot::channel();
		let command = CreateWindow { title, preserve_aspect_ratio, result_tx };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)?;

		let window_id = map_channel_error(result_rx.recv_timeout(Duration::from_secs(2)))?;

		Ok(WindowProxy {
			window_id,
			context_proxy: self.clone(),
		})
	}

	pub fn destroy_window(
		&self,
		window_id: WindowId,
	) -> Result<(), ProxyError<InvalidWindowIdError>> {
		let (result_tx, mut result_rx) = oneshot::channel();
		let command = DestroyWindow { window_id, result_tx };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)?;

		map_channel_error(result_rx.recv_timeout(Duration::from_secs(2)))
	}

	pub fn set_window_image(
		&self,
		window_id: WindowId,
		name: impl Into<String>,
		image: image::DynamicImage,
	) -> Result<(), ProxyError<InvalidWindowIdError>> {
		let name = name.into();

		let (result_tx, mut result_rx) = oneshot::channel();
		let command = SetWindowImage { window_id, name, image, result_tx };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)?;

		map_channel_error(result_rx.recv_timeout(Duration::from_secs(2)))
	}

	pub fn run<F>(&self, function: F) -> Result<(), EventLoopClosedError>
	where
		F: Into<Box<dyn 'static + FnOnce(&mut Context<CustomEvent>) + Send>>,
	{
		let function = function.into();
		let command = RunFunction { function };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)
	}

	pub fn send_custom_event(&self, event: CustomEvent) -> Result<(), EventLoopClosedError> {
		self.event_loop.send_event(ContextCommand::Custom(event)).map_err(|_| EventLoopClosedError)
	}
}

fn map_channel_error<T, E>(result: Result<Result<T, E>, oneshot::TryReceiveError>) -> Result<T, ProxyError<E>> {
	result.map_err(|error| match error {
		oneshot::TryReceiveError::NotReady => ProxyError::Timeout(TimeoutError),
		oneshot::TryReceiveError::Disconnected => ProxyError::EventLoopClosed(EventLoopClosedError),
		oneshot::TryReceiveError::AlreadyRetrieved => unreachable!("oneshot result is already retrieved"),
	})?.map_err(ProxyError::Inner)
}

impl<CustomEvent: 'static> WindowProxy<CustomEvent> {
	pub fn id(&self) -> WindowId {
		self.window_id
	}

	pub fn context_proxy(&self) -> &ContextProxy<CustomEvent> {
		&self.context_proxy
	}

	pub fn destroy(&self) -> Result<(), ProxyError<InvalidWindowIdError>> {
		self.context_proxy.destroy_window(self.window_id)
	}

	pub fn set_image(
		&self,
		name: impl Into<String>,
		image: image::DynamicImage,
	) -> Result<(), ProxyError<InvalidWindowIdError>> {
		self.context_proxy.set_window_image(self.window_id, name, image)
	}
}

pub struct CreateWindow {
	pub title: String,
	pub preserve_aspect_ratio: bool,
	pub result_tx: oneshot::Sender<Result<WindowId, winit::error::OsError>>

}

pub struct DestroyWindow {
	pub window_id: WindowId,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowIdError>>
}

pub struct SetWindowImage {
	pub window_id: WindowId,
	pub name: String,
	pub image: image::DynamicImage,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowIdError>>
}

pub struct RunFunction<CustomEvent: 'static> {
	pub function: Box<dyn FnOnce(&mut Context<CustomEvent>) + Send>,
}

impl<CustomEvent> From<CreateWindow> for ContextCommand<CustomEvent> {
	fn from(other: CreateWindow) -> Self {
		Self::CreateWindow(other)
	}
}

impl<CustomEvent> From<DestroyWindow> for ContextCommand<CustomEvent> {
	fn from(other: DestroyWindow) -> Self {
		Self::DestroyWindow(other)
	}
}

impl<CustomEvent> From<SetWindowImage> for ContextCommand<CustomEvent> {
	fn from(other: SetWindowImage) -> Self {
		Self::SetWindowImage(other)
	}
}

impl<CustomEvent> From<RunFunction<CustomEvent>> for ContextCommand<CustomEvent> {
	fn from(other: RunFunction<CustomEvent>) -> Self {
		Self::RunFunction(other)
	}
}
