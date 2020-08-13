use crate::ContextHandle;
use crate::WindowOptions;
use crate::error::EventLoopClosedError;
use crate::error::InvalidWindowIdError;
use crate::error::ProxyError;
use crate::error::TimeoutError;
use crate::oneshot;
use std::time::Duration;
use winit::event_loop::EventLoopProxy;
use winit::window::WindowId;

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
	SetWindowVisible(SetWindowVisible),
	SetWindowImage(SetWindowImage),
	ExecuteFunction(ExecuteFunction<CustomEvent>),
	Custom(CustomEvent),
}

impl<CustomEvent> ContextProxy<CustomEvent> {
	pub(crate) fn new(event_loop: EventLoopProxy<ContextCommand<CustomEvent>>) -> Self {
		Self { event_loop }
	}

	pub fn create_window(
		&self,
		title: impl Into<String>,
	) -> Result<WindowProxy<CustomEvent>, ProxyError<winit::error::OsError>> {
		self.create_window_with_options(title, WindowOptions {
			preserve_aspect_ratio: true,
		})
	}

	pub fn create_window_with_options(
		&self,
		title: impl Into<String>,
		options: WindowOptions,
	) -> Result<WindowProxy<CustomEvent>, ProxyError<winit::error::OsError>> {
		let title = title.into();

		let (result_tx, mut result_rx) = oneshot::channel();
		let command = CreateWindow { title, options, result_tx };
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

	pub fn set_window_visible(
		&self,
		window_id: WindowId,
		visible: bool,
	) -> Result<(), ProxyError<InvalidWindowIdError>> {
		let (result_tx, mut result_rx) = oneshot::channel();
		let command = SetWindowVisible { window_id, visible, result_tx };
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

	pub fn execute_function<F>(&self, function: F) -> Result<(), EventLoopClosedError>
	where
		F: 'static + FnOnce(ContextHandle<CustomEvent>) + Send,
	{
		self.execute_boxed_function(Box::new(function))
	}

	pub fn execute_boxed_function(&self, function: Box<dyn FnOnce(ContextHandle<CustomEvent>) + Send + 'static>) -> Result<(), EventLoopClosedError> {
		let command = ExecuteFunction { function };
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
	pub options: crate::WindowOptions,
	pub result_tx: oneshot::Sender<Result<WindowId, winit::error::OsError>>

}

pub struct DestroyWindow {
	pub window_id: WindowId,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowIdError>>
}

pub struct SetWindowVisible {
	pub window_id: WindowId,
	pub visible: bool,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowIdError>>
}

pub struct SetWindowImage {
	pub window_id: WindowId,
	pub name: String,
	pub image: image::DynamicImage,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowIdError>>
}

pub struct ExecuteFunction<CustomEvent: 'static> {
	pub function: Box<dyn FnOnce(ContextHandle<CustomEvent>) + Send>,
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

impl<CustomEvent> From<SetWindowVisible> for ContextCommand<CustomEvent> {
	fn from(other: SetWindowVisible) -> Self {
		Self::SetWindowVisible(other)
	}
}

impl<CustomEvent> From<SetWindowImage> for ContextCommand<CustomEvent> {
	fn from(other: SetWindowImage) -> Self {
		Self::SetWindowImage(other)
	}
}

impl<CustomEvent> From<ExecuteFunction<CustomEvent>> for ContextCommand<CustomEvent> {
	fn from(other: ExecuteFunction<CustomEvent>) -> Self {
		Self::ExecuteFunction(other)
	}
}
