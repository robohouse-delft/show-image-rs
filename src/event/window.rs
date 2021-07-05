use super::AxisId;
use super::DeviceId;
use super::ElementState;
use super::KeyboardInput;
use super::ModifiersState;
use super::MouseButton;
use super::MouseButtonState;
use super::MouseScrollDelta;
use super::Theme;
use super::Touch;
use super::TouchPhase;
use crate::WindowId;

use std::path::PathBuf;

/// Window event.
#[derive(Debug, Clone)]
pub enum WindowEvent {
	/// A redraw was requested by the OS or application code.
	RedrawRequested(WindowRedrawRequestedEvent),

	/// A window was resized.
	Resized(WindowResizedEvent),

	/// A window was moved.
	Moved(WindowMovedEvent),

	/// A window was closed.
	CloseRequested(WindowCloseRequestedEvent),

	/// A window was destroyed.
	Destroyed(WindowDestroyedEvent),

	/// A file was dropped on a window.
	DroppedFile(WindowDroppedFileEvent),

	/// A file is being hovered over a window.
	HoveredFile(WindowHoveredFileEvent),

	/// A file that was being hovered over a window was canceled..
	HoveredFileCancelled(WindowHoveredFileCancelledEvent),

	/// A window gained input focus.
	FocusGained(WindowFocusGainedEvent),

	/// A window lost input focus.
	FocusLost(WindowFocusLostEvent),

	/// A window received keyboard input.
	KeyboardInput(WindowKeyboardInputEvent),

	/// A window received text input.
	TextInput(WindowTextInputEvent),

	/// The mouse cursor entered a window.
	MouseEnter(WindowMouseEnterEvent),

	/// The mouse cursor left a window.
	MouseLeave(WindowMouseLeaveEvent),

	/// The mouse cursor was moved on a window.
	MouseMove(WindowMouseMoveEvent),

	/// A mouse button was pressed or released on a window.
	MouseButton(WindowMouseButtonEvent),

	/// A window received mouse wheel input.
	MouseWheel(WindowMouseWheelEvent),

	/// A window received axis motion input.
	AxisMotion(WindowAxisMotionEvent),

	/// A window received touchpad pressure input.
	TouchpadPressure(WindowTouchpadPressureEvent),

	/// A window received touch input.
	Touch(WindowTouchEvent),

	/// The scale factor between logical and physical pixels for a window changed.
	ScaleFactorChanged(WindowScaleFactorChangedEvent),

	/// The theme for a window changed.
	ThemeChanged(WindowThemeChangedEvent),
}

impl WindowEvent {
	/// Get the window ID of the event.
	pub fn window_id(&self) -> WindowId {
		match self {
			Self::RedrawRequested(x) => x.window_id,
			Self::Resized(x) => x.window_id,
			Self::Moved(x) => x.window_id,
			Self::CloseRequested(x) => x.window_id,
			Self::Destroyed(x) => x.window_id,
			Self::DroppedFile(x) => x.window_id,
			Self::HoveredFile(x) => x.window_id,
			Self::HoveredFileCancelled(x) => x.window_id,
			Self::FocusGained(x) => x.window_id,
			Self::FocusLost(x) => x.window_id,
			Self::KeyboardInput(x) => x.window_id,
			Self::TextInput(x) => x.window_id,
			Self::MouseEnter(x) => x.window_id,
			Self::MouseLeave(x) => x.window_id,
			Self::MouseMove(x) => x.window_id,
			Self::MouseButton(x) => x.window_id,
			Self::MouseWheel(x) => x.window_id,
			Self::AxisMotion(x) => x.window_id,
			Self::TouchpadPressure(x) => x.window_id,
			Self::Touch(x) => x.window_id,
			Self::ScaleFactorChanged(x) => x.window_id,
			Self::ThemeChanged(x) => x.window_id,
		}
	}
}

/// A redraw was requested by the OS or application code.
#[derive(Debug, Clone)]
pub struct WindowRedrawRequestedEvent {
	/// The ID of the window.
	pub window_id: WindowId,
}

/// A window was resized.
#[derive(Debug, Clone)]
pub struct WindowResizedEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The new size of the window in physical pixels.
	pub size: glam::UVec2,
}

/// A window was moved.
#[derive(Debug, Clone)]
pub struct WindowMovedEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The new position of the window in physical pixels.
	pub position: glam::IVec2,
}

/// A window was closed.
#[derive(Debug, Clone)]
pub struct WindowCloseRequestedEvent {
	/// The ID of the window.
	pub window_id: WindowId,
}

/// A window was destroyed.
#[derive(Debug, Clone)]
pub struct WindowDestroyedEvent {
	/// The ID of the window.
	pub window_id: WindowId,
}

/// A file was dropped on a window.
#[derive(Debug, Clone)]
pub struct WindowDroppedFileEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The path of the file.
	pub file: PathBuf,
}

/// A file is being hovered over a window.
#[derive(Debug, Clone)]
pub struct WindowHoveredFileEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The path of the file.
	pub file: PathBuf,
}

/// A file that was being hovered over a window was canceled..
#[derive(Debug, Clone)]
pub struct WindowHoveredFileCancelledEvent {
	/// The ID of the window.
	pub window_id: WindowId,
}

/// A window gained input focus.
#[derive(Debug, Clone)]
pub struct WindowFocusGainedEvent {
	/// The ID of the window.
	pub window_id: WindowId,
}

/// A window lost input focus.
#[derive(Debug, Clone)]
pub struct WindowFocusLostEvent {
	/// The ID of the window.
	pub window_id: WindowId,
}

/// A window received keyboard input.
#[derive(Debug, Clone)]
pub struct WindowKeyboardInputEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The received input.
	pub input: KeyboardInput,

	/// Flag to indicate if the input is synthetic.
	///
	/// Some synthetic events may be generated to report changes in keyboard state while the window did not have input focus.
	/// This flag allows you to distinguish such events.
	pub is_synthetic: bool,
}

/// A window received text input.
#[derive(Debug, Clone)]
pub struct WindowTextInputEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The unicode codepoint representing the input.
	pub character: char,
}

/// The mouse cursor entered the window area.
#[derive(Debug, Clone)]
pub struct WindowMouseEnterEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The pressed state of all mouse buttons.
	pub buttons: MouseButtonState,
}

/// The mouse cursor left the window area.
#[derive(Debug, Clone)]
pub struct WindowMouseLeaveEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The pressed state of all mouse buttons.
	pub buttons: MouseButtonState,
}

/// The mouse cursor was moved on a window.
#[derive(Debug, Clone)]
pub struct WindowMouseMoveEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The new position of the cursor in physical pixels, relative to the top-left corner of the window.
	pub position: glam::Vec2,

	/// The position of the mouse cursor before the last movement.
	pub prev_position: glam::Vec2,

	/// The pressed state of all mouse buttons.
	pub buttons: MouseButtonState,

	/// The state of the keyboard modifiers at the time of the event.
	pub modifiers: ModifiersState,
}

/// A window received mouse input.
#[derive(Debug, Clone)]
pub struct WindowMouseButtonEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The mouse button that was pressed.
	pub button: MouseButton,

	/// The new state of the mouse button.
	pub state: ElementState,

	/// The current position of the mouse cursor inside the window.
	pub position: glam::Vec2,

	/// The position of the mouse cursor before the last movement.
	pub prev_position: glam::Vec2,

	/// The pressed state of all mouse buttons.
	pub buttons: MouseButtonState,

	/// The state of the keyboard modifiers at the time of the event.
	pub modifiers: ModifiersState,
}

/// A window received mouse wheel input.
#[derive(Debug, Clone)]
pub struct WindowMouseWheelEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The scroll delta of the mouse wheel.
	pub delta: MouseScrollDelta,

	/// The touch-screen input state.
	pub phase: TouchPhase,

	/// The current position of the mouse cursor inside the window.
	pub position: Option<glam::Vec2>,

	/// The pressed state of all mouse buttons.
	pub buttons: MouseButtonState,

	/// The state of the keyboard modifiers at the time of the event.
	pub modifiers: ModifiersState,
}

/// A window received axis motion input.
#[derive(Debug, Clone)]
pub struct WindowAxisMotionEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The axis that as moved.
	pub axis: AxisId,

	/// The value by which the axis moved.
	pub value: f64,
}

/// A window received touchpad pressure input.
#[derive(Debug, Clone)]
pub struct WindowTouchpadPressureEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The device that generated the input.
	pub device_id: DeviceId,

	/// The pressure on the touch pad, in the range 0 to 1.
	pub pressure: f32,

	/// The click level of the touch pad.
	pub stage: i64,
}

/// A window received touch input.
#[derive(Debug, Clone)]
pub struct WindowTouchEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The touch input.
	pub touch: Touch,
}

/// The scale factor between logical and physical pixels for a window changed.
#[derive(Debug, Clone)]
pub struct WindowScaleFactorChangedEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The new scale factor as physical pixels per logical pixel.
	pub scale_factor: f64,
}

/// The theme for a window changed.
#[derive(Debug, Clone)]
pub struct WindowThemeChangedEvent {
	/// The ID of the window.
	pub window_id: WindowId,

	/// The new theme of the window.
	pub theme: Theme,
}

impl_from_variant!(WindowEvent::RedrawRequested(WindowRedrawRequestedEvent));
impl_from_variant!(WindowEvent::Resized(WindowResizedEvent));
impl_from_variant!(WindowEvent::Moved(WindowMovedEvent));
impl_from_variant!(WindowEvent::CloseRequested(WindowCloseRequestedEvent));
impl_from_variant!(WindowEvent::Destroyed(WindowDestroyedEvent));
impl_from_variant!(WindowEvent::DroppedFile(WindowDroppedFileEvent));
impl_from_variant!(WindowEvent::HoveredFile(WindowHoveredFileEvent));
impl_from_variant!(WindowEvent::HoveredFileCancelled(WindowHoveredFileCancelledEvent));
impl_from_variant!(WindowEvent::FocusGained(WindowFocusGainedEvent));
impl_from_variant!(WindowEvent::FocusLost(WindowFocusLostEvent));
impl_from_variant!(WindowEvent::KeyboardInput(WindowKeyboardInputEvent));
impl_from_variant!(WindowEvent::TextInput(WindowTextInputEvent));
impl_from_variant!(WindowEvent::MouseEnter(WindowMouseEnterEvent));
impl_from_variant!(WindowEvent::MouseLeave(WindowMouseLeaveEvent));
impl_from_variant!(WindowEvent::MouseMove(WindowMouseMoveEvent));
impl_from_variant!(WindowEvent::MouseButton(WindowMouseButtonEvent));
impl_from_variant!(WindowEvent::MouseWheel(WindowMouseWheelEvent));
impl_from_variant!(WindowEvent::AxisMotion(WindowAxisMotionEvent));
impl_from_variant!(WindowEvent::TouchpadPressure(WindowTouchpadPressureEvent));
impl_from_variant!(WindowEvent::Touch(WindowTouchEvent));
impl_from_variant!(WindowEvent::ScaleFactorChanged(WindowScaleFactorChangedEvent));
impl_from_variant!(WindowEvent::ThemeChanged(WindowThemeChangedEvent));
