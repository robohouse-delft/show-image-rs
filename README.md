[![Docs.rs](https://docs.rs/show-image/badge.svg)](https://docs.rs/crate/show-image/)
[![CI](https://github.com/robohouse-delft/show-image-rs/workflows/CI/badge.svg)](https://github.com/robohouse-delft/show-image-rs/actions?query=workflow%3ACI+branch%3Amain)

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
  * The [`Image`] and [`ImageView`] types from this crate.
  * [`image::DynamicImage`][::image::DynamicImage] and [`image::ImageBuffer`][::image::ImageBuffer] (requires the `"image"` feature).
  * [`tch::Tensor`][::tch::Tensor] (requires the `"tch"` feature).
  * [`raqote::DrawTarget`][::raqote::DrawTarget] and [`raqote::Image`][::raqote::Image] (requires the `"raqote"` feature).

If you think support for a some data type is missing,
feel free to send a PR or create an issue on GitHub.

## Global context and threading
The library uses a global context that runs an event loop.
This context must be initialized before any `show-image` functions can be used.
Additionally, some platforms require the event loop to be run in the main thread.
To ensure portability, the same restriction is enforced on all platforms.

The easiest way to initialize the global context and run the event loop in the main thread
is to use the [`main`] attribute macro on your main function.
If you want to run some code in the main thread before the global context takes over,
you can use the [`run_context()`] function or one of it's variations instead.
Note that you must still call those functions from the main thread,
and they do not return control back to the caller.

## Event handling.
You can register an event handler to run in the global context thread using [`WindowProxy::add_event_handler()`] or some of the similar functions.
You can also register an event handler directly with the context to handle global events (including all window events).
Since these event handlers run in the event loop, they should not block for any significant time.

You can also receive events using [`WindowProxy::event_channel()`] or [`ContextProxy::event_channel()`].
These functions create a new channel for receiving window events or global events, respectively.
As long as you're receiving the events in your own thread, you can block as long as you like.

## Saving displayed images.
If the `save` feature is enabled, windows allow the displayed image to be saved using `Ctrl+S` or `Ctrl+Shift+S`.
The first shortcut will open a file dialog to save the currently displayed image.
The second shortcut will directly save the image in the current working directory using the name of the image.

The image is saved without any overlays.
To save an image including overlays, add `Alt` to the shortcut: `Ctrl+Alt+S` and `Ctrl+Alt+Shift+S`.

Note that images are saved in a background thread.
To ensure that no data loss occurs, call [`exit()`] to terminate the process rather than [`std::process::exit()`].
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

## Back-end and GPU selection

This crate uses [`wgpu`] for rendering.
You can force the selection of a specfic WGPU backend by setting the `WGPU_BACKEND` environment variable to one of the supported values:

* `primary`: Use the primary backend for the platform (the default).
* `vulkan`: Use the vulkan back-end.
* `metal`: Use the metal back-end.
* `dx12`: Use the DirectX 12 back-end.
* `dx11`: Use the DirectX 11 back-end.
* `gl`: Use the OpenGL back-end.
* `webgpu`: Use the browser WebGPU back-end.

You can also influence the GPU selection by setting the `WGPU_POWER_PREF` environment variable:

* `low`: Prefer a low power GPU (the default).
* `high`: Prefer a high performance GPU.

[`Image`]: https://docs.rs/show-image/latest/show_image/enum.Image.html
[`ImageView`]: https://docs.rs/show-image/latest/show_image/struct.ImageView.html
[::image::DynamicImage]: https://docs.rs/image/latest/image/dynimage/enum.DynamicImage.html
[::image::ImageBuffer]: https://docs.rs/image/latest/image/buffer_/struct.ImageBuffer.html
[::tch::Tensor]: https://docs.rs/tch/latest/tch/wrappers/tensor/struct.Tensor.html
[::raqote::DrawTarget]: https://docs.rs/raqote/latest/raqote/struct.DrawTarget.html
[::raqote::Image]: https://docs.rs/raqote/latest/raqote/struct.Image.html
[`main`]: https://docs.rs/show-image/latest/show_image/attr.main.html
[`run_context()`]: https://docs.rs/show-image/latest/show_image/fn.run_context.html
[`WindowProxy::add_event_handler()`]: https://docs.rs/show-image/latest/show_image/struct.WindowProxy.html#method.add_event_handler
[`WindowProxy::event_channel()`]: https://docs.rs/show-image/latest/show_image/struct.WindowProxy.html#method.event_channel
[`ContextProxy::event_channel()`]: https://docs.rs/show-image/latest/show_image/struct.ContextProxy.html#method.event_channel
[`exit()`]: https://docs.rs/show-image/latest/show_image/fn.exit.html
[`std::process::exit()`]: https://doc.rust-lang.org/nightly/std/process/fn.exit.html
[`wgpu`]: https://docs.rs/wgpu
