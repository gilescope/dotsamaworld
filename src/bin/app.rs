use bevy::prelude::bevy_main;

#[bevy_main]
fn main() {
	#[cfg(target_arch = "wasm32")]
	console_error_panic_hook::set_once();

	//bridge.update("Hi");
	dotsamaworld::main();
}
