//! This crate contains helper macros for the `show-image` crate.
//! You should not depend on this crate directly.
//! Instead, enable the `macro` feature of the `show-image` crate,
//! which will report all macros at the crate root.

/// Wrap your program entry point for correct initialization of `show_image`.
///
/// The `show-image` context will run in the main thread,
/// and your own entry point will run in a new thread.
/// When the thread running your entry point terminates, the process will terminate with exit status 0.
/// Any other running threads will be killed at that point.
///
/// Your entry point must take a single [`show_image::ContextProxy`] argument.
#[proc_macro_attribute]
pub fn main(attribs: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	match details::main(attribs.into(), input.into()) {
		Ok(x) => x.into(),
		Err(e) => e.to_compile_error().into(),
	}
}

mod details {
	use quote::quote_spanned;
	use quote::quote;
	use syn::spanned::Spanned;

	pub fn main(arguments: proc_macro2::TokenStream, input: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
		if !arguments.is_empty() {
			return Err(syn::Error::new_spanned(arguments, "unexpected macro arguments"));
		}

		let function: syn::ItemFn = syn::parse2(input)?;
		let name = function.sig.ident.clone();
		if function.sig.inputs.len() != 1 {
			return Err(syn::Error::new_spanned(function.sig, "expected function with 1 argument"));
		}

		let context_arg = match &function.sig.inputs[0] {
			syn::FnArg::Typed(x) => x,
			syn::FnArg::Receiver(x) => return Err(syn::Error::new_spanned(x, "expected show_image::ContextProxy argument")),
		};

		let context_ident = get_arg_ident(&context_arg.pat).map_err(|_| syn::Error::new_spanned(context_arg,  "expected show_image::ContextProxy argument"))?;
		let context_ident = quote_spanned!(context_arg.span()=> #context_ident);

		Ok(quote! {
			fn main() {
				#function

				// Assert function type.
				let _: fn(show_image::ContextProxy) = #name;

				show_image::run_context(move |#context_ident| {
					#name(#context_ident);
					std::process::exit(0);
				}).unwrap();
			}
		})
	}

	fn get_arg_ident(arg: &syn::Pat) -> Result<syn::Ident, ()> {
		match arg {
			syn::Pat::Ident(x) => Ok(x.ident.clone()),
			syn::Pat::Reference(x) => get_arg_ident(&x.pat),
			_ => Err(()),
		}
	}
}
