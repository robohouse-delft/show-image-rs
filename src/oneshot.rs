use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::MutexGuard;

const NOT_READY: u8 = 0;
const FINISHED: u8 = 1;
const DISCONNECTED: u8 = 2;

pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>) {
	let inner = Arc::new(Inner::new());
	(Sender::new(inner.clone()), Receiver::new(inner))
}

pub struct Sender<T> {
	inner: Arc<Inner<T>>,
}

pub struct Receiver<T> {
	inner: Arc<Inner<T>>,
}

struct Inner<T> {
	state: AtomicU8,
	mutex: Mutex<Option<T>>,
	condvar: Condvar,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ReceiveError {
	Disconnected,
	AlreadyRetrieved,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TryReceiveError {
	Disconnected,
	AlreadyRetrieved,
	NotReady,
}

impl<T> Inner<T> {
	fn new() -> Self {
		Self {
			state: AtomicU8::new(0),
			mutex: Mutex::new(None),
			condvar: Condvar::new(),
		}
	}
}

impl<T> Sender<T> {
	fn new(inner: Arc<Inner<T>>) -> Self {
		Self { inner }
	}

	pub fn send(self, value: T) {
		let mut lock = self.inner.mutex.lock().unwrap();
		lock.replace(value);
		self.inner.state.store(FINISHED, Ordering::Release);
		self.inner.condvar.notify_all();
	}
}

impl<T> Drop for Sender<T> {
	fn drop(&mut self) {
		let _ = self.inner.state.compare_exchange(NOT_READY, DISCONNECTED, Ordering::Release, Ordering::Relaxed);
		self.inner.condvar.notify_all();
	}
}

impl<T> Receiver<T> {
	fn new(inner: Arc<Inner<T>>) -> Self {
		Self { inner }
	}

	#[allow(unused)]
	pub fn recv(self) -> Result<T, ReceiveError> {
		let mut lock = self.inner.mutex.lock().unwrap();
		loop {
			match self.internal_try_recv(&mut lock) {
				Ok(x) => return Ok(x),
				Err(TryReceiveError::Disconnected) => return Err(ReceiveError::Disconnected),
				Err(TryReceiveError::AlreadyRetrieved) => return Err(ReceiveError::AlreadyRetrieved),
				Err(TryReceiveError::NotReady) => lock = self.inner.condvar.wait(lock).unwrap(),
			}
		}
	}

	#[allow(unused)]
	pub fn try_recv(&mut self) -> Result<T, TryReceiveError> {
		self.internal_try_recv(&mut self.inner.mutex.lock().unwrap())
	}

	#[allow(unused)]
	pub fn recv_timeout(&mut self, timeout: std::time::Duration) -> Result<T, TryReceiveError> {
		self.recv_deadline(std::time::Instant::now() + timeout)
	}

	#[allow(unused)]
	pub fn recv_deadline(&mut self, deadline: std::time::Instant) -> Result<T, TryReceiveError> {
		let mut lock = self.inner.mutex.lock().unwrap();
		loop {
			match self.internal_try_recv(&mut lock) {
				Ok(x) => return Ok(x),
				Err(TryReceiveError::Disconnected) => return Err(TryReceiveError::Disconnected),
				Err(TryReceiveError::AlreadyRetrieved) => return Err(TryReceiveError::AlreadyRetrieved),
				Err(TryReceiveError::NotReady) => {
					let now = std::time::Instant::now();
					if now >= deadline {
						drop(lock);
						return Err(TryReceiveError::NotReady);
					}
					let (new_lock, timeout_result) = self.inner.condvar.wait_timeout(lock, deadline - now).unwrap();
					if timeout_result.timed_out() {
						return Err(TryReceiveError::NotReady);
					} else {
						lock = new_lock;
					}
				},
			}
		}
	}

	fn internal_try_recv(&self, lock: &mut MutexGuard<Option<T>>) -> Result<T, TryReceiveError> {
		match self.inner.state.load(Ordering::Acquire) {
			FINISHED => lock.take().ok_or(TryReceiveError::AlreadyRetrieved),
			DISCONNECTED => Err(TryReceiveError::Disconnected),
			NOT_READY => Err(TryReceiveError::NotReady),
			x => unreachable!("invalid one-shot channel state: {}", x),
		}
	}
}

impl std::error::Error for ReceiveError {}
impl std::error::Error for TryReceiveError {}

impl std::fmt::Display for ReceiveError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ReceiveError::Disconnected => write!(f, "the sender of the oneshot channel was dropped without setting a value"),
			ReceiveError::AlreadyRetrieved => write!(f, "the value of the oneshot channel has already been retrieved"),
		}
	}
}

impl std::fmt::Display for TryReceiveError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			TryReceiveError::Disconnected => ReceiveError::Disconnected.fmt(f),
			TryReceiveError::AlreadyRetrieved => ReceiveError::AlreadyRetrieved.fmt(f),
			TryReceiveError::NotReady => write!(f, "the value of the oneshot channel is not available yet"),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn try_recv_value() {
		let (tx, mut rx) = channel();
		tx.send(10);
		assert!(rx.try_recv() == Ok(10));
	}

	#[test]
	fn try_recv_no_value() {
		let (_tx, mut rx) = channel::<i32>();
		assert!(rx.try_recv() == Err(TryReceiveError::NotReady));
	}

	#[test]
	fn try_recv_twice() {
		let (tx, mut rx) = channel::<i32>();
		tx.send(10);
		assert!(rx.try_recv() == Ok(10));
		assert!(rx.try_recv() == Err(TryReceiveError::AlreadyRetrieved));
	}

	#[test]
	fn try_recv_disconnected() {
		let (tx, mut rx) = channel::<i32>();
		drop(tx);
		assert!(rx.try_recv() == Err(TryReceiveError::Disconnected));
	}

	#[test]
	fn recv() {
		let (tx, rx) = channel();
		tx.send(10);
		assert!(rx.recv().ok() == Some(10));
	}

	#[test]
	fn recv_timeout() {
		let (_tx, mut rx) = channel::<i32>();
		assert!(rx.recv_timeout(std::time::Duration::from_millis(1)) == Err(TryReceiveError::NotReady));
	}

	#[test]
	fn recv_multithreaded() {
		let (tx, mut rx) = channel::<i32>();
		let thread = std::thread::spawn(|| {
			tx.send(12);
		});
		assert!(rx.recv_timeout(std::time::Duration::from_millis(500)) == Ok(12));
		let _ = thread.join();
	}
}
