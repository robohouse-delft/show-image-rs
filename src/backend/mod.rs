pub mod context;
pub mod event;
pub mod proxy;
pub mod util;
pub mod window;

pub use context::Context;
pub use context::ContextHandle;
pub use proxy::ContextProxy;
pub use proxy::WindowProxy;
pub use window::Window;
pub use window::WindowHandle;
pub use window::WindowOptions;

use crate::error;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

static CONTEXT_PROXY_VALID: AtomicBool = AtomicBool::new(false);
static mut CONTEXT_PROXY: Option<ContextProxy> = None;

/// Initialize the global context.
fn initialize_context() -> Result<Context, error::GetDeviceError> {
	let context = Context::new(wgpu::TextureFormat::Bgra8UnormSrgb)?;
	unsafe {
		CONTEXT_PROXY = Some(context.proxy());
	}
	CONTEXT_PROXY_VALID.store(true, Ordering::Release);
	Ok(context)
}

/// Initialize and run the global context and spawn a user task in a new thread.
///
/// This function only returns if it fails.
/// If the context is stopped, the calling thread is terminated.
pub fn run_context<F>(user_task: F) -> Result<(), error::GetDeviceError>
where
	F: FnOnce(ContextProxy) + Send + 'static,
{
	let context = initialize_context()?;

	// Spawn the user task.
	let proxy = context.proxy();
	std::thread::spawn(move || {
		(user_task)(proxy);
	});

	context.run();
}

/// Initialize and run the global context, and run a user task in the same thread.
///
/// This function only returns if it fails.
/// If the context is stopped, the calling thread is terminated.
///
/// *Note*:
/// You should not run a function that blocks for any significant time in the context thread.
/// Doing so will prevent the event loop from processing events and will result in unresponsive windows.
///
/// If you're looking for a place to run your own application code,
/// you probably want to use [`run_context`].
/// But if you can drive your entire application from event handlers,
/// then this function is probably what you're looking for.
pub fn run_context_with_local_task<F>(user_task: F) -> Result<(), error::GetDeviceError>
where
	F: FnOnce(&mut ContextHandle) + Send + 'static,
{
	let context = initialize_context()?;

	// Queue the user task.
	// Unwrap should be safe, the event loop hasn't even started yet, so it can't be closed yet either.
	context.proxy().run_function(user_task).unwrap();

	context.run();
}

/// Get the global context.
///
/// If you manually spawn threads that try to access the context before calling `run_context`, you introduce a race condition.
/// Instead, you should pass a function to [`run_context`] that will be started in a new thread after the context is initialized.
///
/// # Panics
/// This panics if the global context is not yet fully initialized.
pub fn context() -> ContextProxy {
	if !CONTEXT_PROXY_VALID.load(Ordering::Acquire) {
		panic!("show-image: global context is not yet fully initialized");
	}
	unsafe {
		CONTEXT_PROXY.clone().unwrap()
	}
}

/// Create a window with the global context.
///
/// If you manually spawn threads that try to access the context before calling `run_context`, you introduce a race condition.
/// Instead, you should pass a function to [`run_context`] that will be started in a new thread after the context is initialized.
///
/// # Panics
/// This panics if the global context is not yet fully initialized.
pub fn create_window(title: impl Into<String>, options: WindowOptions) -> Result<WindowProxy, error::ProxyCreateWindowError> {
	context().create_window(title, options)
}

/// Exit the program with the given status code.
///
/// The actual exit will be performed after queued events have been processed.
/// This allows all queued actions to be performed before the exit happends.
///
/// You may also just call [`std::process::exit`] instead.
/// That will not wait for queued events and functions to be handled.
/// Whether or not that is a problem depends on your application.
///
/// You are encouraged to perform important operations in your own thread and just call [`std::process::exit`].
/// That will avoid problems where queued functions might take too long to finish and prevent the process from exitting.
///
/// # Panics
/// This panics if the global context is not yet fully initialized.
pub fn exit(status: i32) -> ! {

	// Using a global mutex and condition variable is too much hassle, so we just poll.
	let sleep_increment = std::time::Duration::from_millis(5);
	let sleep_max = std::time::Duration::from_millis(100);
	let mut sleep_duration = sleep_increment;

	loop {
		if let Err(crate::error::EventLoopClosedError) = context().exit(status) {
			std::process::exit(status);
		}

		std::thread::sleep(sleep_duration);
		if sleep_duration < sleep_max {
			sleep_duration += sleep_increment;
		}
	}
}
