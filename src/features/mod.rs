#[cfg(feature = "image")]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "image")))]
pub mod image;

#[cfg(feature = "raqote")]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "raqote")))]
pub mod raqote;

#[cfg(feature = "tch")]
#[cfg_attr(feature = "nightly", doc(cfg(feature = "tch")))]
pub mod tch;
