mod context;
mod event;
mod mouse_cache;
mod proxy;
mod util;
mod window;

pub use context::ContextHandle;
pub use proxy::ContextProxy;
pub use proxy::WindowProxy;
pub use window::WindowHandle;
pub use window::WindowOptions;

use crate::error;
use context::Context;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::panic::{AssertUnwindSafe, catch_unwind};

static CONTEXT_PROXY_VALID: AtomicBool = AtomicBool::new(false);
static mut CONTEXT_PROXY: Option<ContextProxy> = None;

/// Initialize the global context.
fn initialize_context() -> Result<Context, error::GetDeviceError> {
	let context = Context::new(wgpu::TextureFormat::Bgra8Unorm)?;
	unsafe {
		CONTEXT_PROXY = Some(context.proxy.clone());
	}
	CONTEXT_PROXY_VALID.store(true, Ordering::Release);
	Ok(context)
}

/// Initialize the global context, or exit the process.
fn initialize_context_or_exit() -> Context {
	match initialize_context() {
		Ok(x) => x,
		Err(crate::error::GetDeviceError::NoSuitableDeviceFound(e)) => {
			eprintln!("show-image: Failed to find a suitable device: {}. Terminating process.", e);
			std::process::exit(-1);
		},
		Err(crate::error::GetDeviceError::NoSuitableAdapterFound(_)) => {
			eprintln!("show-image: Failed to find a suitable graphics adapter. Terminating process.");
			#[cfg(any(target_os = "android", target_os = "linux"))]
			eprintln!("show-image: You may be missing the correct driver. Consider installing the Vulkan driver for your GPU.");
			std::process::exit(-2);
		}
	}
}

/// Initialize and run the global context and spawn a user task in a new thread.
///
/// This function never returns.
/// Once the user task finishes, the program exits with status code 0.
/// Any background threads spawned by `show-image` will be joined before the process exits.
/// It is the responsibility of the user code to join any manually spawned tasks.
///
/// The user task can call the [`context()`] function to obtain a [`ContextProxy`],
/// or the [`create_window()`] function to create a new window directly.
///
/// If the `macros` feature is enabled, you can also wrap your main function with the [`main`][crate::main] macro
/// instead of manually calling this function.
///
/// It is also possible to run a user task in the same thread as the context.
/// See [`run_context_with_local_task()`] for more details.
///
/// # Panics
/// This function panics if initialization of the global context fails.
/// See [`try_run_context`] for a variant that allows the user task to handle these initialization errors.
///
/// This function also panics if it is called from any thread other than the main thread.
/// Some platforms like OS X require all GUI code to run in the main thread.
/// To ensure portability, this restriction is also enforced on other platforms.
pub fn run_context<F, R>(user_task: F) -> !
where
	F: FnOnce() -> R + Send + 'static,
	R: crate::termination::Termination,
{
	let context = initialize_context_or_exit();

	// Spawn the user task.
	std::thread::spawn(move || {
		match catch_unwind(AssertUnwindSafe(user_task)) {
			Ok(termination) => exit(termination.report()),
			Err(_) => {
				// Make sure the main thread panics too.
				crate::context().run_function(move |_| {
					panic!("show-image: main user task panicked");
				});
			},
		}
	});

	context.run();
}

/// Initialize and run the global context and spawn a user task in a new thread.
///
/// This function is almost identical to [`run_context`],
/// except that it allows the user task to handle initialization errors.
/// If the initialization of the global context fails, the user task will be executed in the calling thread.
/// If initialization succeeds, the user task is started in a newly spawned thread.
///
/// Whether or not initialization succeeded, the process will exit once the user task returns.
/// Any background threads spawned by `show-image` will be joined before the process exits.
/// It is the responsibility of the user code to join any manually spawned tasks.
///
/// It is also possible to run a user task in the same thread as the context.
/// See [`try_run_context_with_local_task()`] for more details.
///
/// # Panics
/// This function panics if it is called from any thread other than the main thread.
/// Some platforms like OS X require all GUI code to run in the main thread.
/// To ensure portability, this restriction is also enforced on other platforms.
pub fn try_run_context<F, R>(user_task: F) -> !
where
	F: FnOnce(Result<(), error::GetDeviceError>) -> R + Send + 'static,
	R: crate::termination::Termination,
{
	let context = match initialize_context() {
		Ok(x) => x,
		Err(e) => {
			let termination = (user_task)(Err(e));
			std::process::exit(termination.report());
		},
	};

	// Spawn the user task.
	std::thread::spawn(move || {
		match catch_unwind(AssertUnwindSafe(move || user_task(Ok(())))) {
			Ok(termination) => exit(termination.report()),
			Err(_) => {
				// Make sure the main thread panics too.
				crate::context().run_function(move |_| {
					panic!("show-image: main user task panicked");
				});
			},
		}
	});

	context.run();
}

/// Initialize and run the global context and run a user task, both in the main thread.
///
/// The global context will execute the user function in the main thread after the context is fully initialized.
///
/// This function never returns.
/// The global context will keep running after the local task finishes.
/// It is up to the user code to call [`std::process::exit`] when the process should exit.
/// Alternatively, you could call [`ContextHandle::set_exit_with_last_window`].
///
/// *Note*:
/// You should not run a function that blocks for any significant time in the main thread.
/// Doing so will prevent the event loop from processing events and will result in unresponsive windows.
///
/// If you're looking for a place to run your own application code,
/// you probably want to use [`run_context`] or the [`main`][crate::main] macro.
/// However, if you can drive your entire application from event handlers,
/// then this function is probably what you're looking for.
///
/// # Panics
/// This function panics if initialization of the global context fails.
/// See [`try_run_context_with_local_task`] for a variant that allows the user task to handle initialization errors.
///
/// This function also panics if it is called from any thread other than the main thread.
/// Some platforms like OS X require all GUI code to run in the main thread.
/// To ensure portability, this restriction is also enforced on other platforms.
pub fn run_context_with_local_task<F>(user_task: F) -> !
where
	F: FnOnce(&mut ContextHandle) + Send + 'static,
{
	let context = initialize_context_or_exit();

	// Queue the user task.
	// It won't be executed until context.run() is called.
	context.proxy.run_function(user_task);
	context.run();
}

/// Initialize and run the global context and run a user task, both in the main thread.
///
/// This function is almost identical to [`run_context_with_local_task`],
/// except that it allows the user task to handle initialization errors.
/// If the initialization of the global context fails, the process will terminate when the user task returns.
/// Otherwise, the global context will continue running the event loop in the main thread.
///
/// # Panics
/// This function panics if it is called from any thread other than the main thread.
/// Some platforms like OS X require all GUI code to run in the main thread.
/// To ensure portability, this restriction is also enforced on other platforms.
pub fn try_run_context_with_local_task<F>(user_task: F) -> !
where
	F: FnOnce(Result<&mut ContextHandle, error::GetDeviceError>) + Send + 'static,
{
	let context = match initialize_context() {
		Ok(x) => x,
		Err(e) => {
			(user_task)(Err(e));
			std::process::exit(0);
		},
	};

	// Queue the user task.
	// It won't be executed until context.run() is called.
	context.proxy.run_function(|context| user_task(Ok(context)));
	context.run();
}

/// Get the global context to interact with existing windows or to create new windows.
///
/// If you manually spawn threads that try to access the context before calling `run_context`, you introduce a race condition.
/// Instead, you should pass a function to [`run_context`] or one of the variants.
/// Those functions take care to initialize the global context before running the user code.
///
/// # Panics
/// This panics if the global context is not yet fully initialized.
pub fn context() -> ContextProxy {
	if !CONTEXT_PROXY_VALID.load(Ordering::Acquire) {
		panic!("show-image: global context is not yet fully initialized");
	}
	unsafe { CONTEXT_PROXY.clone().unwrap() }
}

/// Create a new window with the global context.
///
/// If you manually spawn threads that try to access the context before calling `run_context`, you introduce a race condition.
/// Instead, you should pass a function to [`run_context`] that will be started in a new thread after the context is initialized.
///
/// # Panics
/// This panics if the global context is not yet fully initialized.
pub fn create_window(title: impl Into<String>, options: WindowOptions) -> Result<WindowProxy, error::CreateWindowError> {
	let title = title.into();
	context().run_function_wait(move |context| {
		let window = context.create_window(title, options)?;
		Ok(window.proxy())
	})
}

/// Join all background tasks and then exit the process.
///
/// If you use [`std::process::exit`], running background tasks may be killed.
/// To ensure no data loss occurs, you should use this function instead.
///
/// Background tasks are spawned when an image is saved through the built-in Ctrl+S or Ctrl+Shift+S shortcut, or by user code.
pub fn exit(code: i32) -> ! {
	context().exit(code);
}
