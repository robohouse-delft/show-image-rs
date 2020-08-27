use crate::event::Event;
use crate::event::WindowEvent;

/// Map a non-user [`Event`] to an [`Event`] with different `UserEvent`.
///
/// If the event was a [`Event::UserEvent`], it is returned as [`Err`].
pub fn map_nonuser_event<T, U>(event: Event<T>) -> Result<Event<U>, T> {
	use self::Event::*;
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

/// Clone a static event.
///
/// Returns [`None`] for non-static event.
pub fn clone_static_event<T>(event: &Event<T>) -> Option<Event<'static, T>>
where
	T: Clone,
{
	use self::Event::*;
	match event {
		UserEvent(x) => Some(UserEvent(x.clone())),
		WindowEvent { window_id, event } => Some(WindowEvent {
			window_id: *window_id,
			event: clone_static_window_event(event)?,
		}),
		DeviceEvent { device_id, event } => Some(DeviceEvent {
			device_id: *device_id,
			event: event.clone(),
		}),
		NewEvents(cause) => Some(NewEvents(cause.clone())),
		MainEventsCleared => Some(MainEventsCleared),
		RedrawRequested(window_id) => Some(RedrawRequested(window_id.clone())),
		RedrawEventsCleared => Some(RedrawEventsCleared),
		LoopDestroyed => Some(LoopDestroyed),
		Suspended => Some(Suspended),
		Resumed => Some(Resumed),
	}
}

/// Clone a static window event.
///
/// Returns [`None`] for non-static event.
pub fn clone_static_window_event(event: &WindowEvent) -> Option<WindowEvent<'static>> {
	use WindowEvent::*;
	match event {
		Resized(size) => Some(Resized(size.clone())),
		Moved(position) => Some(Moved(position.clone())),
		CloseRequested => Some(CloseRequested),
		Destroyed => Some(Destroyed),
		DroppedFile(file) => Some(DroppedFile(file.clone())),
		HoveredFile(file) => Some(HoveredFile(file.clone())),
		HoveredFileCancelled => Some(HoveredFileCancelled),
		ReceivedCharacter(c) => Some(ReceivedCharacter(c.clone())),
		Focused(focused) => Some(Focused(focused.clone())),
		KeyboardInput { device_id, input, is_synthetic } => Some(KeyboardInput {
			device_id: *device_id,
			input: *input,
			is_synthetic: *is_synthetic,
		}),
		ModifiersChanged(modifiers) => Some(ModifiersChanged(modifiers.clone())),
		#[allow(deprecated)]
		CursorMoved { device_id, position, modifiers } => Some(CursorMoved {
			device_id: *device_id,
			position: *position,
			modifiers: *modifiers,
		}),
		CursorEntered { device_id } => Some(CursorEntered { device_id: *device_id }),
		CursorLeft { device_id } => Some(CursorLeft { device_id: *device_id }),
		#[allow(deprecated)]
		MouseWheel { device_id, delta, phase, modifiers } => Some(MouseWheel {
			device_id: *device_id,
			delta: *delta,
			phase: *phase,
			modifiers: *modifiers,
		}),
		#[allow(deprecated)]
		MouseInput { device_id, state, button, modifiers } => Some(MouseInput {
			device_id: *device_id,
			state: *state,
			button: *button,
			modifiers: *modifiers,
		}),
		TouchpadPressure { device_id, pressure, stage } => Some(TouchpadPressure {
			device_id: *device_id,
			pressure: *pressure,
			stage: *stage,
		}),
		AxisMotion { device_id, axis, value } => Some(AxisMotion {
			device_id: *device_id,
			axis: *axis,
			value: *value,
		}),
		Touch(touch) => Some(Touch(touch.clone())),
		ThemeChanged(theme) => Some(ThemeChanged(theme.clone())),
		ScaleFactorChanged { .. } => None,
	}
}
