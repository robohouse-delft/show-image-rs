use crate::event::Event;
use super::proxy::ContextEvent;
use super::proxy::ExecuteFunction;

/// Downgrade an `Event<ContextEvent<T>>` to an `Event<T>`.
///
/// If the event was actually a [`ContextCmmand`], it is returned as [`Err`].
pub fn downgrade_event<UserEvent>(event: Event<ContextEvent<UserEvent>>) -> Result<Event<UserEvent>, ExecuteFunction<UserEvent>> {
	match map_nonuser_event(event) {
		Ok(x) => Ok(x),
		Err(ContextEvent::UserEvent(x)) => Ok(Event::UserEvent(x)),
		Err(ContextEvent::ExecuteFunction(x)) => Err(x),
	}
}

/// Map a non-user [`Event`] to an [`Event`] with different `UserEvent`.
///
/// If the event was a [`Event::UserEvent`], it is returned as [`Err`].
fn map_nonuser_event<T, U>(event: Event<T>) -> Result<Event<U>, T> {
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
