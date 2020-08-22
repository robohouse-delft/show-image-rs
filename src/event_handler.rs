/// Return value for event handlers.
#[derive(Debug, Clone, Default)]
pub struct EventHandlerOutput {
	/// Remove the event handler after it returned.
	pub remove_handler: bool,

	/// Stop propagation of the event to other event handlers.
	pub stop_propagation: bool,
}
