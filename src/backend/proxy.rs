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

/// Shorthand type alias for the correct `winit::event::EventLoopProxy`.
type EventLoopProxy<UserEvent> = winit::event_loop::EventLoopProxy<ContextEvent<UserEvent>>;

/// A proxy object to interact with the global context from a different thread.
pub struct ContextProxy<UserEvent: 'static> {
	event_loop: EventLoopProxy<UserEvent>,
}

/// A proxy object to interact with a window from a different thread.
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

/// An event that can be sent to the global context.
///
/// It can be either a [`ContextCommand`] or a user event.
pub enum ContextEvent<UserEvent: 'static> {
	ContextCommand(ContextCommand<UserEvent>),
	UserEvent(UserEvent),
}

/// A command that can be sent to the global context.
pub enum ContextCommand<UserEvent: 'static> {
	CreateWindow(CreateWindow),
	DestroyWindow(DestroyWindow),
	SetWindowVisible(SetWindowVisible),
	SetWindowImage(SetWindowImage),
	AddContextEventHandler(AddContextEventHandler<UserEvent>),
	ExecuteFunction(ExecuteFunction<UserEvent>),
}

impl<UserEvent> ContextProxy<UserEvent> {
	/// Wrap an [`EventLoopProxy`] in a [`ContextProxy`].
	pub(crate) fn new(event_loop: EventLoopProxy<UserEvent>) -> Self {
		Self { event_loop }
	}

	/// Create a new window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
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

	/// Destroy a window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn destroy_window(
		&self,
		window_id: WindowId,
	) -> Result<(), ProxyError<InvalidWindowIdError>> {
		let (result_tx, mut result_rx) = oneshot::channel();
		let command = DestroyWindow { window_id, result_tx };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)?;

		map_channel_error(result_rx.recv_timeout(Duration::from_secs(2)))
	}

	/// Make a window visiable or invsible.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
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

	/// Set the shown image for a window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn set_window_image(
		&self,
		window_id: WindowId,
		name: impl Into<String>,
		image: impl Into<Image<'static>>,
	) -> Result<(), ProxyError<InvalidWindowIdError>> {
		let name = name.into();
		let image = image.into();

		let (result_tx, mut result_rx) = oneshot::channel();
		let command = SetWindowImage { window_id, name, image, result_tx };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)?;

		map_channel_error(result_rx.recv_timeout(Duration::from_secs(2)))
	}

	/// Queue a function for execution in the context thread.
	///
	/// This function does not wait for the queued function to be executed.
	pub fn execute_function<F>(&self, function: F) -> Result<(), EventLoopClosedError>
	where
		F: 'static + FnOnce(ContextHandle<UserEvent>) + Send,
	{
		self.execute_boxed_function(Box::new(function))
	}


	/// Queue a boxed function for execution in the context thread.
	///
	/// This function does not wait for the queued function to be executed.
	///
	/// This does the same as [`Self::execute_function`],
	/// but doesn't add another layer of boxing if you already have a boxed function.
	pub fn execute_boxed_function(&self, function: Box<dyn FnOnce(ContextHandle<UserEvent>) + Send + 'static>) -> Result<(), EventLoopClosedError> {
		let command = ExecuteFunction { function };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)
	}

	/// Add a global event handler to the context.
	///
	/// Some events may still occur before the event handler is truly installed.
	pub fn add_event_handler<F>(&mut self, handler: F) -> Result<(), EventLoopClosedError>
	where
		F: FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + Send + 'static,
	{
		self.add_boxed_event_handler(Box::new(handler))
	}


	/// Add a global event handler to the context.
	///
	/// Some events may still occur before the event handler is truly installed.
	///
	/// This does the same as [`Self::add_event_handler`],
	/// but doesn't add another layer of boxing if you already have a boxed function.
	pub fn add_boxed_event_handler(
		&mut self,
		handler: Box<dyn FnMut(ContextHandle<UserEvent>, &mut Event<UserEvent>) -> EventHandlerOutput + Send + 'static>
	) -> Result<(), EventLoopClosedError> {
		let command = AddContextEventHandler { handler };
		self.event_loop.send_event(command.into()).map_err(|_| EventLoopClosedError)
	}

	/// Send a user event to the context.
	pub fn send_user_event(&self, event: UserEvent) -> Result<(), EventLoopClosedError> {
		self.event_loop.send_event(ContextEvent::UserEvent(event)).map_err(|_| EventLoopClosedError)
	}
}

/// Convert a `Result<Result<T, E>, oneshot::TryReceiveError>` to a `Result<T, ProxyError<E>>`.
fn map_channel_error<T, E>(result: Result<Result<T, E>, oneshot::TryReceiveError>) -> Result<T, ProxyError<E>> {
	result.map_err(|error| match error {
		oneshot::TryReceiveError::NotReady => ProxyError::Timeout(TimeoutError),
		oneshot::TryReceiveError::Disconnected => ProxyError::EventLoopClosed(EventLoopClosedError),
		oneshot::TryReceiveError::AlreadyRetrieved => unreachable!("oneshot result is already retrieved"),
	})?.map_err(ProxyError::Inner)
}

impl<UserEvent: 'static> WindowProxy<UserEvent> {
	/// Create a new window proxy from a context proxy and a window ID.
	pub fn new(window_id: WindowId, context_proxy: ContextProxy<UserEvent>) -> Self {
		Self { window_id, context_proxy }
	}

	/// Get the window ID.
	pub fn id(&self) -> WindowId {
		self.window_id
	}

	/// Get the context proxy of the window proxy.
	pub fn context_proxy(&self) -> &ContextProxy<UserEvent> {
		&self.context_proxy
	}

	/// Destroy the window.
	pub fn destroy(&self) -> Result<(), ProxyError<InvalidWindowIdError>> {
		self.context_proxy.destroy_window(self.window_id)
	}

	/// Set the image of the window.
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
