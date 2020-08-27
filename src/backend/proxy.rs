use crate::ContextHandle;
use crate::EventHandlerControlFlow;
use crate::Image;
use crate::WindowHandle;
use crate::WindowId;
use crate::WindowOptions;
use crate::error::CreateWindowError;
use crate::error::InvalidWindowIdError;
use crate::error::SetImageError;
use crate::event::WindowEvent;
use crate::oneshot;

use std::sync::mpsc;

/// Proxy object to interact with the global context from a user thread.
///
/// You should not use proxy objects from withing the global context thread.
/// The proxy objects often wait for the global context to perform some action.
/// Doing so from within the global context thread would cause a deadlock.
#[derive(Clone)]
pub struct ContextProxy {
	event_loop: EventLoopProxy,
}

/// Proxy object to interact with a window from a user thread.
///
/// You should not use proxy objects from withing the global context thread.
/// The proxy objects often wait for the global context to perform some action.
/// Doing so from within the global context thread would cause a deadlock.
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

	/// Exit the program when the last window closes.
	pub fn set_exit_with_last_window(&self, exit_with_last_window: bool) {
		self.run_function(move |context| {
			context.set_exit_with_last_window(exit_with_last_window);
		})
	}

	/// Create a new window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn create_window(
		&self,
		title: impl Into<String>,
		options: WindowOptions,
	) -> Result<WindowProxy, CreateWindowError> {
		let title = title.into();
		let window_id = self.run_function_wait(move |context| {
			context.create_window(title, options)
				.map(|window| window.id())
		})?;

		Ok(WindowProxy::new(window_id, self.clone()))
	}

	/// Destroy a window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn destroy_window(
		&self,
		window_id: WindowId,
	) -> Result<(), InvalidWindowIdError> {
		self.run_function_wait(move |context| {
			context.destroy_window(window_id)
		})
	}

	/// Make a window visible or invisible.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	pub fn set_window_visible(
		&self,
		window_id: WindowId,
		visible: bool,
	) -> Result<(), InvalidWindowIdError> {
		self.run_function_wait(move |context| {
			context.set_window_visible(window_id, visible)
		})
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
	) -> Result<(), SetImageError> {
		let name = name.into();
		let image = image.into();
		self.run_function_wait(move |context| {
			context.set_window_image(window_id, &name, &image)
		})
	}

	/// Add a global event handler to the context.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`Self::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`Self::run_function`] to post a lambda that adds an error handler instead.
	pub fn add_event_handler<F>(&self, handler: F)
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
	pub fn add_window_event_handler<F>(&self, window_id: WindowId, handler: F) -> Result<(), InvalidWindowIdError>
	where
		F: FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow) + Send + 'static,
	{
		self.run_function_wait(move |context| {
			context.add_window_event_handler(window_id, handler)
		})
	}

	/// Post a function for execution in the context thread without waiting for it to execute.
	///
	/// This function returns immediately, without waiting for the posted function to start or complete.
	/// If you want to get a return value back from the function, use [`Self::run_function_wait`] instead.
	///
	/// *Note:*
	/// You should not post functions to the context thread that block for a long time.
	/// Doing so will block the event loop and will make the windows unresponsive until the event loop can continue.
	pub fn run_function<F>(&self, function: F)
	where
		F: 'static + FnOnce(&mut ContextHandle) + Send,
	{
		let function = Box::new(function);
		if let Err(_) = self.event_loop.send_event(function) {
			panic!("global context stopped running but somehow the process is still alive");
		}
	}

	/// Post a function for execution in the context thread and wait for the return value.
	///
	/// If you do not need a return value from the posted function,
	/// you can use [`Self::run_function`] to avoid blocking the calling thread until it completes.
	///
	/// *Note:*
	/// You should not post functions to the context thread that block for a long time.
	/// Doing so will block the event loop and will make the windows unresponsive until the event loop can continue.
	pub fn run_function_wait<F, T>(&self, function: F) -> T
	where
		F: FnOnce(&mut ContextHandle) -> T + Send + 'static,
		T: Send + 'static,
	{
		let (result_tx, result_rx) = oneshot::channel();
		self.run_function(move |context| {
			result_tx.send((function)(context))
		});
		result_rx.recv()
			.expect("global context failed to send function return value back, which can only happen if the event loop stopped, but that should also kill the process")
	}

	/// Create a channel that receives events from the context.
	///
	/// To close the channel, simply drop de receiver.
	///
	/// *Warning:*
	/// The created channel blocks when you request an event until one is available.
	/// You should never use the receiver from within an event handler or a function posted to the global context thread.
	/// Doing so would cause a deadlock.
	pub fn event_channel(&self) -> mpsc::Receiver<crate::Event<'static>> {
		let (tx, rx) = mpsc::channel();
		self.add_event_handler(move |_context, event, control| {
			// Filter out non-static events.
			if let Some(event) = super::event::clone_static_event(event) {
				// If the receiver is dropped, remove the handler.
				if let Err(_) = tx.send(event) {
					control.remove_handler = true;
				}
			}
		});
		rx
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
	pub fn destroy(&self) -> Result<(), InvalidWindowIdError> {
		self.context_proxy.destroy_window(self.window_id)
	}

	/// Set the image of the window.
	pub fn set_visible(
		&self,
		visible: bool,
	) -> Result<(), InvalidWindowIdError> {
		self.context_proxy.set_window_visible(self.window_id, visible)
	}

	/// Set the image of the window.
	pub fn set_image(
		&self,
		name: impl Into<String>,
		image: impl Into<Image>,
	) -> Result<(), SetImageError> {
		self.context_proxy.set_window_image(self.window_id, name, image)
	}

	/// Add an event handler for the window.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`ContextProxy::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`ContextProxy::run_function`] to post a lambda that adds an event handler instead.
	pub fn add_event_handler<F>(&self, handler: F) -> Result<(), InvalidWindowIdError>
	where
		F: FnMut(&mut WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow) + Send + 'static,
	{
		self.context_proxy.add_window_event_handler(self.window_id, handler)
	}

	/// Create a channel that receives events from the window.
	///
	/// To close the channel, simply drop de receiver.
	/// The channel is closed automatically when the window is destroyed.
	///
	/// *Warning:*
	/// The created channel blocks when you request an event until one is available.
	/// You should never use the receiver from within an event handler or a function posted to the global context thread.
	/// Doing so would cause a deadlock.
	pub fn event_channel(&self) -> Result<mpsc::Receiver<crate::event::WindowEvent<'static>>, InvalidWindowIdError> {
		let (tx, rx) = mpsc::channel();
		self.add_event_handler(move |_window, event, control| {
			// Filter out non-static events.
			if let Some(event) = super::event::clone_static_window_event(event) {
				// If the receiver is dropped, remove the handler.
				if let Err(_) = tx.send(event) {
					control.remove_handler = true;
				}
			}
		})?;
		Ok(rx)
	}

	/// Wait for the window to be destroyed.
	///
	/// This can happen if the application code destroys the window or if the user closes the window.
	///
	/// *Warning:*
	/// This function blocks until the window is closed.
	/// You should never use this function from within an event handler or a function posted to the global context thread.
	/// Doing so would cause a deadlock.
	pub fn wait_until_destroyed(&self) -> Result<(), InvalidWindowIdError> {
		let (tx, rx) = oneshot::channel::<()>();
		self.add_event_handler(move |_window, _event, _control| {
			// Need to mention the tx half so it gets moved into the closure.
			let _tx = &tx;
		})?;

		// We actually want to wait for the transmit handle to be dropped, so ignore receive errors.
		let _ = rx.recv();
		Ok(())
	}
}
