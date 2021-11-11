use crate::ContextHandle;
use crate::Image;
use crate::WindowHandle;
use crate::WindowId;
use crate::error::{InvalidWindowId, SetImageError};
use crate::event::Event;
use crate::event::EventHandlerControlFlow;
use crate::event::WindowEvent;
use crate::oneshot;

use std::sync::mpsc;

/// Proxy object to interact with a window from a user thread.
///
/// The proxy object only exposes a small subset of the functionality of a window.
/// However, you can use [`run_function()`][Self::run_function]
/// to get access to the underlying [`WindowHandle`] from the context thread.
/// With [`run_function_wait()`][Self::run_function_wait`] you can also get the return value of the function back:
///
/// ```no_run
/// # fn foo(window_proxy: show_image::WindowProxy) -> Result<(), show_image::error::InvalidWindowId> {
/// let inner_size = window_proxy.run_function_wait(|window| window.inner_size())?;
/// # Ok(())
/// # }
/// ```
///
/// You should not use proxy objects from withing the global context thread.
/// The proxy objects often wait for the global context to perform some action.
/// Doing so from within the global context thread would cause a deadlock.
#[derive(Clone)]
pub struct WindowProxy {
	window_id: WindowId,
	context_proxy: ContextProxy,
}

/// Proxy object to interact with the global context from a user thread.
///
/// You should not use proxy objects from withing the global context thread.
/// The proxy objects often wait for the global context to perform some action.
/// Doing so from within the global context thread would cause a deadlock.
#[derive(Clone)]
pub struct ContextProxy {
	event_loop: EventLoopProxy,
	context_thread: std::thread::ThreadId,
}

/// Dynamic function that can be run by the global context.
pub type ContextFunction = Box<dyn FnOnce(&mut ContextHandle) + Send>;

/// Internal shorthand for the correct `winit::event::EventLoopProxy`.
///
/// Not for use in public APIs.
type EventLoopProxy = winit::event_loop::EventLoopProxy<ContextFunction>;

impl ContextProxy {
	/// Wrap an [`EventLoopProxy`] in a [`ContextProxy`].
	pub(crate) fn new(event_loop: EventLoopProxy, context_thread: std::thread::ThreadId) -> Self {
		Self {
			event_loop,
			context_thread,
		}
	}

	/// Add a global event handler to the context.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`Self::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`Self::run_function`] to post a lambda that adds an error handler instead.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn add_event_handler<F>(&self, handler: F)
	where
		F: FnMut(&mut ContextHandle, &mut Event, &mut EventHandlerControlFlow) + Send + 'static,
	{
		self.run_function_wait(move |context| context.add_event_handler(handler))
	}

	/// Add an event handler for a specific window.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`Self::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`Self::run_function`] to post a lambda that adds an error handler instead.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn add_window_event_handler<F>(&self, window_id: WindowId, handler: F) -> Result<(), InvalidWindowId>
	where
		F: FnMut(WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow) + Send + 'static,
	{
		self.run_function_wait(move |context| {
			let mut window = context.window(window_id)?;
			window.add_event_handler(handler);
			Ok(())
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
	/// Consider using [`Self::run_background_task`] for long blocking tasks instead.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn run_function<F>(&self, function: F)
	where
		F: 'static + FnOnce(&mut ContextHandle) + Send,
	{
		let function = Box::new(function);
		if self.event_loop.send_event(function).is_err() {
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
	/// Consider using [`Self::run_background_task`] for long blocking tasks instead.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn run_function_wait<F, T>(&self, function: F) -> T
	where
		F: FnOnce(&mut ContextHandle) -> T + Send + 'static,
		T: Send + 'static,
	{
		self.assert_thread();

		let (result_tx, result_rx) = oneshot::channel();
		self.run_function(move |context| result_tx.send((function)(context)));
		result_rx.recv()
			.expect("global context failed to send function return value back, which can only happen if the event loop stopped, but that should also kill the process")
	}

	/// Run a task in a background thread and register it with the context.
	///
	/// The task will be executed in a different thread than the context.
	/// Currently, each task is spawned in a separate thread.
	/// In the future, tasks may be run in a dedicated thread pool.
	///
	/// The background task will be joined before the process is terminated when you use [`Self::exit()`] or one of the other exit functions of this crate.
	pub fn run_background_task<F>(&self, task: F)
	where
		F: FnOnce() + Send + 'static,
	{
		self.run_function(move |context| {
			context.run_background_task(task);
		});
	}

	/// Create a channel that receives events from the context.
	///
	/// To close the channel, simply drop de receiver.
	///
	/// *Warning:*
	/// The created channel blocks when you request an event until one is available.
	/// You should never use the receiver from within an event handler or a function posted to the global context thread.
	/// Doing so would cause a deadlock.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn event_channel(&self) -> mpsc::Receiver<Event> {
		let (tx, rx) = mpsc::channel();
		self.add_event_handler(move |_context, event, control| {
			// If the receiver is dropped, remove the handler.
			if tx.send(event.clone()).is_err() {
				control.remove_handler = true;
			}
		});

		rx
	}

	/// Create a channel that receives events from a window.
	///
	/// To close the channel, simply drop de receiver.
	/// The channel is closed automatically when the window is destroyed.
	///
	/// *Warning:*
	/// The created channel blocks when you request an event until one is available.
	/// You should never use the receiver from within an event handler or a function posted to the global context thread.
	/// Doing so would cause a deadlock.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn window_event_channel(&self, window_id: WindowId) -> Result<mpsc::Receiver<WindowEvent>, InvalidWindowId> {
		let (tx, rx) = mpsc::channel();
		self.add_window_event_handler(window_id, move |_window, event, control| {
			// If the receiver is dropped, remove the handler.
			if tx.send(event.clone()).is_err() {
				control.remove_handler = true;
			}
		})?;
		Ok(rx)
	}

	/// Join all background tasks and then exit the process.
	///
	/// If you use [`std::process::exit`], running background tasks may be killed.
	/// To ensure no data loss occurs, you should use this function instead.
	///
	/// Background tasks are spawned when an image is saved through the built-in Ctrl+S or Ctrl+Shift+S shortcut, or by user code.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn exit(&self, code: i32) -> ! {
		self.assert_thread();
		self.run_function(move |context| context.exit(code));
		loop {
			std::thread::park();
		}
	}

	/// Check that the current thread is not running the context event loop.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	#[track_caller]
	fn assert_thread(&self) {
		if std::thread::current().id() == self.context_thread {
			panic!("ContextProxy used from within the context thread, which would cause a deadlock. Use ContextHandle instead.");
		}
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

	/// Set the displayed image of the window.
	///
	/// The real work is done in the context thread.
	/// This function blocks until the context thread has performed the action.
	///
	/// Note that you can not change the overlays with this function.
	/// To modify those, you can use [`Self::run_function`] or [`Self::run_function_wait`]
	/// to get access to the [`WindowHandle`].
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn set_image(&self, name: impl Into<String>, image: impl Into<Image>) -> Result<(), SetImageError> {
		let name = name.into();
		let image = image.into();
		self.run_function_wait(move |mut window| -> Result<(), SetImageError> {
			window.set_image(name, &image.as_image_view()?);
			Ok(())
		})?
	}

	/// Add an event handler for the window.
	///
	/// Events that are already queued with the event loop will not be passed to the handler.
	///
	/// This function uses [`ContextProxy::run_function_wait`] internally, so it blocks until the event handler is added.
	/// To avoid blocking, you can use [`ContextProxy::run_function`] to post a lambda that adds an event handler instead.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn add_event_handler<F>(&self, handler: F) -> Result<(), InvalidWindowId>
	where
		F: FnMut(WindowHandle, &mut WindowEvent, &mut EventHandlerControlFlow) + Send + 'static,
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
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn event_channel(&self) -> Result<mpsc::Receiver<WindowEvent>, InvalidWindowId> {
		self.context_proxy.window_event_channel(self.window_id)
	}

	/// Wait for the window to be destroyed.
	///
	/// This can happen if the application code destroys the window or if the user closes the window.
	///
	/// *Warning:*
	/// This function blocks until the window is closed.
	/// You should never use this function from within an event handler or a function posted to the global context thread.
	/// Doing so would cause a deadlock.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn wait_until_destroyed(&self) -> Result<(), InvalidWindowId> {
		let (tx, rx) = oneshot::channel::<()>();
		self.add_event_handler(move |_window, _event, _control| {
			// Need to mention the tx half so it gets moved into the closure.
			let _tx = &tx;
		})?;

		// We actually want to wait for the transmit handle to be dropped, so ignore receive errors.
		let _ = rx.recv();
		Ok(())
	}

	/// Post a function for execution in the context thread without waiting for it to execute.
	///
	/// This function returns immediately, without waiting for the posted function to start or complete.
	/// If you want to get a return value back from the function, use [`Self::run_function_wait`] instead.
	///
	/// *Note:*
	/// You should not use this to post functions that block for a long time.
	/// Doing so will block the event loop and will make the windows unresponsive until the event loop can continue.
	/// Consider using [`self.context_proxy().run_background_task(...)`][ContextProxy::run_background_task] for long blocking tasks instead.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn run_function<F>(&self, function: F)
	where
		F: 'static + FnOnce(WindowHandle) + Send,
	{
		let window_id = self.window_id;
		self.context_proxy.run_function(move |context| {
			if let Ok(window) = context.window(window_id) {
				function(window);
			}
		})
	}

	/// Post a function for execution in the context thread and wait for the return value.
	///
	/// If you do not need a return value from the posted function,
	/// you can use [`Self::run_function`] to avoid blocking the calling thread until it completes.
	///
	/// *Note:*
	/// You should not use this to post functions that block for a long time.
	/// Doing so will block the event loop and will make the windows unresponsive until the event loop can continue.
	/// Consider using [`self.context_proxy().run_background_task(...)`][ContextProxy::run_background_task] for long blocking tasks instead.
	///
	/// # Panics
	/// This function will panic if called from within the context thread.
	pub fn run_function_wait<F, T>(&self, function: F) -> Result<T, InvalidWindowId>
	where
		F: FnOnce(WindowHandle) -> T + Send + 'static,
		T: Send + 'static,
	{
		let window_id = self.window_id;
		self.context_proxy.run_function_wait(move |context| {
			let window = context.window(window_id)?;
			Ok(function(window))
		})
	}
}
