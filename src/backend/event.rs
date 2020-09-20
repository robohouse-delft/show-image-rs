pub fn convert_winit_event(event: winit::event::Event<()>) -> Option<crate::event::Event> {
	use crate::event::Event as C;
	use winit::event::Event as W;

	match event {
		W::UserEvent(_) => None,
		W::WindowEvent { window_id, event } => Some(convert_winit_window_event(window_id, event)?.into()),
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

pub fn convert_winit_device_event(device_id: winit::event::DeviceId, event: winit::event::DeviceEvent) -> crate::event::DeviceEvent {
	use crate::event;
	use winit::event::DeviceEvent as W;
	match event {
		W::Added => event::DeviceAddedEvent { device_id }.into(),
		W::Removed => event::DeviceRemovedEvent { device_id }.into(),
		W::MouseMotion { delta } => event::DeviceMouseMotionEvent {
			device_id,
			delta: [delta.0, delta.1],
		}
		.into(),
		W::MouseWheel { delta } => event::DeviceMouseWheelEvent { device_id, delta }.into(),
		W::Motion { axis, value } => event::DeviceMotionEvent { device_id, axis, value }.into(),
		W::Button { button, state } => event::DeviceButtonEvent {
			device_id,
			button,
			state: convert_winit_element_state(state),
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
) -> Option<crate::event::WindowEvent> {
	use crate::event;
	use winit::event::WindowEvent as W;

	#[allow(deprecated)]
	match event {
		W::Resized(size) => Some(event::WindowResizedEvent { window_id, size }.into()),
		W::Moved(position) => Some(event::WindowMovedEvent { window_id, position }.into()),
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
		} => Some(
			event::WindowCursorMovedEvent {
				window_id,
				device_id,
				position,
				modifiers,
			}
			.into(),
		),
		W::CursorEntered { device_id } => Some(event::WindowCursorEnteredEvent { window_id, device_id }.into()),
		W::CursorLeft { device_id } => Some(event::WindowCursorLeftEvent { window_id, device_id }.into()),
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
				modifiers,
			}
			.into(),
		),
		W::MouseInput {
			device_id,
			state,
			button,
			modifiers,
		} => Some(
			event::WindowMouseInputEvent {
				window_id,
				device_id,
				button,
				state: convert_winit_element_state(state),
				modifiers,
			}
			.into(),
		),
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
				theme: convert_winit_theme(theme),
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
		state: convert_winit_element_state(input.state),
	}
}

fn convert_winit_element_state(state: winit::event::ElementState) -> crate::event::ElementState {
	match state {
		winit::event::ElementState::Pressed => crate::event::ElementState::Pressed,
		winit::event::ElementState::Released => crate::event::ElementState::Released,
	}
}

fn convert_winit_theme(theme: winit::window::Theme) -> crate::event::Theme {
	match theme {
		winit::window::Theme::Light => crate::event::Theme::Light,
		winit::window::Theme::Dark => crate::event::Theme::Dark,
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
