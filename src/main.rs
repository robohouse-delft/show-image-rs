use show_image::Context;
use show_image::ContextProxy;
use show_image::WindowOptions;
use show_image::ImageData;

fn main() {
	let args : Vec<_> = std::env::args().collect();
	let image = image::open(args.get(1).unwrap()).unwrap();
	let image = image.into_image().unwrap();

	let context = Context::new(wgpu::TextureFormat::Bgra8UnormSrgb).unwrap();
	let proxy = context.proxy();

	std::thread::spawn(move || fake_main(image, proxy));
	context.run();
}

fn fake_main(image: show_image::Image<'static>, proxy: ContextProxy) {
	proxy.run_function_wait(move |context| {
		eprintln!("queued function running!");
		let mut window = context.create_window("Show Image", WindowOptions::default()).unwrap();
		window.set_image("image", &image).unwrap();
		window.set_visible(true).unwrap();
		window.add_event_handler(|window, event| {
			eprintln!("received event for window {:?}: {:#?}", window.id(), event);
			Default::default()
		}).unwrap();
	}).unwrap();
}
