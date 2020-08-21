use show_image::Context;
use show_image::ContextProxy;
use show_image::ContextHandle;
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

fn fake_main(image: show_image::Image<'static>, proxy: ContextProxy<()>) {
	proxy.execute_function(move |mut context: ContextHandle<()>| {
		eprintln!("Making new window.");
		let window_id = context.create_window("Show Image", WindowOptions::default()).unwrap();
		eprintln!("Setting image.");
		context.set_window_image(window_id, "image", &image).unwrap();
		eprintln!("Making window visible.");
		context.set_window_visible(window_id, true).unwrap();
		eprintln!("Done, waiting to be killed.");
	}).unwrap();
}
