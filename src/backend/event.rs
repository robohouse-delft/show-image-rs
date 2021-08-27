use super::mouse_cache::MouseCache;

pub fn convert_winit_event(
	event: winit::event::Event<()>,
	mouse_cache: &MouseCache,
) -> Option<crate::event::Event> {
	use crate::event::Event as C;
	use winit::event::Event as W;

	match event {
		W::UserEvent(_) => None,
		W::WindowEvent { window_id, event } => Some(convert_winit_window_event(window_id, event, mouse_cache)?.into()),
		W::DeviceEvent { device_id, event } => Some(convert_winit_device_event(device_id, event).into()),
		W::NewEvents(_) => Some(C::NewEvents),
		W::MainEventsCleared => Some(C::MainEventsCleared),
		W::RedrawRequested(window_id) => Some(C::WindowEvent(crate::event::WindowRedrawRequestedEvent { window_id }.into())),
		W::RedrawEventsCleared => Some(C::RedrawEventsCleared),
		// You can't stop the event loop!
		W::LoopDestroyed => None,
		W::Suspended => Some(C::Suspended),
		W::Resumed => Some(C::Resumed),
	}
}

pub fn convert_winit_device_event(
	device_id: winit::event::DeviceId,
	event: winit::event::DeviceEvent,
) -> crate::event::DeviceEvent {
	use crate::event;
	use winit::event::DeviceEvent as W;
	match event {
		W::Added => event::DeviceAddedEvent { device_id }.into(),
		W::Removed => event::DeviceRemovedEvent { device_id }.into(),
		W::MouseMotion { delta } => event::DeviceMouseMotionEvent {
			device_id,
			delta: glam::DVec2::new(delta.0, delta.1).as_vec2(),
		}
		.into(),
		W::MouseWheel { delta } => event::DeviceMouseWheelEvent { device_id, delta }.into(),
		W::Motion { axis, value } => event::DeviceMotionEvent { device_id, axis, value }.into(),
		W::Button { button, state } => event::DeviceButtonEvent {
			device_id,
			button,
			state: state.into(),
		}
		.into(),
		W::Key(input) => event::DeviceKeyboardInputEvent {
			device_id,
			input: convert_winit_keyboard_input(input),
		}
		.into(),
		W::Text { codepoint } => event::DeviceTextInputEvent { device_id, codepoint }.into(),
	}
}

pub fn convert_winit_window_event(
	window_id: winit::window::WindowId,
	event: winit::event::WindowEvent,
	mouse_cache: &MouseCache,
) -> Option<crate::event::WindowEvent> {
	use crate::event;
	use winit::event::WindowEvent as W;

	#[allow(deprecated)]
	match event {
		W::Resized(size) => Some(event::WindowResizedEvent { window_id, size: glam::UVec2::new(size.width, size.height) }.into()),
		W::Moved(position) => Some(event::WindowMovedEvent { window_id, position: glam::IVec2::new(position.x, position.y) }.into()),
		W::CloseRequested => Some(event::WindowCloseRequestedEvent { window_id }.into()),
		W::Destroyed => Some(event::WindowDestroyedEvent { window_id }.into()),
		W::DroppedFile(file) => Some(event::WindowDroppedFileEvent { window_id, file }.into()),
		W::HoveredFile(file) => Some(event::WindowHoveredFileEvent { window_id, file }.into()),
		W::HoveredFileCancelled => Some(event::WindowHoveredFileCancelledEvent { window_id }.into()),
		W::ReceivedCharacter(character) => Some(event::WindowTextInputEvent { window_id, character }.into()),
		W::Focused(true) => Some(event::WindowFocusGainedEvent { window_id }.into()),
		W::Focused(false) => Some(event::WindowFocusLostEvent { window_id }.into()),
		W::KeyboardInput {
			device_id,
			input,
			is_synthetic,
		} => Some(
			event::WindowKeyboardInputEvent {
				window_id,
				device_id,
				input: convert_winit_keyboard_input(input),
				is_synthetic,
			}
			.into(),
		),
		W::ModifiersChanged(_) => None,
		W::CursorMoved {
			device_id,
			position,
			modifiers,
		} => {
			let position = glam::DVec2::new(position.x, position.y).as_vec2();
			Some(event::WindowMouseMoveEvent {
				window_id,
				device_id,
				position,
				prev_position: mouse_cache.get_prev_position(window_id, device_id).unwrap_or(position),
				modifiers,
				buttons: mouse_cache.get_buttons(device_id).cloned().unwrap_or_default(),
			}.into())
		},
		W::CursorEntered { device_id } => Some(event::WindowMouseEnterEvent {
			window_id,
			device_id,
			buttons: mouse_cache.get_buttons(device_id).cloned().unwrap_or_default(),
		}.into()),
		W::CursorLeft { device_id } => Some(event::WindowMouseLeaveEvent {
			window_id,
			device_id,
			buttons: mouse_cache.get_buttons(device_id).cloned().unwrap_or_default(),
		}.into()),
		W::MouseWheel {
			device_id,
			delta,
			phase,
			modifiers,
		} => Some(
			event::WindowMouseWheelEvent {
				window_id,
				device_id,
				delta,
				phase,
				position: mouse_cache.get_position(window_id, device_id),
				buttons: mouse_cache.get_buttons(device_id).cloned().unwrap_or_default(),
				modifiers,
			}
			.into(),
		),
		W::MouseInput {
			device_id,
			state,
			button,
			modifiers,
		} => {
			let position = mouse_cache.get_position(window_id, device_id)?;
			let prev_position = mouse_cache.get_prev_position(window_id, device_id).unwrap_or(position);
			Some(event::WindowMouseButtonEvent {
				window_id,
				device_id,
				button: button.into(),
				state: state.into(),
				position,
				prev_position,
				buttons: mouse_cache.get_buttons(device_id).cloned().unwrap_or_default(),
				modifiers,
			}.into())
		},
		W::TouchpadPressure {
			device_id,
			pressure,
			stage,
		} => Some(
			event::WindowTouchpadPressureEvent {
				window_id,
				device_id,
				pressure,
				stage,
			}
			.into(),
		),
		W::AxisMotion { device_id, axis, value } => Some(
			event::WindowAxisMotionEvent {
				window_id,
				device_id,
				axis,
				value,
			}
			.into(),
		),
		W::Touch(touch) => Some(event::WindowTouchEvent { window_id, touch }.into()),
		W::ThemeChanged(theme) => Some(
			event::WindowThemeChangedEvent {
				window_id,
				theme: theme.into(),
			}
			.into(),
		),
		W::ScaleFactorChanged { scale_factor, .. } => Some(event::WindowScaleFactorChangedEvent { window_id, scale_factor }.into()),
	}
}

pub fn convert_winit_keyboard_input(input: winit::event::KeyboardInput) -> crate::event::KeyboardInput {
	#[allow(deprecated)]
	crate::event::KeyboardInput {
		scan_code: input.scancode,
		key_code: input.virtual_keycode,
		modifiers: input.modifiers,
		state: input.state.into(),
	}
}

/// Map a non-user [`Event`] to an [`Event`] with different `UserEvent`.
///
/// If the event was a [`Event::UserEvent`], it is returned as [`Err`].
pub fn map_nonuser_event<T, U>(event: winit::event::Event<T>) -> Result<winit::event::Event<U>, T> {
	use winit::event::Event::*;
	match event {
		UserEvent(x) => Err(x),
		WindowEvent { window_id, event } => Ok(WindowEvent { window_id, event }),
		DeviceEvent { device_id, event } => Ok(DeviceEvent { device_id, event }),
		NewEvents(cause) => Ok(NewEvents(cause)),
		MainEventsCleared => Ok(MainEventsCleared),
		RedrawRequested(wid) => Ok(RedrawRequested(wid)),
		RedrawEventsCleared => Ok(RedrawEventsCleared),
		LoopDestroyed => Ok(LoopDestroyed),
		Suspended => Ok(Suspended),
		Resumed => Ok(Resumed),
	}
}

impl From<winit::event::ElementState> for crate::event::ElementState {
	fn from(other: winit::event::ElementState) -> Self {
		match other {
			winit::event::ElementState::Pressed => Self::Pressed,
			winit::event::ElementState::Released => Self::Released,
		}
	}
}

impl From<winit::event::MouseButton> for crate::event::MouseButton {
	fn from(other: winit::event::MouseButton) -> Self {
		match other {
			winit::event::MouseButton::Left => Self::Left,
			winit::event::MouseButton::Right => Self::Right,
			winit::event::MouseButton::Middle => Self::Middle,
			winit::event::MouseButton::Other(x) => Self::Other(x),
		}
	}
}

impl From<winit::window::Theme> for crate::event::Theme {
	fn from(other: winit::window::Theme) -> Self {
		match other {
			winit::window::Theme::Light => Self::Light,
			winit::window::Theme::Dark => Self::Dark,
		}
	}
}
