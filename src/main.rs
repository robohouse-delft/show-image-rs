use show_image::WindowOptions;

#[show_image::main]
fn main(context: show_image::ContextProxy) {
	let args : Vec<_> = std::env::args().collect();
	let image = image::open(args.get(1).unwrap()).unwrap();

	context.add_event_handler(|_context, event, _control| {
		if let show_image::Event::UserEvent(show_image::AllWindowsClosed) = event {
			eprintln!("last window closed");
			std::process::exit(0);
		}
	});

	let window = context.create_window("Show Image", WindowOptions::default()).unwrap();
	window.set_image("image", image).unwrap();
	window.set_visible(true).unwrap();
	window.add_event_handler(|window, event, _control| {
		eprintln!("received event for window {:?}: {:#?}", window.id(), event);
	}).unwrap();

	loop {
		std::thread::sleep(std::time::Duration::from_secs(10));
	}
}
