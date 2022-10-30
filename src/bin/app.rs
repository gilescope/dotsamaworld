fn main() {
	#[cfg(target_arch = "wasm32")]
	console_error_panic_hook::set_once();

	dotsamaworld::main();
}
