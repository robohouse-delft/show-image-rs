//! This crate contains helper macros for the `show-image` crate.
//! You should not depend on this crate directly.
//! Instead, enable the `macro` feature of the `show-image` crate and use them from there.

/// Wrap your program entry point for correct initialization of the `show-image` global context.
///
/// The `show-image` global context will run in the main thread,
/// and your own entry point will be executed in a new thread.
/// When the thread running your entry point terminates, the whole process will terminate with exit status 0.
/// Any other running threads will be killed at that point.
/// To exit with a different status code, just call [`std::process::exit`].
///
/// Note that we are very sorry about stealing your main thread.
/// We would rather let you keep the main thread and run the global context in a background thread.
/// However, some platforms require all GUI code to run in the "main" thread (looking at you, OS X).
/// To ensure portability, the same restriction is enforced on other platforms.
///
/// # Examples
///
/// ```no_run
/// use show_image::{ContextProxy, WindowOptions};
/// use image::Image;
///
/// #[show_image::main]
/// fn main() -> Result<(), String> {
///   let context = show_image::context();
///   let window = context
///     .create_window("My Awesome Window", WindowOptions::default())
///     .map_err(|e| e.to_string())?;
///
///   let image = Image::load("/path/to/image.png")
///     .map_err(|e| e.to_string())?;
///
///   window.set_image("image", image)
///     .map_err(|e| e.to_string())?;
///
///   // Tell the context to terminate the process when the last window closes,
///   // and then wait forever.
///   context.set_exit_with_last_window(true);
///   loop {
///     std::thread::park();
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn main(attribs: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	match details::main(attribs.into(), input.into()) {
		Ok(x) => x.into(),
		Err(e) => details::error_to_tokens(e).into(),
	}
}

mod details {
	use quote::quote;

	/// Convert a syn::Error into a compile error with a dummy main.
	///
	/// The dummy main prevents the compiler from complaining about a missing entry point, which is confusing noise.
	/// We only want the compiler to show the real error.
	pub fn error_to_tokens(error: syn::Error) -> proc_macro2::TokenStream {
		let error = error.to_compile_error();
		quote! {
			#error
			fn main() {
				panic!("#[show_image::main]: compilation should have failed, please report this bug");
			}
		}
	}

	pub fn main(arguments: proc_macro2::TokenStream, input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
		if !arguments.is_empty() {
			return Err(syn::Error::new_spanned(arguments, "unexpected macro arguments"));
		}

		let function: syn::ItemFn = syn::parse2(input)?;
		let name = function.sig.ident.clone();

		Ok(quote! {
			fn main() {
				#function
				::show_image::run_context(#name);
			}
		})
	}
}
