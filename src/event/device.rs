use super::AxisId;
use super::ButtonId;
use super::DeviceId;
use super::ElementState;
use super::KeyboardInput;
use super::MouseScrollDelta;

/// Raw hardware events that are not associated with any particular window.
///
/// Useful for interactions that diverge significantly from a conventional 2D GUI, such as 3D camera or first-person game controls.
/// Many physical actions, such as mouse movement, can produce both device and window events.
/// Because window events typically arise from virtual devices (corresponding to GUI cursors and keyboard focus) the device IDs may not match.
///
/// Note that these events are delivered regardless of input focus.
#[derive(Debug, Clone)]
pub enum DeviceEvent {
	/// A new device was added.
	Added(DeviceAddedEvent),

	/// A device was removed.
	Removed(DeviceRemovedEvent),

	/// Change in physical position of a pointing device.
	MouseMotion(DeviceMouseMotionEvent),

	/// The scroll-wheel of a mouse was moved.
	MouseWheel(DeviceMouseWheelEvent),

	/// Motion on some analog axis.
	Motion(DeviceMotionEvent),

	/// A button on a device was pressed or released.
	Button(DeviceButtonEvent),

	/// A device generated keyboard input.
	KeyboardInput(DeviceKeyboardInputEvent),

	/// A device generated text input.
	TextInput(DeviceTextInputEvent),
}

#[derive(Debug, Clone)]
/// A new device was added.
pub struct DeviceAddedEvent {
	/// The ID of the device.
	pub device_id: DeviceId,
}

/// A device was removed.
#[derive(Debug, Clone)]
pub struct DeviceRemovedEvent {
	/// The ID of the device.
	pub device_id: DeviceId,
}

/// The physical position of a pointing device was moved.
///
/// This represents raw, unfiltered physical motion.
/// Not to be confused with [`WindowMouseMoveEvent`][super::WindowMouseMoveEvent].
#[derive(Debug, Clone)]
pub struct DeviceMouseMotionEvent {
	/// The ID of the device.
	pub device_id: DeviceId,

	/// The relative motion.
	pub delta: glam::Vec2,
}

/// The scroll-wheel of a mouse was moved.
#[derive(Debug, Clone)]
pub struct DeviceMouseWheelEvent {
	/// The ID of the device.
	pub device_id: DeviceId,

	/// The scroll delta.
	pub delta: MouseScrollDelta,
}

/// An analog axis of a device was moved.
///
/// This event will be reported for all arbitrary input devices that winit supports on this platform, including mouse devices.
/// If the device is a mouse device then this will be reported alongside the [`DeviceMouseMotionEvent`].
#[derive(Debug, Clone)]
pub struct DeviceMotionEvent {
	/// The ID of the device.
	pub device_id: DeviceId,

	/// The axis that was moved.
	pub axis: AxisId,

	/// The value by which the axis was moved.
	pub value: f64,
}

/// A button on a device was pressed or released.
#[derive(Debug, Clone)]
pub struct DeviceButtonEvent {
	/// The ID of the device.
	pub device_id: DeviceId,

	/// The button that was pressed or released.
	pub button: ButtonId,

	/// The new state of the button (pressed or released).
	pub state: ElementState,
}

/// A device generated keyboard input.
#[derive(Debug, Clone)]
pub struct DeviceKeyboardInputEvent {
	/// The ID of the device.
	pub device_id: DeviceId,

	/// The keyboard input.
	pub input: KeyboardInput,
}

/// A device generated text input.
#[derive(Debug, Clone)]
pub struct DeviceTextInputEvent {
	/// The ID of the device.
	pub device_id: DeviceId,

	/// The unicode codepoint that was generated.
	pub codepoint: char,
}

impl_from_variant!(DeviceEvent::Added(DeviceAddedEvent));
impl_from_variant!(DeviceEvent::Removed(DeviceRemovedEvent));
impl_from_variant!(DeviceEvent::MouseMotion(DeviceMouseMotionEvent));
impl_from_variant!(DeviceEvent::MouseWheel(DeviceMouseWheelEvent));
impl_from_variant!(DeviceEvent::Motion(DeviceMotionEvent));
impl_from_variant!(DeviceEvent::Button(DeviceButtonEvent));
impl_from_variant!(DeviceEvent::KeyboardInput(DeviceKeyboardInputEvent));
impl_from_variant!(DeviceEvent::TextInput(DeviceTextInputEvent));
