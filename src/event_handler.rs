/// Return value of event handlers.
#[derive(Debug, Clone, Default)]
pub struct EventHandlerOutput {
	/// Remove the event handler.
	///
	/// If this field is set to true, the event handler will be dropped.
	pub remove_handler: bool,

	/// Stop processing this event.
	///
	/// No other event handlers will be called.
	pub stop_processing: bool,
}
