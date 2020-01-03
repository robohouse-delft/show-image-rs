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
	let window = show_image.window("image")?;
	window.show(&image)?;

	let window2 = show_image.window("image2")?;
	window2.show(&image)?;

	show_image.run()?;

	Ok(())
}
