# show-image [![Docs.rs](https://docs.rs/com/badge.svg)](https://docs.rs/crate/show-image/)

`show-image` is a library for quickly displaying images.
It is intended as a debugging aid for writing image processing code.
The library is not intended for making full-featured GUIs,
but you can process keyboard events from the created windows.

## Supported image types.
The library aims to support as many different data types used to represent images.
To keep the dependency graph as small as possible,
support for third party libraries must be enabled explicitly with feature flags.

Currently, the following types are supported:
  * Tuples of binary data and `ImageInfo`.
  * `image::DynamicImage` and `image::ImageBuffer` with the `image` feature.
  * `tch::Tensor` with the `tch` feature.

If you think support for a some data type is missing,
feel free to send a PR or create an issue on GitHub.

## Keyboard events.
You can handle keyboard events for windows using `Window::wait_key` or `Window::wait_key_deadline`.
These functions will wait for key press events while discarding key up events.
Alternatively you can use `Window::events` to get direct access to a channel with all keyboard events.

Keyboard events are reported using types re-exported from the `keyboard-types` crate for easy interoperability with other crates.

## Example 1: Showing an image.
This example uses a tuple of `(&[u8], ImageInfo)` as image,
but any type that implements `ImageData` will do.
```rust
use show_image::{ImageInfo, make_window};

let image = (pixel_data, ImageInfo::rgb8(1920, 1080));

// Create a window and display the image.
let window = make_window("image")?;
window.set_image(image)?;

```

## Example 2: Handling keyboard events.
```rust
use show_image::{KeyCode, make_window};

#
// Create a window and display the image.
let window = make_window("image")?;
window.set_image(&image)?;

// Print keyboard events until Escape is pressed, then exit.
// If the user closes the window, wait_key() will return an error and the loop also exits.
while let Ok(event) = window.wait_key(Duration::from_millis(100)) {
    if let Some(event) = event {
        println!("{:#?}", event);
        if event.key == KeyCode::Escape {
            break;
        }
    }
}

```
