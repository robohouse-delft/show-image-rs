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

pub fn make_window(options: crate::WindowOptions) -> Result<Window, String> {
	context()?.make_window(options)
}

pub fn make_window_defaults(name: impl Into<String>) -> Result<Window, String> {
	context()?.make_window_defaults(name)
}
