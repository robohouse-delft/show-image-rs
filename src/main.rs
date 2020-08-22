use show_image::WindowOptions;
use show_image::ImageData;

fn main() {
	let args : Vec<_> = std::env::args().collect();
	let image = image::open(args.get(1).unwrap()).unwrap();
	let image = image.into_image().unwrap();

	show_image::run_context_with_local_task(move |context| {
		eprintln!("queued function running!");
		let mut window = context.create_window("Show Image", WindowOptions::default()).unwrap();
		window.set_image("image", &image).unwrap();
		window.set_visible(true).unwrap();
		window.add_event_handler(|window, event, _control| {
			eprintln!("received event for window {:?}: {:#?}", window.id(), event);
		}).unwrap();
	}).unwrap();
}
