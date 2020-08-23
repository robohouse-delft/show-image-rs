use crate::ContextHandle;
use crate::EventHandlerControlFlow;
use crate::Image;
use crate::WindowHandle;
use crate::WindowId;
use crate::WindowOptions;
use crate::error::EventLoopClosedError;
use crate::error::ProxyCreateWindowError;
use crate::error::ProxySetImageError;
use crate::error::ProxyWindowOperationError;
use crate::event::WindowEvent;
use crate::oneshot;

/// A proxy object to interact with the global context from a different thread.
#[derive(Clone)]
pub struct ContextProxy {
	event_loop: EventLoopProxy,
}

/// A proxy object to interact with a window from a different thread.
#[derive(Clone)]
pub struct WindowProxy {
	window_id: WindowId,
	context_proxy: ContextProxy,
}

/// A dynamic function that can be run by the global context.
pub type ContextFunction = Box<dyn FnOnce(&mut ContextHandle) + Send>;

/// Internal shorthand for the correct `winit::event::EventLoopProxy`.
///
/// Not for use in public APIs.
type EventLoopProxy = winit::event_loop::EventLoopProxy<ContextFunction>;

impl ContextProxy {
	/// Wrap an [`EventLoopProxy`] in a [`ContextProxy`].
	pub(crate) fn new(event_loop: EventLoopProxy) -> Self {
		Self { event_loop }
	}

	/// Exit the application with the given status code as soon as possible.
	///
	/// The actual exit will be performed after queued events have been processed.
	/// This allows all queued actions to be performed before the exit happends.
	///
	/// If a non-zero exit status has already been set,
	/// the new exit status is ignored and the program will exit with the previously set status code.
	pub fn exit(&self, status: i32) -> Result<(), EventLoopClosedError> {
		self.run_function(move |context| context.exit(status))
	}

	/// Create a new window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn create_window(
		&self,
		title: impl Into<String>,
		options: WindowOptions,
	) -> Result<WindowProxy, ProxyCreateWindowError> {
		let title = title.into();
		let window_id = self.run_function_wait(move |context| {
			context.create_window(title, options)
				.map(|window| window.id())
		})??;

		Ok(WindowProxy::new(window_id, self.clone()))
	}

	/// Destroy a window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn destroy_window(
		&self,
		window_id: WindowId,
	) -> Result<(), ProxyWindowOperationError> {
		self.run_function_wait(move |context| {
			context.destroy_window(window_id)
		})??;
		Ok(())
	}

	/// Make a window visiable or invsible.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn set_window_visible(
		&self,
		window_id: WindowId,
		visible: bool,
	) -> Result<(), ProxyWindowOperationError> {
		self.run_function_wait(move |context| {
			context.set_window_visible(window_id, visible)
		})??;
		Ok(())
	}

	/// Set the shown image for a window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn set_window_image(
		&self,
		window_id: WindowId,
		name: impl Into<String>,
		image: impl Into<Image>,
	) -> Result<(), ProxySetImageError> {
		let name = name.into();
		let image = image.into();
		self.run_function_wait(move |context| {
			context.set_window_image(window_id, &name, &image)
		})??;
		Ok(())
	}

	/// Add a global event handler to the context.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`Self::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`Self::run_function`] to post a lambda that adds an error handler instead.
	pub fn add_event_handler<F>(&mut self, handler: F) -> Result<(), EventLoopClosedError>
	where
		F: FnMut(&mut ContextHandle, &mut crate::Event, &mut EventHandlerControlFlow) + Send + 'static,
	{
		self.run_function_wait(move |context| {
			context.add_event_handler(handler)
		})
	}

	/// Add an event handler for a specific window.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`Self::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`Self::run_function`] to post a lambda that adds an error handler instead.
	pub fn add_window_event_handler<F>(&mut self, window_id: WindowId, handler: F) -> Result<(), ProxyWindowOperationError>
	where
		F: FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow) + Send + 'static,
	{
		self.run_function_wait(move |context| {
			context.add_window_event_handler(window_id, handler)
		})??;
		Ok(())
	}

	/// Post a function for execution in the context thread without waiting for it to execute.
	///
	/// This function returns immediately, without waiting for the posted function to start or complete.
	/// If you want to get a return value back from the function, use [`Self::run_function_wait`] instead.
	///
	/// *Note:*
	/// You should not post functions to the context thread that block for a long time.
	/// Doing so will block the event loop and will make the windows unresponsive until the event loop can continue.
	pub fn run_function<F>(&self, function: F) -> Result<(), EventLoopClosedError>
	where
		F: 'static + FnOnce(&mut ContextHandle) + Send,
	{
		let function = Box::new(function);
		self.event_loop.send_event(function).map_err(|_| EventLoopClosedError)
	}

	/// Post a function for execution in the context thread and wait for the return value.
	///
	/// If you do not need a return value from the posted function,
	/// you can use [`Self::run_function`] to avoid blocking it completes.
	///
	/// *Note:*
	/// You should not post functions to the context thread that block for a long time.
	/// Doing so will block the event loop and will make the windows unresponsive until the event loop can continue.
	pub fn run_function_wait<F, T>(&self, function: F) -> Result<T, EventLoopClosedError>
	where
		F: FnOnce(&mut ContextHandle) -> T + Send + 'static,
		T: Send + 'static,
	{
		let (result_tx, result_rx) = oneshot::channel();
		self.run_function(move |context| {
			result_tx.send((function)(context))
		})?;
		result_rx.recv().map_err(|_| EventLoopClosedError)
	}
}

impl WindowProxy {
	/// Create a new window proxy from a context proxy and a window ID.
	pub fn new(window_id: WindowId, context_proxy: ContextProxy) -> Self {
		Self { window_id, context_proxy }
	}

	/// Get the window ID.
	pub fn id(&self) -> WindowId {
		self.window_id
	}

	/// Get the context proxy of the window proxy.
	pub fn context_proxy(&self) -> &ContextProxy {
		&self.context_proxy
	}

	/// Destroy the window.
	pub fn destroy(&self) -> Result<(), ProxyWindowOperationError> {
		self.context_proxy.destroy_window(self.window_id)
	}

	/// Set the image of the window.
	pub fn set_visible(
		&self,
		visible: bool,
	) -> Result<(), ProxyWindowOperationError> {
		self.context_proxy.set_window_visible(self.window_id, visible)
	}

	/// Set the image of the window.
	pub fn set_image(
		&self,
		name: impl Into<String>,
		image: impl Into<Image>,
	) -> Result<(), ProxySetImageError> {
		self.context_proxy.set_window_image(self.window_id, name, image)
	}

	/// Add an event handler for a specific window.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`ContextHandle::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`ContextHandle::run_function`] to post a lambda that adds an error handler instead.
	pub fn add_window_event_handler<F>(&mut self, handler: F) -> Result<(), ProxyWindowOperationError>
	where
		F: FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow) + Send + 'static,
	{
		self.context_proxy.add_window_event_handler(self.window_id, handler)
	}
}
