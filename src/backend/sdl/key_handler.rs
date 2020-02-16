use std::sync::Arc;

use crate::background_thread::BackgroundThread;
use crate::KeyboardEvent;
use crate::ImageInfo;

/// A key event handler.
pub type KeyHandler = Box<dyn FnMut(&mut KeyHandlerContext) + Send>;

/// The context for a registered keyboard event handler.
pub struct KeyHandlerContext<'a> {
	/// The vector to add spawned tasks too.
	background_tasks: &'a mut Vec<BackgroundThread<()>>,

	/// Flag to indicate if the key event should be passed to other handlers.
	stop_propagation: bool,

	/// The keyboard event to be handler.
	event: &'a KeyboardEvent,

	/// The currently visible image in the window.
	image: Option<&'a (Arc<[u8]>, ImageInfo, String)>,
}

impl<'a> KeyHandlerContext<'a> {
	pub(crate) fn new(
		background_tasks: &'a mut Vec<BackgroundThread<()>>,
		event: &'a KeyboardEvent,
		image: Option<&'a (Arc<[u8]>, ImageInfo, String)>
	) -> Self {
		Self {
			background_tasks,
			stop_propagation: false,
			event,
			image,
		}
	}

	/// Stop propagation of the keyboard event to other handlers.
	pub fn stop_propagation(&mut self) {
		self.stop_propagation = true;
	}

	/// Check if we should stop propagation of the keyboard event.
	pub(crate) fn should_stop_propagation(&self) -> bool {
		self.stop_propagation
	}

	/// Get the keyboard event.
	pub fn event(&self) -> &'a KeyboardEvent {
		self.event
	}

	/// Get the currently displayed image for the window.
	pub fn image(&self) -> Option<&'a (Arc<[u8]>, ImageInfo, String)> {
		self.image
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
