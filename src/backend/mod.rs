use std::sync::Once;

static INIT: Once = Once::new();
static mut CONTEXT: Option<Result<Context, String>> = None;

mod sdl;
pub use sdl::Context;
pub use sdl::Window;

pub fn context() -> Result<&'static Context, &'static str> {
	unsafe {
		INIT.call_once(|| {
			let context = Context::new()
				.map_err(|e| format!("failed to initialize global context: {}", e));
			CONTEXT = Some(context);
		});
		match CONTEXT.as_ref().expect("global context not initialized") {
			Ok(ref x) => Ok(x),
			Err(ref e) => Err(e),
		}
	}
}

/// Make a window with default options using the global context.
///
/// See [`Context::make_window`] for more details.
pub fn make_window(name: impl Into<String>) -> Result<Window, String> {
	context()?.make_window(name)
}

/// Make a window with the given options using the global context.
///
/// See [`Context::make_window_full`] for more details.
pub fn make_window_full(options: crate::WindowOptions) -> Result<Window, String> {
	context()?.make_window_full(options)
}
