use winit::dpi::PhysicalSize;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
pub(crate) type OnResizeSender = Sender<()>;
pub(crate) type OnResizeReceiver = Receiver<()>;

//todo: needs to be in a mutex really?
pub(crate) fn setup_viewport_resize_system(resize_sender: Mutex<OnResizeSender>) {
	#[cfg(target_family = "wasm")]
	{
		let web_window = web_sys::window().expect("could not get window");
		let local_sender = resize_sender.lock().unwrap().clone();

		local_sender.send(()).unwrap();

		gloo_events::EventListener::new(&web_window, "resize", move |_event| {
			local_sender.send(()).unwrap();
		})
		.forget();
	}
}

#[cfg(target_family = "wasm")]
pub(crate) fn viewport_resize_system(
	// mut window: &mut Window,
	resize_receiver: &Mutex<OnResizeReceiver>,
) -> Option<winit::dpi::PhysicalSize<u32>> {
	if resize_receiver.lock().unwrap().try_recv().is_ok() {
		let new_size = get_viewport_size();
		//TODO: bugout if window size is already this.
		if new_size.width > 0 && new_size.height > 0 {
			return Some(new_size)
		}
	}
	None
}


//from bevy_web_fullscreen https://github.com/ostwilkens/bevy_web_fullscreen/blob/master/LICENSE

#[cfg(target_family = "wasm")]
fn get_viewport_size() -> PhysicalSize<u32> {
	let web_window = web_sys::window().expect("could not get window");
	let document_element = web_window
		.document()
		.expect("could not get document")
		.document_element()
		.expect("could not get document element");

	let width = document_element.client_width();
	let height = document_element.client_height();

	PhysicalSize::new(width as u32, height as u32)
}
