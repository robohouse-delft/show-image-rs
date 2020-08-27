/// Event indicating that all open windows have been closed.
#[derive(Debug, Copy, Clone)]
pub struct AllWindowsClosed;

/// Control flow properties for event handlers.
///
/// Instances of this struct are passed to event handlers
/// to allow them to remove themselves and to stop event propagation.
#[derive(Debug, Default, Clone)]
pub struct EventHandlerControlFlow {
	/// Remove the event handler after it returned.
	pub remove_handler: bool,

	/// Stop propagation of the event to other event handlers.
	pub stop_propagation: bool,
}
