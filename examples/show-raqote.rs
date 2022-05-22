use raqote::DrawOptions;
use raqote::DrawTarget;
use raqote::PathBuilder;
use raqote::StrokeStyle;
use show_image::event::ModifiersState;
use show_image::event::VirtualKeyCode;
use show_image::event::WindowEvent;
use show_image::Image;

#[show_image::main]
fn main() -> Result<(), String> {
	env_logger::init();

	let args: Vec<_> = std::env::args().collect();
	if args.len() != 1 {
		return Err(format!("usage: {}", args[0]));
	}

	let mut image = DrawTarget::new(1000, 1000);
	let mut overlay = DrawTarget::new(500, 1000);
	image.set_transform(&raqote::Transform::create_scale(1000.0, 1000.0));
	overlay.set_transform(&raqote::Transform::create_scale(1000.0, 1000.0));

	let black = raqote::Color::new(255, 0, 0, 0).into();
	let white = raqote::Color::new(255, 255, 255, 255).into();
	let red = raqote::Color::new(255, 190, 0, 0).into();
	let yellow = raqote::Color::new(255, 255, 215, 85).into();
	let blue = raqote::Color::new(255, 0, 50, 160).into();

	let draw_options = DrawOptions::new();

	image.fill_rect(0.0, 0.0, 1.0, 1.0, &white, &draw_options);

	image.fill_rect(0.00, 0.00, 0.25, 0.30, &red, &draw_options);
	image.fill_rect(0.00, 0.70, 0.25, 0.30, &blue, &draw_options);
	image.fill_rect(0.85, 0.70, 0.15, 0.30, &yellow, &draw_options);

	let mut path = PathBuilder::new();
	path.move_to(0.25, 0.00);
	path.line_to(0.25, 1.00);
	image.stroke(
		&path.finish(),
		&black,
		&StrokeStyle {
			width: 0.03,
			..Default::default()
		},
		&draw_options,
	);

	let mut path = PathBuilder::new();
	path.move_to(0.00, 0.30);
	path.line_to(0.25, 0.30);
	image.stroke(
		&path.finish(),
		&black,
		&StrokeStyle {
			width: 0.04,
			..Default::default()
		},
		&draw_options,
	);

	let mut path = PathBuilder::new();
	path.move_to(0.00, 0.70);
	path.line_to(1.00, 0.70);
	image.stroke(
		&path.finish(),
		&black,
		&StrokeStyle {
			width: 0.03,
			..Default::default()
		},
		&draw_options,
	);

	let mut path = PathBuilder::new();
	path.move_to(0.85, 0.70);
	path.line_to(0.85, 1.00);
	image.stroke(
		&path.finish(),
		&black,
		&StrokeStyle {
			width: 0.03,
			..Default::default()
		},
		&draw_options,
	);

	let mut path = PathBuilder::new();
	path.move_to(0.85, 0.70);
	path.line_to(0.85, 1.00);
	image.stroke(
		&path.finish(),
		&black,
		&StrokeStyle {
			width: 0.03,
			..Default::default()
		},
		&draw_options,
	);

	let mut path = PathBuilder::new();
	path.move_to(0.00, 0.00);
	path.line_to(1.00, 1.00);
	overlay.stroke(
		&path.finish(),
		&yellow,
		&StrokeStyle {
			width: 0.03,
			..Default::default()
		},
		&draw_options,
	);

	let image: Image = image.into();
	let image_view = image.as_image_view().map_err(|x| x.to_string())?;
	println!("{:#?}", image_view.info());
	let overlay: show_image::Image = overlay.into();

	let window = show_image::context().run_function_wait(move |context| -> Result<_, String> {
		let mut window = context.create_window("image", Default::default()).map_err(|e| e.to_string())?;
		window.set_image("mondriaan", &image.as_image_view().map_err(|e| e.to_string())?);
		window.add_overlay("overlay", &overlay.as_image_view().map_err(|e| e.to_string())?);
		Ok(window.proxy())
	})?;


	// Wait for the window to be closed or Escape to be pressed.
	for event in window.event_channel().map_err(|e| e.to_string())? {
		if let WindowEvent::KeyboardInput(event) = event {
			if event.is_synthetic || !event.input.state.is_pressed() {
				continue;
			}
			if event.input.key_code == Some(VirtualKeyCode::Escape) {
				println!("Escape pressed!");
				break;
			} else if event.input.key_code == Some(VirtualKeyCode::O) && event.input.modifiers == ModifiersState::CTRL {
				println!("Ctrl+O pressed, toggling overlay");
				window.run_function_wait(|mut window| {
					window.set_overlays_visible(!window.overlays_visible());
				}).map_err(|e| e.to_string())?;
			}
		}
	}

	Ok(())
}
