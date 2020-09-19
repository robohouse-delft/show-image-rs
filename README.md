[![Docs.rs](https://docs.rs/show-image/badge.svg)](https://docs.rs/crate/show-image/)
[![CI](https://github.com/robohouse-delft/show-image-rs/workflows/CI/badge.svg)](https://github.com/robohouse-delft/show-image-rs/actions?query=workflow%3ACI+branch%3Amaster)

# show-image

`show-image` is a library for quickly displaying images.
It is intended as a debugging aid for writing image processing code.
The library is not intended for making full-featured GUIs,
but you can process keyboard events from the created windows.

## Supported image types.
The library aims to support as many different data types used to represent images.
To keep the dependency graph as small as possible,
support for third party libraries must be enabled explicitly with feature flags.

Currently, the following types are supported:
  * The `Image` and `ImageView` types from this crate.
  * `image::DynamicImage` and `image::ImageBuffer` (requires the `"image"` feature).
  * `tch::Tensor` (requires the `"tch"` feature).
  * `raqote::DrawTarget` and `raqote::Image` (requires the `"raqote"` feature).

If you think support for a some data type is missing,
feel free to send a PR or create an issue on GitHub.

## Global context and threading
The library uses a global context that runs an event loop.
This context must be initialized before any `show-image` functions can be used.
Additionally, some platforms require the event loop to be run in the main thread.
To ensure portability, the same restriction is enforced on all platforms.

The easiest way to initialize the global context and run the event loop in the main thread
is to use the `main` attribute macro on your main function.
If you want to run some code in the main thread before the global context takes over,
you can use the `run_context()` function or one of it's variations instead.
Note that you must still call those functions from the main thread,
and they do not return control back to the caller.

## Event handling.
You can register an event handler to run in the global context thread using `WindowProxy::add_event_handler()` or some of the similar functions.
You can also register an event handler directly with the context to handle global events (including all window events).
Since these event handlers run in the event loop, they should not block for any significant time.

You can also receive events using `WindowProxy::event_channel()` or `ContextProxy::event_channel()`.
These functions create a new channel for receiving window events or global events, respectively.
As long as you're receiving the events in your own thread, you can block as long as you like.

## Saving displayed images.
If the `save` feature is enabled, windows allow the displayed image to be saved using `Ctrl+S`.
This will open a file dialog to save the currently displayed image.

Note that images are saved in a background thread.
To ensure that no data loss occurs, call `exit()` to terminate the process rather than `std::process::exit()`.
That will ensure that the background threads are joined before the process is terminated.

## Example 1: Showing an image.
```rust
use show_image::{ImageView, ImageInfo, create_window};

#[show_image::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {

  let image = ImageView::new(ImageInfo::rgb8(1920, 1080), pixel_data);

  // Create a window with default options and display the image.
  let window = create_window("image", Default::default())?;
  window.set_image("image-001", image)?;

  Ok(())
}
```

## Example 2: Handling keyboard events using an event channel.
```rust
use show_image::{event, create_window};

// Create a window and display the image.
let window = create_window("image", Default::default())?;
window.set_image("image-001", &image)?;

// Print keyboard events until Escape is pressed, then exit.
// If the user closes the window, the channel is closed and the loop also exits.
for event in window.event_channel()? {
  if let event::WindowEvent::KeyboardInput(event) = event {
        println!("{:#?}", event);
        if event.input.key_code == Some(event::VirtualKeyCode::Escape) && event.input.state.is_pressed() {
            break;
        }
    }
}

```
