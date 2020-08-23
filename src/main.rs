use show_image::WindowOptions;

fn main() {
	let args : Vec<_> = std::env::args().collect();
	let image = image::open(args.get(1).unwrap()).unwrap();

	show_image::run_context_with_local_task(move |context| {
		eprintln!("queued function running!");

		context.add_event_handler(|context, event, _control| {
			if let show_image::Event::UserEvent(show_image::AllWindowsClosed) = event {
				eprintln!("last window closed");
				context.stop();
			}
		});

		let mut window = context.create_window("Show Image", WindowOptions::default()).unwrap();
		window.set_image("image", &image).unwrap();
		window.set_visible(true).unwrap();
		window.add_event_handler(|window, event, _control| {
			eprintln!("received event for window {:?}: {:#?}", window.id(), event);
		}).unwrap();
	}).unwrap();
}
