use raqote::DrawOptions;
use raqote::DrawTarget;
use raqote::PathBuilder;
use raqote::SolidSource;
use raqote::Source;
use raqote::StrokeStyle;
use show_image::ImageData;
use show_image::make_window;

fn main() -> Result<(), String> {
	let args: Vec<_> = std::env::args().collect();
	if args.len() != 1 {
		return Err(format!("usage: {}", args[0]));
	}

	let mut image = DrawTarget::new(1920, 1080);

	let black  = Source::Solid(SolidSource::from_unpremultiplied_argb(255,   0,   0,   0));
	let white  = Source::Solid(SolidSource::from_unpremultiplied_argb(255, 255, 255, 255));
	let red    = Source::Solid(SolidSource::from_unpremultiplied_argb(255, 190,   0,   0));
	let yellow = Source::Solid(SolidSource::from_unpremultiplied_argb(255, 255, 215,  85));
	let blue   = Source::Solid(SolidSource::from_unpremultiplied_argb(255,   0,  50, 160));

	let draw_options = DrawOptions::new();

	image.set_transform(&raqote::Transform::create_scale(1920.0, 1080.0));
	image.fill_rect(0.0, 0.0, 1.0, 1.0, &white, &draw_options);

	image.fill_rect(0.00, 0.00, 0.25, 0.30, &red,    &draw_options);
	image.fill_rect(0.00, 0.70, 0.25, 0.30, &blue,   &draw_options);
	image.fill_rect(0.85, 0.70, 0.15, 0.30, &yellow, &draw_options);

	let mut path = PathBuilder::new();
	path.move_to(0.25, 0.00);
	path.line_to(0.25, 1.00);
	image.stroke(&path.finish(), &black, &StrokeStyle { width: 0.03, ..Default::default() }, &draw_options);

	let mut path = PathBuilder::new();
	path.move_to(0.00, 0.30);
	path.line_to(0.25, 0.30);
	image.stroke(&path.finish(), &black, &StrokeStyle { width: 0.04, ..Default::default() }, &draw_options);

	let mut path = PathBuilder::new();
	path.move_to(0.00, 0.70);
	path.line_to(1.00, 0.70);
	image.stroke(&path.finish(), &black, &StrokeStyle { width: 0.03, ..Default::default() }, &draw_options);

	let mut path = PathBuilder::new();
	path.move_to(0.85, 0.70);
	path.line_to(0.85, 1.00);
	image.stroke(&path.finish(), &black, &StrokeStyle { width: 0.03, ..Default::default() }, &draw_options);

	println!("{:#?}", image.info());

	let window = make_window("image")?;
	window.set_image(image, "mondriaan")?;

	while let Ok(event) = window.wait_key(std::time::Duration::from_millis(100)) {
		if let Some(event) = event {
			println!("{:#?}", event);
			if event.key == show_image::KeyCode::Escape {
				break;
			}
		}
	}

	show_image::stop()?;
	Ok(())
}
