mod context;
mod proxy;
mod window;
mod util;

pub mod oneshot;
pub mod error;

pub use context::Context;
pub use context::ContextHandle;
pub use proxy::ContextProxy;
pub use window::Window;
pub use window::WindowOptions;

pub use wgpu::Color;
pub use winit::window::WindowId;


fn main() {
	let args : Vec<_> = std::env::args().collect();
	let image = image::open(args.get(1).unwrap()).unwrap();

	let context = Context::new(wgpu::TextureFormat::Bgra8UnormSrgb).unwrap();
	let proxy = context.proxy();

	std::thread::spawn(move || fake_main(image, proxy));
	context.run(|_context, _command: ()| ());
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
