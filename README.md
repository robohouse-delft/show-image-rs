# show-image

`show-image` is a library for quickly displaying images.
It is intended as a debugging aid for writing image processing code.
The library is not inteded to be used for writing full-featured GUIs,
although you can process keyboard events from the created windows.

It is the goal of the library to support as many different data types used to represent images.
To prevent dependency bloat and unreasonable compile times, feature flags can be used to enable support for third party libraries.
If you think support for a specific data type is missing, feel free to send a PR or create an issue on GitHub.

## Global context or manually created context.
The library uses a `Context` object to manage an event loop running in a background thread.
You can manually create such a context, or you can use the module functions `make_window` and `make_window_full`.
The free functions will use a global context that is initialized when needed.

Only one `Context` object can ever be created, so you can not mix the free functions with a manually created context.

## Keyboard events
You can handle keyboard events for windows.
You can use `Window::wait_key` or `Window::wait_key_deadline` to wait for key press events.
Alternatively you can use `Window::events` to get direct access to the channel where all keyboard events are sent (including key release events).


## Example 1: Using the global context.
```rust
use show_image::make_window;
use show_image::KeyCode;

let image = read_image("/path/to/image.png")?;

// Create a window and display the image.
let window = make_window("image")?;
window.set_image(&image)?;

// Print keyboard events until Escape is pressed, then exit.
while let Ok(event) = window.wait_key(Duration::from_millis(100)) {
    if let Some(event) = event {
        println!("{:#?}", event);
        if event.key == KeyCode::Escape {
            break;
        }
    }
}

```

## Example 2: Using a manually created context.

Alternatively, you can manually create a `Context` and use that to create a window.
This avoids using global state, but it requires you to pass a context everywhere in your code.

```rust
use show_image::Context;
let context = Context::new()?;
let window = context.make_window("image")?;
window.set_image(&image)?;
```
