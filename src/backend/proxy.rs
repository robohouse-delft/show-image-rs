use crate::ContextHandle;
use crate::EventHandlerOutput;
use crate::event::Event;
use crate::Image;
use crate::WindowId;
use crate::WindowOptions;
use crate::error::EventLoopClosedError;
use crate::error::InvalidWindowIdError;
use crate::error::ProxyError;
use crate::error::TimeoutError;
use crate::oneshot;
use std::time::Duration;

type EventLoopProxy<UserEvent> = winit::event_loop::EventLoopProxy<ContextEvent<UserEvent>>;

pub struct ContextProxy<UserEvent: 'static> {
	event_loop: EventLoopProxy<UserEvent>,
}

#[derive(Clone)]
pub struct WindowProxy<UserEvent: 'static> {
	window_id: WindowId,
	context_proxy: ContextProxy<UserEvent>,
}

impl<UserEvent: 'static> Clone for ContextProxy<UserEvent> {
	fn clone(&self) -> Self {
		Self { event_loop: self.event_loop.clone() }
	}
}

pub enum ContextEvent<UserEvent: 'static> {
	ContextCommand(ContextCommand<UserEvent>),
	UserEvent(UserEvent),
}

pub enum ContextCommand<UserEvent: 'static> {
	CreateWindow(CreateWindow),
	DestroyWindow(DestroyWindow),
	SetWindowVisible(SetWindowVisible),
	SetWindowImage(SetWindowImage),
	AddContextEventHandler(AddContextEventHandler<UserEvent>),
	ExecuteFunction(ExecuteFunction<UserEvent>),
}

impl<UserEvent> ContextProxy<UserEvent> {
	pub(crate) fn new(event_loop: EventLoopProxy<UserEvent>) -> Self {
		Self { event_loop }
	}

	pub fn create_window(
		&self,
		title: impl Into<String>,
		options: WindowOptions,
	) -> Result<WindowProxy<UserEvent>, ProxyError<winit::error::OsError>> {
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
		image: Image<'static>,
	) -> Result<(), ProxyError<InvalidWindowIdError>> {
		let name = name.into();

		let (result_tx, mut result_rx) = oneshot::channel();
		let command = SetWindowImage { window_id, name, image, result_tx };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)?;

		map_channel_error(result_rx.recv_timeout(Duration::from_secs(2)))
	}

	pub fn execute_function<F>(&self, function: F) -> Result<(), EventLoopClosedError>
	where
		F: 'static + FnOnce(ContextHandle<UserEvent>) + Send,
	{
		self.execute_boxed_function(Box::new(function))
	}

	pub fn add_event_handler<F>(&mut self, handler: F) -> Result<(), EventLoopClosedError>
	where
		F: FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + Send + 'static,
	{
		self.add_boxed_event_handler(Box::new(handler))
	}

	pub fn add_boxed_event_handler(
		&mut self,
		handler: Box<dyn FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + Send + 'static>
	) -> Result<(), EventLoopClosedError> {
		let command = AddContextEventHandler { handler };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)
	}

	pub fn execute_boxed_function(&self, function: Box<dyn FnOnce(ContextHandle<UserEvent>) + Send + 'static>) -> Result<(), EventLoopClosedError> {
		let command = ExecuteFunction { function };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)
	}

	pub fn send_custom_event(&self, event: UserEvent) -> Result<(), EventLoopClosedError> {
		self.event_loop.send_event(ContextEvent::UserEvent(event)).map_err(|_| EventLoopClosedError)
	}
}

fn map_channel_error<T, E>(result: Result<Result<T, E>, oneshot::TryReceiveError>) -> Result<T, ProxyError<E>> {
	result.map_err(|error| match error {
		oneshot::TryReceiveError::NotReady => ProxyError::Timeout(TimeoutError),
		oneshot::TryReceiveError::Disconnected => ProxyError::EventLoopClosed(EventLoopClosedError),
		oneshot::TryReceiveError::AlreadyRetrieved => unreachable!("oneshot result is already retrieved"),
	})?.map_err(ProxyError::Inner)
}

impl<UserEvent: 'static> WindowProxy<UserEvent> {
	pub fn new(window_id: WindowId, context_proxy: ContextProxy<UserEvent>) -> Self {
		Self { window_id, context_proxy }
	}

	pub fn id(&self) -> WindowId {
		self.window_id
	}

	pub fn context_proxy(&self) -> &ContextProxy<UserEvent> {
		&self.context_proxy
	}

	pub fn destroy(&self) -> Result<(), ProxyError<InvalidWindowIdError>> {
		self.context_proxy.destroy_window(self.window_id)
	}

	pub fn set_image(
		&self,
		name: impl Into<String>,
		image: Image<'static>,
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
	pub image: Image<'static>,
	pub result_tx: oneshot::Sender<Result<(), InvalidWindowIdError>>
}

pub struct AddContextEventHandler<UserEvent: 'static> {
	pub handler: Box<dyn FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + Send + 'static>,
}

pub struct ExecuteFunction<UserEvent: 'static> {
	pub function: Box<dyn FnOnce(ContextHandle<UserEvent>) + Send>,
}

impl<UserEvent> From<ContextCommand<UserEvent>> for ContextEvent<UserEvent> {
	fn from(other: ContextCommand<UserEvent>) -> Self {
		Self::ContextCommand(other)
	}
}

impl<UserEvent> From<CreateWindow> for ContextEvent<UserEvent> {
	fn from(other: CreateWindow) -> Self {
		ContextCommand::CreateWindow(other).into()
	}
}

impl<UserEvent> From<DestroyWindow> for ContextEvent<UserEvent> {
	fn from(other: DestroyWindow) -> Self {
		ContextCommand::DestroyWindow(other).into()
	}
}

impl<UserEvent> From<SetWindowVisible> for ContextEvent<UserEvent> {
	fn from(other: SetWindowVisible) -> Self {
		ContextCommand::SetWindowVisible(other).into()
	}
}

impl<UserEvent> From<SetWindowImage> for ContextEvent<UserEvent> {
	fn from(other: SetWindowImage) -> Self {
		ContextCommand::SetWindowImage(other).into()
	}
}

impl<UserEvent> From<AddContextEventHandler<UserEvent>> for ContextEvent<UserEvent> {
	fn from(other: AddContextEventHandler<UserEvent>) -> Self {
		ContextCommand::AddContextEventHandler(other).into()
	}
}

impl<UserEvent> From<ExecuteFunction<UserEvent>> for ContextEvent<UserEvent> {
	fn from(other: ExecuteFunction<UserEvent>) -> Self {
		ContextCommand::ExecuteFunction(other).into()
	}
}
