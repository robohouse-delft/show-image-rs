use crate::WindowId;

#[derive(Debug, Clone)]
pub struct InvalidWindowIdError {
	pub window_id: WindowId,
}

#[derive(Debug, Clone)]
pub struct NoSuitableAdapterFoundError;

#[derive(Debug)]
pub enum ProxyError<T> {
	EventLoopClosed(EventLoopClosedError),
	Timeout(TimeoutError),
	Inner(T),
}

#[derive(Debug, Clone)]
pub struct EventLoopClosedError;

#[derive(Debug, Clone)]
pub struct TimeoutError;

impl<T> From<EventLoopClosedError> for ProxyError<T> {
	fn from(other: EventLoopClosedError) -> Self {
		Self::EventLoopClosed(other)
	}
}

impl<T> From<TimeoutError> for ProxyError<T> {
	fn from(other: TimeoutError) -> Self {
		Self::Timeout(other)
	}
}
