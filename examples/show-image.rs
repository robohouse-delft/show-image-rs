use show_image::ImageData;
use show_image::make_window;

fn main() -> Result<(), String> {
	let args : Vec<_> = std::env::args().collect();
	if args.len() != 2 {
		return Err(format!("usage: {} IMAGE", args[0]));
	}
	let path = &args[1];

	let image = image::open(path)
		.map_err(|e| format!("Failed to read image from {:?}: {}", path, e))?;
	println!("{:#?}", image.info());

	let window = make_window("image")?;
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
