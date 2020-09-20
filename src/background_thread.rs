use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct BackgroundThread<T> {
	done: Arc<AtomicBool>,
	handle: std::thread::JoinHandle<T>,
}

impl<T> BackgroundThread<T> {
	pub fn new<F>(f: F) -> Self
	where
		F: FnOnce() -> T,
		F: Send + 'static,
		T: Send + 'static,
	{
		let done = Arc::new(AtomicBool::new(false));
		let handle = std::thread::spawn({
			let done = done.clone();
			move || {
				let result = f();
				done.store(true, Ordering::Release);
				result
			}
		});

		Self { done, handle }
	}

	pub fn is_done(&self) -> bool {
		self.done.load(Ordering::Acquire)
	}

	pub fn join(self) -> std::thread::Result<T> {
		self.handle.join()
	}
}
