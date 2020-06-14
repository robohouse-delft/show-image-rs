use crate::background_thread::BackgroundThread;
use crate::Event;
use crate::Image;
use super::WindowInner;

/// A event handler.
pub type EventHandler = Box<dyn FnMut(&mut EventHandlerContext) + Send>;

/// The context for a registered event handler.
pub struct EventHandlerContext<'a> {
	/// The vector to add spawned tasks too.
	background_tasks: &'a mut Vec<BackgroundThread<()>>,

	/// Flag to indicate if the event should be passed to other handlers.
	stop_propagation: bool,

	/// Flag to indicate the handler should be removed.
	remove_handler: bool,

	/// The event to be handled.
	event: &'a Event,

	/// The window that triggered the event.
	window: &'a mut WindowInner,
}

impl<'a> EventHandlerContext<'a> {
	pub(crate) fn new(
		background_tasks: &'a mut Vec<BackgroundThread<()>>,
		event: &'a Event,
		window: &'a mut WindowInner,
	) -> Self {
		Self {
			background_tasks,
			stop_propagation: false,
			remove_handler: false,
			event,
			window,
		}
	}

	/// Stop propagation of the event to other handlers.
	pub fn stop_propagation(&mut self) {
		self.stop_propagation = true;
	}

	/// Check if we should stop propagation of the event.
	pub(crate) fn should_stop_propagation(&self) -> bool {
		self.stop_propagation
	}

	/// Remove the event handler after it returns.
	pub fn remove_handler(&mut self) {
		self.remove_handler = true;
	}

	/// Check if we should remove the event handler after it returns.
	pub(crate) fn should_remove_handler(&self) -> bool {
		self.stop_propagation
	}

	/// Get the event.
	pub fn event(&self) -> &'a Event {
		self.event
	}

	/// Get the currently displayed image for the window.
	pub fn image(&self) -> Option<&Image> {
		self.window.image()
	}

	/// Get the window that triggered the event.
	pub fn window<'b>(&'b self) -> &'b WindowInner {
		self.window
	}

	/// Get the window that triggered the event.
	pub fn window_mut<'b>(&'b mut self) -> &'b mut WindowInner {
		self.window
	}

	/// Spawn a background task.
	///
	/// The task will run in a new thread.
	/// The thread will be joined when [`crate::stop`] is called.
	/// If this is not desired, simply spawn a thread manually.
	pub fn spawn_task<F: FnOnce() + Send + 'static>(&mut self, task: F) {
		self.background_tasks.push(BackgroundThread::new(task));
	}
}
