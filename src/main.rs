mod backend;
pub use backend::context::Context;
pub use backend::context::ContextHandle;
pub use backend::proxy::ContextProxy;
pub use backend::window::Window;
pub use backend::window::WindowOptions;

pub mod oneshot;
pub mod error;

pub use wgpu::Color;
pub use winit::window::WindowId;

fn main() {
	let args : Vec<_> = std::env::args().collect();
	let image = image::open(args.get(1).unwrap()).unwrap();

	let context = Context::new(wgpu::TextureFormat::Bgra8UnormSrgb).unwrap();
	let proxy = context.proxy();

	std::thread::spawn(move || fake_main(image, proxy));
	context.run();
}

fn fake_main(image: image::DynamicImage, proxy: ContextProxy<()>) {
	proxy.execute_function(move |mut context: ContextHandle<()>| {
		eprintln!("Making new window.");
		let window_id = context.create_window("Show Image", WindowOptions {
			preserve_aspect_ratio: true,
			background_color: Color::BLACK,
			start_hidden: true,
		}).unwrap();
		eprintln!("Setting image.");
		context.set_window_image(window_id, "image", &image).unwrap();
		eprintln!("Making window visible.");
		context.set_window_visible(window_id, true).unwrap();
		eprintln!("Done, waiting to be killed.");
	}).unwrap();
}
