use raqote::DrawOptions;
use raqote::DrawTarget;
use raqote::PathBuilder;
use raqote::StrokeStyle;
use show_image::event;
use show_image::Image;

#[show_image::main]
fn main() -> Result<(), String> {
	let args: Vec<_> = std::env::args().collect();
	if args.len() != 1 {
		return Err(format!("usage: {}", args[0]));
	}

	let mut image   = DrawTarget::new(1000, 1000);
	let mut overlay = DrawTarget::new(500, 1000);
	image.set_transform(&raqote::Transform::create_scale(1000.0, 1000.0));
	overlay.set_transform(&raqote::Transform::create_scale(1000.0, 1000.0));

	let black  = raqote::Color::new(255,   0,   0,   0).into();
	let white  = raqote::Color::new(255, 255, 255, 255).into();
	let red    = raqote::Color::new(255, 190,   0,   0).into();
	let yellow = raqote::Color::new(255, 255, 215,  85).into();
	let blue   = raqote::Color::new(255,   0,  50, 160).into();

	let draw_options = DrawOptions::new();

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

	let mut path = PathBuilder::new();
	path.move_to(0.85, 0.70);
	path.line_to(0.85, 1.00);
	image.stroke(&path.finish(), &black, &StrokeStyle { width: 0.03, ..Default::default() }, &draw_options);

	let mut path = PathBuilder::new();
	path.move_to(0.00, 0.00);
	path.line_to(1.00, 1.00);
	overlay.stroke(&path.finish(), &yellow, &StrokeStyle { width: 0.03, ..Default::default() }, &draw_options);

	let image : Image = image.into();
	let image_view = image.as_image_view().map_err(|x| x.to_string())?;
	println!("{:#?}", image_view.info());

	let window = show_image::create_window("image", Default::default()).map_err(|e| e.to_string())?;
	window.set_image("mondriaan", image).map_err(|e| e.to_string())?;
	window.add_overlay("overlay", overlay).map_err(|e| e.to_string())?;

	// Wait for the window to be closed or Escape to be pressed.
	for event in window.event_channel().map_err(|e| e.to_string())? {
		if let event::WindowEvent::KeyboardInput(event) = event {
			if event.is_synthetic {
				continue;
			}
			if event.input.key_code == Some(event::VirtualKeyCode::Escape) && event.input.state.is_pressed() {
				println!("Escape pressed!");
				break;
			} else if event.input.key_code == Some(event::VirtualKeyCode::O) && event.input.state.is_pressed() && event.input.modifiers == show_image::event::ModifiersState::CTRL {
				println!("Ctrl+O pressed, toggling overlay");
				window.set_options(|options| options.clone().set_show_overlays(!options.show_overlays)).map_err(|e| e.to_string())?;
			}
		}
	}

	Ok(())
}
