#[cfg(any(test, feature = "image"))]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "image")))]
pub mod image;

#[cfg(any(test, feature = "raqote"))]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "raqote")))]
pub mod raqote;

#[cfg(any(test, feature = "tch"))]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "tch")))]
pub mod tch;
