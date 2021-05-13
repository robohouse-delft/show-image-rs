//! This crate contains helper macros for the `show-image` crate.
//! You should not depend on this crate directly.
//! Instead, enable the `macro` feature of the `show-image` crate and use them from there.

/// Wrap your program entry point for correct initialization of the `show-image` global context.
///
/// The `show-image` global context will run in the main thread,
/// and your own entry point will be executed in a new thread.
/// When your entry point returns, the whole process will exit.
///
/// Any background tasks spawned by `show-image` will be joined before the process terminates,
/// but any threads spawned by user code will be killed.
/// You should manually join those if needed.
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
/// # use std::error::Error;
/// # mod image {
/// #   pub fn open(_path: impl AsRef<std::path::Path>) -> Result<show_image::ImageView<'static>, std::io::Error> {
/// #     Ok(show_image::ImageView::new(show_image::ImageInfo::rgb8(1, 1), &[255, 0, 0]))
/// #   }
/// # }
///
/// #[show_image::main]
/// fn main() -> Result<(), Box<dyn Error>> {
///   let window = show_image::create_window("My Awesome Window", WindowOptions::default())?;
///   let image = image::open("/path/to/image.png")?;
///
///   window.set_image("image", image)?;
///   window.wait_until_destroyed()?;
///   Ok(())
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
