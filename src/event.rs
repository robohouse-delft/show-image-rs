use crate::KeyboardEvent;

/// Enum describing any of the possible events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
	/// A keyboard event.
	KeyboardEvent(KeyboardEvent),

	/// A mouse move event.
	MouseMoveEvent(MouseMoveEvent),

	/// A mouse button event.
	MouseButtonEvent(MouseButtonEvent),
}

/// Information describing a mouse state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MouseButtonState {
	/// The state of the left mouse button.
	pub left: bool,

	/// The state of the middle mouse button.
	pub middle: bool,

	/// The state of the right mouse button.
	pub right: bool,
}

/// Information describing a mouse move event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MouseMoveEvent {
	/// The mouse ID identifying which mouse triggered the event.
	pub mouse_id: u32,

	/// State of the mouse buttons at the time of the event triggering.
	pub mouse_state: MouseButtonState,

	/// The X coordinate of the mouse relative to the window.
	pub position_x: i32,

	/// The Y coordinate of the mouse relative to the window.
	pub position_y: i32,

	/// The relative X coordinate w.r.t. the previous event of the mouse relative to the window.
	pub relative_x: i32,

	/// The relative Y coordinate w.r.t. the previous event of the mouse relative to the window.
	pub relative_y: i32,
}

/// Enum describing the mouse buttons.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MouseButton {
	/// Some unknown button.
	Unknown,

	/// The left mouse button.
	Left,

	/// The middle mouse button.
	Middle,

	/// The right mouse button.
	Right,
}

/// Enum describing the mouse buttons.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MouseState {
	/// State when a button is pressed.
	Down,

	/// State when a button is released.
	Up,
}

/// Information describing a mouse button event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MouseButtonEvent {
	/// The mouse ID identifying which mouse triggered the event.
	pub mouse_id: u32,

	/// The button that triggered the event.
	pub button: MouseButton,

	/// State after triggering the event.
	pub state: MouseState,

	/// Number of clicks that happened.
	pub clicks: u8,

	/// The X coordinate of the mouse relative to the window.
	pub position_x: i32,

	/// The Y coordinate of the mouse relative to the window.
	pub position_y: i32,
}
