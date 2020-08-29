pub use device::*;
pub use window::*;

pub use winit::dpi::LogicalPosition;
pub use winit::dpi::LogicalSize;
pub use winit::dpi::PhysicalPosition;
pub use winit::dpi::PhysicalSize;
pub use winit::event::AxisId;
pub use winit::event::ButtonId;
pub use winit::event::DeviceId;
pub use winit::event::Force;
pub use winit::event::ModifiersState;
pub use winit::event::MouseButton;
pub use winit::event::MouseScrollDelta;
pub use winit::event::ScanCode;
pub use winit::event::StartCause;
pub use winit::event::Touch;
pub use winit::event::TouchPhase;
pub use winit::event::VirtualKeyCode;

macro_rules! impl_from_variant {
	($for:ident::$variant:ident($from:ty)) => {
		impl From<$from> for $for {
			fn from(other: $from) -> Self {
				Self::$variant(other)
			}
		}
	};
}

mod device;
mod window;

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

/// Global event.
///
/// This also includes window events for all windows.
#[derive(Debug, Clone)]
pub enum Event {
	/// New events are available for processing.
	///
	/// This indicates the start of a new event-processing cycle.
	NewEvents,

	/// A window event.
	WindowEvent(WindowEvent),

	/// A device event.
	DeviceEvent(DeviceEvent),

	/// The application has been suspended.
	Suspended,

	/// The application has been resumed.
	Resumed,

	/// All input events have been processed and redraw processing is about to begin.
	MainEventsCleared,

	/// All open redraw requests have been processed.
	RedrawEventsCleared,

	/// All windows were closed.
	AllWindowsClosed,
}

impl_from_variant!(Event::WindowEvent(WindowEvent));
impl_from_variant!(Event::DeviceEvent(DeviceEvent));

/// Keyboard input.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct KeyboardInput {
	/// Scan code of the physical key.
	///
	/// This should not change if the user adjusts the host's keyboard map.
	/// Use when the physical location of the key is more important than the key's host GUI semantics, such as for movement controls in a first-person game.
	pub scan_code: ScanCode,

	/// Virtual key code indentifying the semantic meaning of the key.
	///
	/// Use this when the semantics of the key are more important than the physical location of the key, such as when implementing appropriate behavior for "page up".
	pub key_code: Option<VirtualKeyCode>,

	/// State of the key (pressed or released).
	pub state: ElementState,

	/// Keyboard modifiers that were active at the time of the event.
	pub modifiers: ModifiersState,
}

/// OS theme (light or dark).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Theme {
	Light,
	Dark,
}

impl Theme {
	pub fn is_light(self) -> bool {
		self == Self::Light
	}

	pub fn is_dark(self) -> bool {
		self == Self::Dark
	}
}

/// State of a button or key.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ElementState {
	Pressed,
	Released,
}

impl ElementState {
	pub fn is_pressed(self) -> bool {
		self == Self::Pressed
	}

	pub fn is_released(self) -> bool {
		self == Self::Released
	}
}
