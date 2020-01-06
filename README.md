# show-image

`show-image` is a library for quickly displaying images.
It is intended as a debugging aid for writing image processing code.
The library is not intended to be used for writing full-featured GUIs,
although you can process keyboard events from the created windows.

## Supported image types.
It is the goal of the library to support as many different data types used to represent images.
To prevent dependency bloat and unreasonable compile times, feature flags can be used to enable support for third party libraries.

Currently, the following types are supported:
  * Tuples of binary data and an `ImageInfo`.
  * `image::DynamicImage` and `image::ImageBuffer` with the `image` feature.
  * `tch::Tensor` with the `tch` feature.

If you think support for a specific data type is missing, feel free to send a PR or create an issue on GitHub.

## Global context or manually created context.
The library uses a `Context` object to manage an event loop running in a background thread.
You can manually create such a context, or you can use the global functions `make_window` and `make_window_full`.
These free functions will use a global context that is initialized when needed.

Only one `Context` object can ever be created, so you can not mix the free functions with a manually created context.

## Keyboard events.
You can handle keyboard events for windows.
You can use `Window::wait_key` or `Window::wait_key_deadline` to wait for key press events.
Alternatively you can use `Window::events` to get direct access to the channel where all keyboard events are sent (including key release events).

Keyboard events are reported using types re-exported from the `keyboard-types` crate for easy interoperability with other crates.


## Example 1: Using the global context.
This example uses a tuple of `(&[u8], ``ImageInfo``)` as image,
but any type that implements `ImageData` will do.
```rust
use show_image::{ImageInfo, make_window};

let image = (pixel_data, ImageInfo::rgb8(1920, 1080));

// Create a window and display the image.
let window = make_window("image")?;
window.set_image(image)?;

```

## Example 2: Using a manually created context.

Alternatively, you can manually create a `Context` and use that to create a window.
This avoids using global state, but since you can only create one context,
you will have to pass the context everywhere in your code.

```rust
use show_image::Context;

let context = Context::new()?;
let window = context.make_window("image")?;
window.set_image(&image)?;
```

## Example 3: Handling keyboard events.
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
