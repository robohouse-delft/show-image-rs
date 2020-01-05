use image;

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

	let mut show_image = show_image::Context::new()?;
	let window = show_image.make_window(show_image::WindowOptions {
		name: "image".into(),
		size: [800, 600],
		resizable: true,
		preserve_aspect_ratio: true,
	})?;
	window.set_image(&image)?;

	loop {
		if let Some(event) = window.wait_key(std::time::Duration::from_millis(100)) {
			println!("{:#?}", event);
			if event.key == show_image::KeyCode::Escape {
				break;
			}
		}
	}

	Ok(())
}
