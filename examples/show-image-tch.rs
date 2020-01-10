use show_image::ImageData;
use show_image::make_window;
use show_image::tch::TensorAsImage;

fn main() -> Result<(), String> {
	let args : Vec<_> = std::env::args().collect();
	if args.len() != 2 {
		return Err(format!("usage: {} IMAGE", args[0]));
	}

	let tensor = tch::vision::imagenet::load_image(&args[1])
		.map_err(|e| format!("failed to load image from {:?}: {}", &args[1], e))?;
	println!("{:#?}", image.info());

	let window = make_window("image")?;
	window.set_image(tensor.as_image_guess_rgb())?;

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
