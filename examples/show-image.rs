use image;
use show_image::make_window_defaults;

use std::path::Path;


fn read_png(path: impl AsRef<Path>) -> Result<image::DynamicImage, String> {
	let path = path.as_ref();
	image::open(path).map_err(|e| format!("Failed to read image from {:?}: {}", path, e))
}

fn main() -> Result<(), String> {
	let args : Vec<_> = std::env::args().collect();
	if args.len() != 2 {
		return Err(format!("usage: {} IMAGE", args[0]));
	}

	let image = read_png(&args[1])?;

	let window = make_window_defaults("image")?;
	window.set_image(&image)?;

	while let Ok(event) = window.wait_key(std::time::Duration::from_millis(100)) {
		if let Some(event) = event {
			println!("{:#?}", event);
			if event.key == show_image::KeyCode::Escape {
				break;
			}
		}
	}

	Ok(())
}
