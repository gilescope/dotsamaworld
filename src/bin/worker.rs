#[cfg(target_arch = "wasm32")]
use dotsamaworld::IOWorker;
#[cfg(target_arch = "wasm32")]
use gloo_worker::Registrable;

fn main() {
	#[cfg(target_arch = "wasm32")]
	console_error_panic_hook::set_once();

	#[cfg(target_arch = "wasm32")]
	IOWorker::registrar().register();
}
