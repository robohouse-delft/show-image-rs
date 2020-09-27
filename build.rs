use rustc_version::{version_meta, Channel};

fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	if version_meta().unwrap().channel <= Channel::Nightly {
		println!("cargo:rustc-cfg=nightly");
	}
}
