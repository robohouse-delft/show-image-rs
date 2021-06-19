//! Local version of [`std::process::Termination`] until it is stabilized.
//!
//! On `nightly`, this module just re-exports the real [`std::process::Termination`] trait.

#[cfg(feature = "nightly")]
mod contents {
	pub use std::process::Termination;
}

#[cfg(not(feature = "nightly"))]
mod contents {
	/// Dressed down version of [`std::process::Termination`].
	///
	/// This is used to allow user tasks to return `Result<(), E>` or just `()` on stable and beta.
	pub trait Termination {
		/// Print any messages to standard error and give the exit code for the process.
		fn report(self) -> i32;
	}

	impl Termination for () {
		fn report(self) -> i32 {
			0
		}
	}

	impl<E: std::fmt::Debug> Termination for Result<(), E> {
		fn report(self) -> i32 {
			if let Err(e) = self {
				eprintln!("Error: {:?}", e);
				1
			} else {
				0
			}
		}
	}
}

pub use contents::*;
