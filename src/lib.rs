#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(slice_pattern)]
#![feature(slice_group_by)]
#![feature(option_get_or_insert_default)]
#![feature(async_closure)]
#![feature(stmt_expr_attributes)]
#![feature(let_chains)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]
#![allow(clippy::identity_op)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
use crate::{
	camera::CameraUniform,
	movement::Destination,
	ui::{ui_bars_system, Details, DotUrl, UrlBar},
};
use chrono::prelude::*;
use core::num::NonZeroI64;
use datasource::DataUpdate;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use jpeg_decoder::Decoder;
use lazy_static::lazy_static;
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use std::{
	collections::{HashMap, HashSet},
	convert::AsRef,
	f32::consts::PI,
	iter,
	sync::{
		atomic::{AtomicU32, Ordering},
		Arc, Mutex,
	},
	time::Duration,
};
use wgpu::{util::DeviceExt, TextureFormat};
use winit::{
	dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
	event::{WindowEvent, *},
	event_loop::EventLoop,
	window::{Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
use {
	core::future::Future, gloo_worker::Spawnable, gloo_worker::WorkerBridge, wasm_bindgen::JsCast,
	webworker::WorkerResponse, winit::platform::web::WindowBuilderExtWebSys,
};

//Block space calculations:
//TODO: free_weight = total weight of block - used weight
// average_extrinsic_weight = (total used weight / extrinsic count)
// capacity extrinsics = free weight / average_extrinsic_weight

// Define macros before mods
macro_rules! log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

// Until fn is defined in std lib prelude.
#[inline]
fn default<T>() -> T
where
	T: Default,
{
	Default::default()
}

mod camera;
mod content;
mod datasource;
mod input;
mod movement;
mod style;
mod texture;
mod ui;
mod resize;

#[cfg(target_family = "wasm")]
pub mod webworker;
#[cfg(target_family = "wasm")]
pub use webworker::IOWorker;

// #[cfg(feature = "spacemouse")]
// use bevy_spacemouse::{SpaceMousePlugin, SpaceMouseRelativeControllable};

mod networks;
pub mod recorder;
use networks::Env;

/// Pick a faster allocator.
#[cfg(all(not(target_env = "msvc"), not(target_arch = "wasm32")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_env = "msvc"), not(target_arch = "wasm32")))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "spacemouse")]
pub struct MovementSettings {
	pub sensitivity: f32,
	pub speed: f32,
}

#[cfg(feature = "spacemouse")]
impl Default for MovementSettings {
	fn default() -> Self {
		Self { sensitivity: 0.00012, speed: 12. }
	}
}

/// Distance vertically between layer 0 and layer 1
const LAYER_GAP: f32 = 0.;
const CHAIN_HEIGHT: f32 = 0.001;
const CUBE_WIDTH: f32 = 0.8;

static FREE_TXS: AtomicU64 = AtomicU64::new(0);

// The time by which all times should be placed relative to each other on the x axis.
lazy_static! {
	static ref BASETIME: Arc<Mutex<i64>> = default();
}

lazy_static! {
	static ref LINKS: Arc<Mutex<Vec<MessageSource>>> = default();
}

lazy_static! {
	static ref CHAIN_STATS: Arc<Mutex<HashMap<isize, ChainStats>>> = default();
}

lazy_static! {
	static ref UPDATE_QUEUE: Arc<std::sync::Mutex<RenderUpdate>> = default();
}

lazy_static! {
	static ref SELECTED: Arc<std::sync::Mutex<Vec<(u32, Details, ChainInfo)>>> = default();
}

lazy_static! {
	static ref DETAILS: Arc<std::sync::Mutex<RenderDetails>> = default();
}

lazy_static! {
	static ref SOVEREIGNS: Arc<std::sync::Mutex<Option<Sovereigns>>> = default();
}

//TODO could these be thread local?
lazy_static! {
	static ref REQUESTS: Arc<std::sync::Mutex<Vec<BridgeMessage>>> = default();
}

lazy_static! {
	static ref PALLETS: Arc<std::sync::Mutex<HashSet<String>>> = default();
}

/// Bump this to tell the current datasources to stop.
static DATASOURCE_EPOC: AtomicU32 = AtomicU32::new(0);

/// if you need bestest fps...
static PAUSE_DATA_FETCH: AtomicU32 = AtomicU32::new(0);

/// Immutable once set up.
#[derive(Clone, Serialize, Deserialize)]
pub struct ChainInfo {
	pub chain_ws: Vec<String>,
	// Negative is other direction from center.
	pub chain_index: isize,
	pub chain_url: DotUrl,
	pub chain_name: String,
}

use chrono::DateTime;
use core::sync::atomic::AtomicU64;

#[derive(Default)]
pub struct ChainStats {
	/// weight of blocks containing non-boring extrinsics (with base block already subtracted)
	total_block_weight: u64,
	/// number of non-boring extrinsics in blocks
	total_extrinsics: u32,
}

impl ChainStats {
	fn avg_free_transactions(&self) -> Option<u64> {
							 
		//currently max ever seen + 100m
		let max_block_size = 500_227_690_912u64; //todo: get from system state call
										 // let min_block_weight = 5_000_000_000u64;

		if self.total_extrinsics == 0 {
			return None
		}

		//MIN_BLOCK_WEIGHT has been subtracted from the block weights.
		let weight_per_extrinsic =
			(self.total_block_weight).checked_div(self.total_extrinsics as u64)?;
		// log!("weight per extrinsic: {}", weight_per_extrinsic);
		let free_weight = max_block_size
			.checked_sub(self.total_block_weight)
			.unwrap_or_else(|| panic!("dude heavy {}", self.total_block_weight));

		//round down
		free_weight.checked_div(weight_per_extrinsic)
	}
}

pub struct DataSourceChangedEvent {
	source: String,
	timestamp: Option<i64>,
}

#[derive(Default)]
pub struct Anchor {
	pub follow_chain: bool,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
	// Use `js_namespace` here to bind `console.log(..)` instead of just
	// `log(..)`
	#[wasm_bindgen::prelude::wasm_bindgen(js_namespace = console)]
	fn log(s: &str);

	// The `console.log` is quite polymorphic, so we can bind it with multiple
	// signatures. Note that we need to use `js_name` to ensure we always call
	// `log` in JS.
	#[wasm_bindgen(js_namespace = console, js_name = log)]
	fn log_u32(a: u32);

	// Multiple arguments too!
	#[wasm_bindgen(js_namespace = console, js_name = log)]
	fn log_many(a: &str, b: &str);
}

#[cfg(not(target_arch = "wasm32"))]
fn log(s: &str) {
	println!("{}", s);
}

pub fn main() {
	#[cfg(target_arch = "wasm32")]
	async_std::task::block_on(async_main());
	#[cfg(not(target_arch = "wasm32"))]
	async_std::task::block_on(async_main()).unwrap();
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
	position: [f32; 3],
	color: [f32; 3],
	tex: [f32; 2],
}

impl Vertex {
	fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x2,
				},
			],
		}
	}
}

// We don't use blue. r, g, color is texture co-ordinates
//
// counter clockwise to be visible
// B-D
// | |
// A-C

fn rect_instances(count: usize) -> Vec<Vertex> {
	let count = count as f32;
	let mut results = vec![];
	let scale = 3.25;

	results.push(Vertex {
		position: [0., 0., 0.0],
		tex: [0., 1. / count],
		color: [0., 1. / count, -2.],
	}); // A
	results.push(Vertex { position: [scale, 0., 0.0], tex: [0., 0.], color: [0., 0., -2.] }); // B
	results.push(Vertex {
		position: [0., 0., 3. * scale],
		tex: [1., 1. / count],
		color: [1., 1. / count, -2.],
	}); // C
	results.push(Vertex { position: [scale, 0., 3. * scale], tex: [1., 0.], color: [1., 0., -2.] }); // D

	//TODO: should one set of these texture positions be reversed?
	results.push(Vertex {
		position: [0., 0.3, 0.0],
		tex: [0., 1. / count],
		color: [0., 1. / count, -2.],
	}); // A
	results.push(Vertex { position: [scale, 0.3, 0.0], tex: [0., 0.], color: [0., 0., -2.] }); // B
	results.push(Vertex {
		position: [0., 0.3, 3. * scale],
		tex: [1., 1. / count],
		color: [1., 1. / count, -2.],
	}); // C
	results.push(Vertex {
		position: [scale, 0.3, 3. * scale],
		tex: [1., 0.],
		color: [1., 0., -2.],
	}); // D

	// 0,0                    0,1
	//   texture co-ordinates
	// 1,0                    1,1
	results
}

/// Counter clockwise to show up as looking from outside at cube.
// const INDICES: &[u16] = &cube_indicies(0);

const fn rect_indicies(offset: u16) -> [u16; 12] {
	let a = offset + 0;
	let b = offset + 1;
	let c = offset + 2;
	let d = offset + 3;
	[
		a,
		b,
		d,
		d,
		c,
		a,
		// Second side (backwards)
		d + 4,
		b + 4,
		a + 4,
		a + 4,
		c + 4,
		d + 4,
	]
}

/// https://www.researchgate.net/profile/John-Sheridan-7/publication/253573419/figure/fig1/AS:298229276135426@1448114808488/A-volume-is-subdivided-into-cubes-The-vertices-are-numbered-0-7.png

//TODO: rename to cube!!
fn cube(z_width: f32, y_height: f32, x_depth: f32, r: f32, g: f32, b: f32) -> [Vertex; 20] {
	let col = |bump: f32| -> [f32; 3] {
		[(r + bump).clamp(0., 2.), (g + bump).clamp(0., 2.), (b + bump).clamp(0., 2.)]
	};
	let bump = 0.10;
	[
		Vertex { tex: [1., 0.], position: [0.0, y_height, 0.0], color: col(bump) }, // C
		Vertex { tex: [1., 0.], position: [0.0, y_height, z_width], color: col(bump) }, // D
		Vertex { tex: [1., 1.], position: [0., 0., z_width], color: col(-bump) },   // B
		Vertex { tex: [1., 1.], position: [0., 0.0, 0.0], color: col(-bump) },      // A
		Vertex { tex: [0., 0.], position: [x_depth, y_height, 0.0], color: col(bump) }, // C
		Vertex { tex: [0., 0.], position: [x_depth, y_height, z_width], color: col(bump) }, // D
		Vertex { tex: [0., 1.], position: [x_depth, 0., z_width], color: col(bump * 2.0) }, // B
		Vertex { tex: [0., 1.], position: [x_depth, 0.0, 0.0], color: col(bump * 2.0) }, // A
		// Same as above but with different tex co-ordinates
		//BACKWARDS left-right
		Vertex { tex: [0., 0.], position: [0.0, y_height, 0.0], color: col(bump) }, // C
		Vertex { tex: [1., 0.], position: [0.0, y_height, z_width], color: col(bump) }, // D
		Vertex { tex: [1., 1.], position: [0., 0., z_width], color: col(-bump) },   // B
		Vertex { tex: [0., 1.], position: [0., 0.0, 0.0], color: col(-bump) },      // A
		Vertex { tex: [1., 0.], position: [x_depth, y_height, 0.0], color: col(bump) }, // C
		Vertex { tex: [0., 0.], position: [x_depth, y_height, z_width], color: col(bump) }, // D
		Vertex { tex: [0., 1.], position: [x_depth, 0., z_width], color: col(bump * 2.0) }, // B
		Vertex { tex: [1., 1.], position: [x_depth, 0.0, 0.0], color: col(bump * 2.0) }, // A
		// RIGHT needs textures backwards
		//1 => 16
		Vertex { tex: [0., 0.], position: [0.0, y_height, z_width], color: col(bump) }, // D
		//2 => 17
		Vertex { tex: [0., 1.], position: [0., 0., z_width], color: col(-bump) }, // B
		//5 => 18
		Vertex { tex: [1., 0.], position: [x_depth, y_height, z_width], color: col(bump) }, // D
		//6 => 19
		Vertex { tex: [1., 1.], position: [x_depth, 0., z_width], color: col(bump * 2.0) }, // B
	]
	//TODO: can duplicate vertex with different texture co-ordinates.
}

/*
		1,1,0    1,1,1       6 7

		1,0,0  1,0,1//MIN    4  5

0,1,0    0,1,1               2  3

0,0,0  0,0,1//MIN            0  1

*/

const fn cube_indicies(offset: u16) -> [u16; 36] {
	[
		//TOP
		// 6,5,4,
		// 4,7,6,
		8 + offset + 6, //back
		8 + offset + 7,
		8 + offset + 4, //TODO only need external faces
		8 + offset + 4,
		8 + offset + 5,
		8 + offset + 6, // // //
		// 0,1,2,
		// 2,3,0,
		8 + offset + 0, //front
		8 + offset + 3,
		8 + offset + 2,
		8 + offset + 2,
		8 + offset + 1,
		8 + offset + 0, //
		// 5,6,2,
		// 2,1,5,
		offset + 18, //5,//right ! (BACKWARDS)
		offset + 16, //1,
		offset + 17, //2,
		offset + 17, //2,
		offset + 19, //6,
		offset + 18, //5, // //
		// 7,4,0,
		// 0,3,7,
		offset + 7, //left
		offset + 3,
		offset + 0,
		offset + 0,
		offset + 4,
		offset + 7, // //
		// 7,3,2,
		// 2,6,7,
		offset + 7, //defnitely this is bottom!
		offset + 6,
		offset + 2,
		offset + 2,
		offset + 3,
		offset + 7,
		//
		offset + 4, //top
		offset + 0,
		offset + 1,
		offset + 1,
		offset + 5,
		offset + 4,
		// 4,5,1,
		// 1,0,4,
	]
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Deserialize, Serialize, Debug)]
struct Instance {
	position: [f32; 3],
	color: u32, //r g b a - uses alpha to point to 255 emojiis.
}

impl Instance {
	fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
			// We need to switch from using a step mode of Vertex to Instance
			// This means that our shaders will only change to use the next
			// instance when the shader starts processing a new instance
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					// While our vertex shader only uses locations 0, and 1 now, in later tutorials
					// we'll be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict
					// with them later
					shader_location: 3,
					format: wgpu::VertexFormat::Float32x3,
				},
				// A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a
				// slot for each vec4. We'll have to reassemble the mat4 in
				// the shader.
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
					shader_location: 4,
					format: wgpu::VertexFormat::Uint32,
				},
			],
		}
	}
}

// #[repr(C)]
// #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Deserialize, Serialize, Debug)]
// struct RainbowInstance {
// 	position: [f32; 3],
// 	destination: [f32; 3],
// 	color: u32, //r g b a - could use alpha to point to emoji 0-4 mod 2... gets you 255
// 	link_type: u32 // we get this for free due to alignment...
// }

// impl RainbowInstance {
// 	fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
// 		use std::mem;
// 		wgpu::VertexBufferLayout {
// 			array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
// 			// We need to switch from using a step mode of Vertex to Instance
// 			// This means that our shaders will only change to use the next
// 			// instance when the shader starts processing a new instance
// 			step_mode: wgpu::VertexStepMode::Instance,
// 			attributes: &[
// 				wgpu::VertexAttribute {
// 					offset: 0,
// 					// While our vertex shader only uses locations 0, and 1 now, in later tutorials
// 					// we'll be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict
// 					// with them later
// 					shader_location: 5,
// 					format: wgpu::VertexFormat::Float32x3,
// 				},
// 				wgpu::VertexAttribute {
// 					offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
// 					// While our vertex shader only uses locations 0, and 1 now, in later tutorials
// 					// we'll be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict
// 					// with them later
// 					shader_location: 6,
// 					format: wgpu::VertexFormat::Float32x3,
// 				},
// 				// A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a
// 				// slot for each vec4. We'll have to reassemble the mat4 in
// 				// the shader.
// 				wgpu::VertexAttribute {
// 					offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
// 					shader_location: 7,
// 					format: wgpu::VertexFormat::Uint32,
// 				},
// 				wgpu::VertexAttribute {
// 					offset: mem::size_of::<[f32; 7]>() as wgpu::BufferAddress, // This is actually 6 f32 + 1 u32 size
// 					shader_location: 8,
// 					format: wgpu::VertexFormat::Uint32,
// 				},
// 			],
// 		}
// 	}
// }

async fn async_main() -> std::result::Result<(), ()> {
	#[cfg(target_family = "wasm")]
	let url = web_sys::window().unwrap().location().search().expect("no search exists");
	#[cfg(not(target_family = "wasm"))]
	let url = "";

	let url = url::Url::parse(&format!("http://dotsama.world/{}", url)).unwrap();

	let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

	log!("url : {:?}", params.get("env"));

	#[cfg(target_arch = "wasm32")]
	console_error_panic_hook::set_once();
	// let error = console_log::init_with_level(Level::Warn);
	//.expect("Failed to enable logging");

	// App assumes the target dir exists for caching data
	#[cfg(not(feature = "wasm32"))]
	let _ = std::fs::create_dir_all("target");

	let _low_power_mode = false;

	#[cfg(target_feature = "atomics")]
	log!("Yay atomics!");

	// app.insert_resource(Msaa { samples: 4 });

	//  .insert_resource(WinitSettings::desktop_app()) - this messes up the 3d space mouse?

	// app.add_plugin(SpaceMousePlugin);

	// if low_power_mode {
	// 	app.insert_resource(WinitSettings::desktop_app());
	// }
	// .add_plugin(recorder::RecorderPlugin)
	// app.add_plugin(PolylinePlugin);

	// #[cfg(feature = "spacemouse")]
	// app.add_startup_system(move |mut scale: ResMut<bevy_spacemouse::Scale>| {
	// 	scale.rotate_scale = 0.00010;
	// 	scale.translate_scale = 0.004;
	// });

	// // .add_system(pad_system)
	// app.add_system_to_stage(CoreStage::PostUpdate, update_visibility);

	// html_body::get().request_pointer_lock();

	let event_loop = winit::event_loop::EventLoopBuilder::<()>::with_user_event().build();

	let mut winit_window_builder = winit::window::WindowBuilder::new();

	#[cfg(target_family = "wasm")]
	{
		let window = web_sys::window().unwrap();
		let document = window.document().unwrap();
		let canvas = document.query_selector("canvas").expect("Cannot query for canvas element.");
		if let Some(canvas) = canvas {
			let canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok();
			winit_window_builder = winit_window_builder.with_canvas(canvas);
		} else {
			panic!("Cannot find element: {}.", "canvas");
		}
	}

	log!("about to run event loop");
	let window = winit_window_builder.build(&event_loop).unwrap();
	#[cfg(target_family = "wasm")]
	wasm_bindgen_futures::spawn_local(run(event_loop, window, params));
	#[cfg(not(target_family = "wasm"))]
	run(event_loop, window, params).await;

	log!("event loop finished");
	Ok::<(), ()>(())
}

async fn run(event_loop: EventLoop<()>, window: Window, params: HashMap<String, String>) {
	// let movement_settings = MovementSettings {
	// 	sensitivity: 0.00020, // default: 0.00012
	// 	speed: 12.0,          // default: 12.0
	// 	boost: 5.,
	// };
	let ground_width = 1000000.0f32;
	let touch_sensitivity = 2.0f64;
	let sample_count = 1;

	let mut q = params.get("q").unwrap_or(&"dotsama:live".to_string()).clone();
	if !q.contains(':') {
		q.push_str(":live");
	}
	log!("q: {}", q);

	//"dotsama:/1//10504599".to_string()
	let mut urlbar = ui::UrlBar::new(q.clone(), Utc::now().naive_utc(), Env::Local);
	// app.insert_resource();
	let sovereigns = Sovereigns { relays: vec![], default_track_speed: 1. };

	// let mouse_capture = movement::MouseCapture::default();
	let mut anchor = Anchor::default();
	let mut destination = movement::Destination::default();
	let mut inspector = Inspector::default();
	let mut occupied_screen_space = ui::OccupiedScreenSpace::default();

	let instance = wgpu::Instance::new(wgpu::Backends::all());
	// SAFETY: `window` Handle must be a valid object to create a surface upon
	// and must remain valid for the lifetime of the returned surface.
	let mut surface = unsafe { instance.create_surface(&window) };

	let adapter = instance
		.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: default(),
			compatible_surface: Some(&surface),
			force_fallback_adapter: false,
		})
		.await
		.unwrap();

	//TODO: can we await instead of block_on here?
	let features = if sample_count > 1 {
		wgpu::Features::default() | wgpu::Features::CLEAR_TEXTURE
	} else {
		wgpu::Features::default()
	};

	let (device, queue) = pollster::block_on(adapter.request_device(
		&wgpu::DeviceDescriptor {
			features,
			limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
			label: None,
		},
		None,
	))
	.unwrap();

	let mut size: PhysicalSize<u32> = window.inner_size();
	let mut hidpi_factor = window.scale_factor(); // 2.0 <-- this is why quaters!
	log!("hidpi factor {:?}", hidpi_factor);

	// size.width *= hidpi_factor as u32;//todo!
	// size.height *= hidpi_factor as u32;

	log!("Initial size: width:{} height:{}", size.width, size.height);
	// size.width = 1024; - seems double this so 4x pixels
	// size.height = 768;

	let channel = std::sync::mpsc::channel();
	let resize_sender: resize::OnResizeSender = channel.0;
	let resize_receiver = Mutex::new(channel.1);
	resize::setup_viewport_resize_system(Mutex::new(resize_sender));

	let surface_format = surface.get_supported_formats(&adapter)[0];
	let mut surface_config = wgpu::SurfaceConfiguration {
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
		format: surface_format,
		width: size.width,
		height: size.height,
		present_mode: wgpu::PresentMode::Fifo, //Immediate not supported on web
		alpha_mode: wgpu::CompositeAlphaMode::Auto,
	};
	surface.configure(&device, &surface_config);

	assert!(size.width > 0);
	// We use the egui_winit_platform crate as the platform.
	let mut platform = Platform::new(PlatformDescriptor {
		physical_width: size.width,
		physical_height: size.height,
		scale_factor: window.scale_factor(),
		font_definitions: default(),
		style: default(),
	});

	// We use the egui_wgpu_backend crate as the render backend.
	let mut egui_rpass = RenderPass::new(&device, surface_format, 1);

	// Display the application

	let mut frame_time = Utc::now().timestamp();
	let tx_time = Utc::now().timestamp();
	// let instance_buffer: wgpu::Buffer;

	// let instance_buffer = device.create_buffer_init(
	//     &wgpu::util::BufferInitDescriptor {
	//         label: Some("Instance Buffer"),
	//         contents: bytemuck::cast_slice(&instance_data),
	//         usage: wgpu::BufferUsages::VERTEX,
	//     }
	// )
	// let    render_pipeline: wgpu::RenderPipeline = ;
	let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: Some("Shader"),
		source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
	});

	// let mut camera = Camera {
	//     // position the camera one unit up and 2 units back
	//     // +z is out of the screen
	//     eye: (0.0, 1.0, 2.0).into(),
	//     // have it look at the origin
	//     target: (0.0, 0.0, 0.0).into(),
	//     // which way is "up"
	//     up: cgmath::Vector3::unit_y(),
	//     aspect: size.width as f32 / size.height as f32,
	//     fovy: 45.0,
	//     znear: 0.1,
	//     zfar: 100.0,
	// };
	let mut camera =
		camera::Camera::new((-200.0, 100.0, 0.0), cgmath::Deg(0.0), cgmath::Deg(-20.0));
	let mut projection =
		camera::Projection::new(size.width, size.height, cgmath::Deg(45.0), 0.1, 400000.0);
	let mut camera_controller = input::CameraController::new(4.0, 0.4);

	let mut camera_uniform = CameraUniform::new();
	camera_uniform.update_view_proj(&camera, &projection);

	let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Camera Buffer"),
		contents: bytemuck::cast_slice(&[camera_uniform]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});

	let camera_bind_group_layout =
		device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: sample_count > 1,
						view_dimension: wgpu::TextureViewDimension::D2,
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 2,
					visibility: wgpu::ShaderStages::FRAGMENT,
					// This should match the filterable field of the
					// corresponding Texture entry above.
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 3,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: sample_count > 1,
						view_dimension: wgpu::TextureViewDimension::D2,
						sample_type: wgpu::TextureSampleType::Float { filterable: true },
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
					binding: 4,
					visibility: wgpu::ShaderStages::FRAGMENT,
					// This should match the filterable field of the
					// corresponding Texture entry above.
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None,
				},
			],
			label: Some("camera_bind_group_layout"),
		});

	//if !loaded_textures {
	//	loaded_textures = true;

	let (diffuse_texture_view, diffuse_sampler, texture_map) =
		load_textures(&device, &queue, sample_count).await;
	let (diffuse_texture_view_emoji, diffuse_sampler_emoji) =
		load_textures_emoji(&device, &queue, sample_count).await;
	// diffuse_texture_view =diffuse_texture_view1;
	// diffuse_sampler = diffuse_sampler1;
	//}

	let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &camera_bind_group_layout,
		entries: &[
			wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() },
			wgpu::BindGroupEntry {
				binding: 1,
				resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
			},
			wgpu::BindGroupEntry {
				binding: 2,
				resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
			},
			wgpu::BindGroupEntry {
				binding: 3,
				resource: wgpu::BindingResource::TextureView(&diffuse_texture_view_emoji),
			},
			wgpu::BindGroupEntry {
				binding: 4,
				resource: wgpu::BindingResource::Sampler(&diffuse_sampler_emoji),
			},
		],
		label: Some("camera_bind_group"),
	});

	let mut depth_texture = texture::Texture::create_depth_texture(
		&device,
		&surface_config,
		"depth_texture",
		sample_count,
	);

	let mut vertices = vec![]; //cube
	let start_cube = vertices.len(); //block
	vertices.extend(cube(CUBE_WIDTH, CUBE_WIDTH, CUBE_WIDTH, 0., 0., 0.));
	let start_block = vertices.len(); //block
	vertices.extend(cube(10., 0.3, 10., 0., 0.0, 0.));
	let start_chain = vertices.len(); //chain
	vertices.extend(cube(10., CHAIN_HEIGHT, 100000., 0.0, 0.0, 0.));
	let start_ground = vertices.len(); //ground
	vertices.extend(cube(ground_width, 10., ground_width, 0.0, 0.0, 0.));
	let start_selected = vertices.len(); //selected
	vertices.extend(cube(CUBE_WIDTH + 0.2, CUBE_WIDTH + 0.2, CUBE_WIDTH + 0.2, 0., 0., 0.));
	let start_textured = vertices.len(); // textured rectangle
	vertices.extend(&rect_instances(texture_map.len()));

	// vertices.extend(cube(ground_width, 0.00001, ground_width, 0.0, 0.0, 0.));

	let mut indicies: Vec<u16> = vec![];
	indicies.extend(cube_indicies(start_cube as u16));
	let indicies_cube = 0..indicies.len() as u32;
	let end = indicies.len() as u32;
	indicies.extend(cube_indicies(start_block as u16));
	let indicies_block = end..indicies.len() as u32;
	let end = indicies.len() as u32;
	indicies.extend(cube_indicies(start_chain as u16));
	let indicies_chain = end..indicies.len() as u32;
	let end = indicies.len() as u32;
	indicies.extend(cube_indicies(start_ground as u16));
	let indicies_ground = end..indicies.len() as u32;
	let end = indicies.len() as u32;
	indicies.extend(cube_indicies(start_selected as u16));
	let indicies_selected = end..indicies.len() as u32;
	let end = indicies.len() as u32;
	indicies.extend(rect_indicies(start_textured as u16));
	let indicies_textured = end..indicies.len() as u32;
	// let end = indicies.len() as u32;

	let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Vertex Buffer"),
		contents: bytemuck::cast_slice(&vertices[..]),
		usage: wgpu::BufferUsages::VERTEX,
	});

	let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Index Buffer"),
		contents: bytemuck::cast_slice(&indicies[..]),
		usage: wgpu::BufferUsages::INDEX,
	});

	// let r = 230./255.;
	// let b = 122./255.;

	// let c = as_rgba_u32(-10., -10., -10., 1.0);
	let c = as_rgba_u32(0.0, 0.0, 0.0, 1.0);
	let ground_instance_data: Vec<Instance> = vec![
		// Instance{ position: [-ground_width/2.0,-100.,-ground_width/2.0], color:
		// as_rgba_u32(-1.0, -1.0, -1.0, 1.0) },
		 Instance { position: [-ground_width / 2.0, -100., -ground_width / 2.0], color: c },
		// Instance { position: [-ground_width / 2.0, 500., -ground_width / 2.0], color: c },
		// Instance{ position: [-ground_width/2.0,1000.,-ground_width/2.0], color: 344411 }
	];

	let mut chain_instance_data = vec![];
	let mut block_instance_data = vec![];
	let mut extrinsic_instance_data : Vec<Instance> = vec![];
	let mut event_instance_data : Vec<Instance> = vec![];
	let mut selected_instance_data = vec![];
	let mut textured_instance_data: Vec<Instance> = vec![];

	let mut extrinsic_target_heights: Vec<f32> = vec![];
	let mut event_target_heights: Vec<f32> = vec![];
	//let instance_data = instances;//.iter().map(Instance::to_raw).collect::<Vec<_>>();

	let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		label: Some("Render Pipeline Layout"),
		bind_group_layouts: &[&camera_bind_group_layout],
		push_constant_ranges: &[],
	});

	let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Render Pipeline"),
		layout: Some(&render_pipeline_layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: "vs_main",
			buffers: &[Vertex::desc(), Instance::desc()],
		},
		fragment: Some(wgpu::FragmentState {
			// 3.
			module: &shader,
			entry_point: "fs_main",
			targets: &[Some(wgpu::ColorTargetState {
				// 4.
				format: TextureFormat::Rgba8UnormSrgb,
				blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
				write_mask: wgpu::ColorWrites::ALL,
			})],
		}),

		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleList,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Ccw,
			cull_mode: Some(wgpu::Face::Back),
			// Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
			polygon_mode: wgpu::PolygonMode::Fill,
			// Requires Features::DEPTH_CLIP_CONTROL
			unclipped_depth: false,
			// Requires Features::CONSERVATIVE_RASTERIZATION
			conservative: false,
		},

		depth_stencil: Some(wgpu::DepthStencilState {
			format: texture::Texture::DEPTH_FORMAT,
			depth_write_enabled: true,
			depth_compare: wgpu::CompareFunction::Less,
			stencil: default(),
			bias: default(),
		}),
		multisample: wgpu::MultisampleState {
			count: sample_count,
			// mask: !0,
			// alpha_to_coverage_enabled: false,
			..default()
		},
		multiview: None,
	});

	let mut last_render_time = Utc::now();

	let mut frames = 0u64;
	let mut fps = 0;
	let mut tx = 0u64;
	let mut tps = 0;

	let initial_event = DataSourceChangedEvent {
		//source: "dotsama:/1//10504599".to_string(),
		// source: "local:live".to_string(),
		source: q, //"test:live".to_string(),
		timestamp: None,
	};

	source_data(
		initial_event,
		sovereigns,
		// details: Query<Entity, With<ClearMeAlwaysVisible>>,
		// clean_me: Query<Entity, With<ClearMe>>,
		&mut urlbar, /* handles: Res<ResourceHandles>,
		              * #[cfg(not(target_arch="wasm32"))]
		              * writer: EventWriter<DataSourceStreamEvent>, */
	);

	// let mut ctx = egui::Context::default();
	let mut mouse_pressed = false;

	use crate::camera::OPENGL_TO_WGPU_MATRIX;

	// #[cfg(target_family = "wasm")]
	// let viewport_size = get_viewport_size();
	// #[cfg(not(target_family = "wasm"))]
	// let viewport_size = window.inner_size();

	let matrix = OPENGL_TO_WGPU_MATRIX;
	let x: glam::Vec4 = glam::Vec4::new(matrix.x.x, matrix.x.y, matrix.x.z, matrix.x.w);
	let y: glam::Vec4 = glam::Vec4::new(matrix.y.x, matrix.y.y, matrix.y.z, matrix.y.w);
	let z: glam::Vec4 = glam::Vec4::new(matrix.z.x, matrix.z.y, matrix.z.z, matrix.z.w);
	let w: glam::Vec4 = glam::Vec4::new(matrix.w.x, matrix.w.y, matrix.w.z, matrix.w.w);
	let opengl_to_wgpu_matrix_mat4 = glam::Mat4::from_cols(x, y, z, w);
	let mut last_touch_location: HashMap<
		u64,
		(PhysicalPosition<f64>, DateTime<Utc>, Option<(PhysicalPosition<f64>, DateTime<Utc>)>),
	> = default();

	let mut last_mouse_position = None;
	let window_id = window.id();

	let mut ground_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("ground Instance Buffer"),
		contents: bytemuck::cast_slice(&ground_instance_data),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let mut ground_instance_data_count = ground_instance_data.len();
	let mut textured_instance_buffer =
		device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("textured Instance Buffer"),
			contents: bytemuck::cast_slice(&textured_instance_data),
			usage: wgpu::BufferUsages::VERTEX,
		});
	let mut textured_instance_data_count = textured_instance_data.len();
	let mut chain_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("chain Instance Buffer"),
		contents: bytemuck::cast_slice(&chain_instance_data),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let mut chain_instance_data_count = chain_instance_data.len();
	let mut block_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("block Instance Buffer"),
		contents: bytemuck::cast_slice(&block_instance_data),
		usage: wgpu::BufferUsages::VERTEX,
	});
	let mut block_instance_data_count = block_instance_data.len();
	// let mut extrinsic_instance_buffer =
	// 	device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
	// 		label: Some("cube Instance Buffer"),
	// 		contents: bytemuck::cast_slice(&extrinsic_instance_data),
	// 		usage: wgpu::BufferUsages::VERTEX,
	// 	});
	// let mut event_instance_buffer =
	// 	device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
	// 		label: Some("cube Instance Buffer"),
	// 		contents: bytemuck::cast_slice(&event_instance_data),
	// 		usage: wgpu::BufferUsages::VERTEX,
	// 	});
	// let mut cube_instance_data_count = cube_instance_data.len();
	//  =
	// 	device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
	// 		label: Some("selected Instance Buffer"),
	// 		contents: bytemuck::cast_slice(&selected_instance_data),
	// 		usage: wgpu::BufferUsages::VERTEX,
	// 	});
	// let mut selected_instance_data_count = selected_instance_data.len();

	// let mut loaded_textures = false;
	// let diffuse_texture_view: wgpu::TextureView;
	// let diffuse_sampler : wgpu::Sampler;

	// Don't try and select something if your in the middle of moving
	// let mut last_movement_time = Utc::now();
	event_loop.run(move |event, _, _control_flow| {
		let selected_instance_buffer;
		let event_instance_buffer;
		let extrinsic_instance_buffer;

		let now = Utc::now();

		let scale_x = size.width as f32 / hidpi_factor as f32;
		let scale_y = size.height as f32 / hidpi_factor as f32;

		// Pass the winit events to the platform integration.
		platform.handle_event(&event);


		let selected_details = SELECTED.lock().unwrap().clone();
		// TODO: avoid doing this every frame...
		selected_instance_data.clear();
		for (index, details, _chain_info) in &selected_details {
			if details.doturl.event.is_some() {
				selected_instance_data.push(create_selected_instance(&event_instance_data[*index as usize].clone()));
			} else {
				selected_instance_data.push(create_selected_instance(&extrinsic_instance_data[*index as usize].clone()));
			}
		}

		// viewport_resize_system(&resize_receiver);
		#[cfg(target_family = "wasm")]
		if let Some(new_size) = resize::viewport_resize_system(&resize_receiver) {
			log!("set new size width: {} height: {}", new_size.width, new_size.height);
			// window.set_inner_size(new_size);
			window.set_inner_size(LogicalSize::new(new_size.width, new_size.height));
		// 	projection.resize(new_size.width, new_size.height);
		// 	size = new_size;
		// 	surface.configure(&device, &surface_config);
		// 	depth_texture =
		// 		texture::Texture::create_depth_texture(&device, &surface_config, "depth_texture");

			// TODO can we set canvas size?

			size = new_size;
			hidpi_factor = window.scale_factor();
			resize(&size, &device, &mut surface_config, &mut projection, &mut surface, &mut depth_texture, hidpi_factor, &mut camera_uniform, &mut camera, sample_count, &mut platform, &window_id);
		}

		//if frames % 10 == 1
		let mut redraw = true;
		match event {
			 Event::DeviceEvent { event: DeviceEvent::MouseMotion{ delta },
			    .. // We're not using device_id currently
			} => if mouse_pressed {
				// log!("mouse click at screen: {:?}", delta);
			    camera_controller.process_mouse(delta.0, delta.1)
			}

			Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
				redraw = input::input(&mut camera_controller, event, &mut mouse_pressed);
				if let WindowEvent::CursorMoved{ position, .. } = event {
					last_mouse_position = Some(*position);
				}

				// WindowEvent::TouchpadMagnify and WindowEvent::TouchpadRotate events are
				// only available on macos, so build up from touch events:

				if let WindowEvent::Touch(Touch{ location, phase, id, .. }) = event {
					// let normal_loc = PhysicalPosition::<f64>{ x:location.x, y: size.height as f64 - location.y};
					// let ctx = platform.context();
					let touch_in_egui = false;
					// if let Some(layer) = ctx.layer_id_at(normal_loc) {
					// 	if layer.order == Order::Background {
					// 		!ctx.frame_state().unused_rect.contains(normal_loc)
					// 	} else {
					// 		true
					// 	}
					// } else {
					// 	false
					// };
					if !touch_in_egui {
						// Are two fingers being used? (pinch to zoom / rotate)
						let mut our_finger = None;
						let mut other_finger = None;
						for (other_id, (last_touch_location, last_time, previous)) in last_touch_location.iter() {
							if let Some((prev_loc, prev_time)) = previous {
								if (now - *last_time).num_milliseconds() < 200
								&& (now - *prev_time).num_milliseconds() < 500 {
									if other_id != id {
										other_finger = Some((last_touch_location, prev_loc));
									} else {
										our_finger = Some((location, prev_loc));
									}
								}
							}
						}

						// We have previous and current locations of two fingers.
						if let (Some((cur1, prev1)), Some((cur2, prev2))) = (our_finger, other_finger) {
							let dist = | loc1: &PhysicalPosition<f64>, loc2: &PhysicalPosition<f64> | {
								let x_diff = loc1.x - loc2.x;
								let y_diff = loc1.y - loc2.y;
								(x_diff*x_diff + y_diff * y_diff).sqrt()
							};
							let cur_dist = dist(cur1, cur2);
							let prev_dist = dist(prev1, prev2);
							//TODO: if dist less than X then it's a 2 fingers together
							// rotate if that.

							//TODO: could use pressure to boost?
							if cur_dist > prev_dist {
								// Zoom out
								camera_controller.process_scroll(
									&MouseScrollDelta::PixelDelta(
										PhysicalPosition { y: cur_dist - prev_dist, x:0. }));
							} else {
								// Zoom in
								camera_controller.process_scroll(
									&MouseScrollDelta::PixelDelta(
										PhysicalPosition { y: -(prev_dist - cur_dist), x:0. }));
							}
							*SELECTED.lock().unwrap() = vec![];
							selected_instance_data.clear();
						} else {

							//TODO: distingush from one finger touch move and a select.

							// one finger move touch.
							log!("Touch! {:?}", &location);
							// if *id == 0 {
							if let Some((last_touch_location, last_time, _prev)) = last_touch_location.get(id) {
								if let TouchPhase::Moved = phase {
									// LOL Gotcha: touch y 0 starts from the bottom!

									let x_diff = last_touch_location.x - location.x;
									let y_diff = last_touch_location.y - location.y;

									// If the distance is small then this is the continuation of a move
									// rather than a new touch.
									if x_diff.abs() + y_diff.abs() < 200. {
										// camera_controller.rotate_horizontal -= (x_diff / touch_sensitivity) as f32;
										// camera_controller.rotate_vertical += (y_diff / touch_sensitivity) as f32;

										//TODO:
										// time distance since updates * fps = frames to make movement in (to be smooth).
										let millies_elapsed = (now - *last_time).num_milliseconds();
										let elapsed_frames = fps as f32 * millies_elapsed as f32 / 1000.0;

										let per_frame_horiz = (x_diff / touch_sensitivity) as f32 / elapsed_frames;
										let per_frame_vert = (y_diff / touch_sensitivity) as f32 / elapsed_frames;

										let add = | stack: &mut Vec<f32>, bump, len | {
											for i in stack.iter_mut().rev().take(len) {
												*i += bump;
											}
											for _i in stack.len()..len {
												//TODO: these need to be inserted in the front not the end!
												stack.push(bump);
											}
										};
										let before = camera_controller.rotate_horizontal_stack.len();
										add(&mut camera_controller.rotate_horizontal_stack, per_frame_horiz, elapsed_frames as usize);
										add(&mut camera_controller.rotate_vertical_stack, per_frame_vert, elapsed_frames as usize);
										log!("stack before {} len {}, amount: {} duration: {} ",before, camera_controller.rotate_horizontal_stack.len(), 
										per_frame_horiz, millies_elapsed );
									}
								}
							}

							try_select(&camera, &projection, opengl_to_wgpu_matrix_mat4,
								&extrinsic_instance_data, &event_instance_data,
								&mut selected_instance_data, scale_x, scale_y, &size, location);
						}

						let val = last_touch_location.entry(*id).or_insert((*location, now, None));
						*val = (*location, now, Some((val.0, val.1)));
					}
				}

				if let WindowEvent::MouseInput { button: winit::event::MouseButton::Left, state, .. } = event {
					if let Some(position) = last_mouse_position {
						if let ElementState::Pressed = state {
							// If we're over an egui area we should not be trying to select anything.
							if !platform.context().is_pointer_over_area() {
								try_select(&camera, &projection, opengl_to_wgpu_matrix_mat4, &extrinsic_instance_data, &event_instance_data, &mut selected_instance_data,
								scale_x, scale_y, &size, &position);
							} else {
								log!("suppressing click as on egui");
							}
						}
					}
				}
				if let WindowEvent::Resized(new_size) = event {
					log!("WINIT: set new size width: {} height: {}", new_size.width, new_size.height);
					// window.set_inner_size(*new_size);       
					//window.set_inner_size(LogicalSize::new(new_size.width, new_size.height));  
					// size = new_size.clone();
					// surface_config.width = size.width;
					// surface_config.height = size.height;
					// projection.resize(size.width, size.height);
					// surface.configure(&device, &surface_config);
					// depth_texture =
					// texture::Texture::create_depth_texture(&device, &surface_config, "depth_texture");
					size = *new_size;
					hidpi_factor = window.scale_factor();
					resize(&size, &device, &mut surface_config, &mut projection, &mut surface, &mut depth_texture, hidpi_factor, &mut camera_uniform,&mut camera, sample_count, &mut platform, &window_id);
				} else if let WindowEvent::ScaleFactorChanged { new_inner_size, .. } = event {
					size = **new_inner_size;
					hidpi_factor = window.scale_factor();
					resize(&size, &device, &mut surface_config, &mut projection, &mut surface, &mut depth_texture, hidpi_factor, &mut camera_uniform, &mut camera, sample_count, &mut platform, &window_id);
				}
			},
			Event::RedrawRequested(window_id) if window_id == window.id() => {
				let now = Utc::now();
				let dt = now - last_render_time;
				last_render_time = now;

				camera_controller.update_camera(&mut camera, dt);
				camera_uniform.update_view_proj(&camera, &projection);
				queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
			},
			Event::MainEventsCleared => {
				// RedrawRequested will only trigger once, unless we manually
				// request it.
				window.request_redraw();
			},
			_ => {},
		}

		if redraw {
			frames += 1;

			// let mut data_update: Option<DataUpdate> = None;
			if let Ok(render_update) = &mut UPDATE_QUEUE.lock() {
				if render_update.any() {
					// if render_update.cube_instances.len() > 0 {
					// 	log!("Got update {:?}", render_update.cube_instances[0].0.position[0]);
					// }
					// log!("Got block {:?}", render_update.block_instances.len());
					// log!("Got chain {:?}", render_update.count());

					for (instance, height) in &render_update.extrinsic_instances {
						extrinsic_instance_data.push(*instance);
						extrinsic_target_heights.push(*height);
					}

					tx += render_update.extrinsic_instances.len() as u64;

					for (instance, height) in &render_update.event_instances {
						event_instance_data.push(*instance);
						event_target_heights.push(*height);
					}
					//TODO: drain not clone!
					block_instance_data.extend(render_update.block_instances.clone());
					chain_instance_data.extend(render_update.chain_instances.clone());

					for instance in &render_update.textured_instances {
						let key = if instance.color > 99_000 {
							(1, instance.color - 100_000)
						} else {
							(0, instance.color)
						};
						let texture = texture_map.get(&key);
						if let Some(texture_index) = texture {
							let (y,x) = texture_index;
							let texture_loc = (x + (y << 8)) as u32;
							textured_instance_data.push(Instance{color: texture_loc, ..*instance});
						}
					}

					if let Some(basetime) = render_update.basetime {
						// log!("Updated basetime");
						*BASETIME.lock().unwrap() = basetime.into();
					}

					render_update.chain_instances.truncate(0);
					render_update.block_instances.truncate(0);
					render_update.extrinsic_instances.truncate(0);
					render_update.event_instances.truncate(0);
					render_update.textured_instances.truncate(0);
				}
			}

			//todo rain in gpu
			if frames % 4 == 0 {
				rain(&mut extrinsic_instance_data, &mut extrinsic_target_heights);
				rain(&mut event_instance_data, &mut event_target_heights);
			}

			// TODO: when refreshing a buffer can we append to it???
			if ground_instance_data_count != ground_instance_data.len() {
				ground_instance_data_count = ground_instance_data.len();
				ground_instance_buffer =
					device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
						label: Some("ground Instance Buffer"),
						contents: bytemuck::cast_slice(&ground_instance_data),
						usage: wgpu::BufferUsages::VERTEX,
					}
				);
			}
			if chain_instance_data_count != chain_instance_data.len() {
				chain_instance_data_count = chain_instance_data.len();
				chain_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("chain Instance Buffer"),
					contents: bytemuck::cast_slice(&chain_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});
			}
			if block_instance_data_count != block_instance_data.len() {
				block_instance_data_count = block_instance_data.len();
				block_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("block Instance Buffer"),
					contents: bytemuck::cast_slice(&block_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});
			}
			//TODO: at the moment we have to do this every time due to rain.
			// if cube_instance_data_count != cube_instance_data.len() {
				// cube_instance_data_count = cube_instance_data.len();
				extrinsic_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("cube Instance Buffer"),
					contents: bytemuck::cast_slice(&extrinsic_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});

				event_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("cube Instance Buffer"),
					contents: bytemuck::cast_slice(&event_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});
			// }

			// render selected instance buffer eachtime as selected item might have changed
			selected_instance_buffer =
			device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("selected Instance Buffer"),
				contents: bytemuck::cast_slice(&selected_instance_data),
				usage: wgpu::BufferUsages::VERTEX,
			});

			if textured_instance_data_count != textured_instance_data.len() {
				textured_instance_data_count = textured_instance_data.len();
				textured_instance_buffer =
					device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
						label: Some("textured Instance Buffer"),
						contents: bytemuck::cast_slice(&textured_instance_data),
						usage: wgpu::BufferUsages::VERTEX,
					});
			}

			let output = surface.get_current_texture().unwrap();
			let view = output.texture.create_view(&default());

			let output_frame = output; //

			let output_view = view;

			// Begin to draw the UI frame.
			platform.begin_frame();

			ui_bars_system(
				&mut platform.context(),
				&mut occupied_screen_space,
				&camera.position,
				&mut urlbar,
				&mut anchor,
				&mut inspector,
				&mut destination,
				fps,
				tps,
				selected_details,
			);

			// End the UI frame. We could now handle the output and draw the UI with the backend.
			let full_output = platform.end_frame(Some(&window));
			let paint_jobs = platform.context().tessellate(full_output.shapes);

			let mut encoder = device
				.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("encoder") });

			// Upload all resources for the GPU.
			let screen_descriptor = ScreenDescriptor {
				physical_width: surface_config.width,
				physical_height: surface_config.height,
				scale_factor: hidpi_factor as f32,
			};
			let tdelta: egui::TexturesDelta = full_output.textures_delta;
			egui_rpass.add_textures(&device, &queue, &tdelta).expect("add texture ok");
			egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

			// Record all render passes.
			egui_rpass
				.execute(
					&mut encoder,
					&output_view,
					&paint_jobs,
					&screen_descriptor,
					Some(wgpu::Color::TRANSPARENT),
				)
				.unwrap();
			// Submit the commands.

			queue.submit(iter::once(encoder.finish()));

			let mut encoder = device
				.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("encoder") });
			// let mut encoder = device
			// 	.create_render_bundle_encoder(
			// 		&wgpu::RenderBundleEncoderDescriptor {
			// 			label: None,
			// 			color_formats: &[Some(config.format)],
			// 			depth_stencil: None,
			// 			sample_count,
			// 			multiview: None,
			// 		}
			// 	);
			{
				let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					label: Some("Render Pass"),
					color_attachments: &[
						// This is what @location(0) in the fragment shader targets
						Some(wgpu::RenderPassColorAttachment {
							view: &output_view,
							resolve_target: None,
							ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
						}),
					],
					depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
						view: &depth_texture.view,
						depth_ops: Some(wgpu::Operations {
							load: //wgpu::LoadOp::Load, 
							wgpu::LoadOp::Clear(1.0),
							store: true,
						}),
						stencil_ops: None,
					}),
				});

				render_pass.set_pipeline(&render_pipeline);
				render_pass.set_bind_group(0, &camera_bind_group, &[]);
				render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
				// Clip window:
				// let (x, y) = ((occupied_screen_space.left * scale_x) as u32,
				// (occupied_screen_space.top*scale_y) as u32); let (width, height) =
				// 	(size.width as u32 - x, size.height as u32 - (y +
				// (occupied_screen_space.bottom*scale_y) as u32));

				let side = if selected_instance_data.is_empty() { 0u32 } else {
					(occupied_screen_space.left * hidpi_factor as f32) as u32
				};

				// log!("size {:?}", size);
				let (x, y) = (0 + side, 50);
				let (width, height) =
					(size.width.saturating_sub(side), (size.height.saturating_sub(y)).saturating_sub(50) /* + 400 */);

				//if old_width != width as u32 {
				// log!(
				// 	"set scissor rect: x: {} y: {}, width: {} height: {}, was {}",
				// 	x,
				// 	y,
				// 	width,
				// 	height,
				// 	old_width
				// );
				// old_width = width as u32;
				//}

				render_pass.set_scissor_rect(x, y, width, height);

				// render_pass.set_viewport(x as f32,y as f32,width as f32, height as f32, 0., 1.);
				render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

				// Draw ground
				render_pass.set_vertex_buffer(1, ground_instance_buffer.slice(..));
				render_pass.draw_indexed(
					indicies_ground.clone(),
					// (36 + 36 + 36)..((36 + 36 + 36 + 36) as u32),
					0,
					0..ground_instance_data.len() as _,
				);

				// Draw chains
				render_pass.set_vertex_buffer(1, chain_instance_buffer.slice(..));
				render_pass.draw_indexed(
					indicies_chain.clone(),
					// (36 + 36)..((36 + 36 + 36) as u32),
					0,
					0..chain_instance_data.len() as _,
				);

				// Draw blocks
				render_pass.set_vertex_buffer(1, block_instance_buffer.slice(..));
				render_pass.draw_indexed(
					indicies_block.clone(),
					//36..((36 + 36) as u32),
					0,
					0..block_instance_data.len() as _,
				);

				// Draw cubes - todo these draw calls can be combined.
				render_pass.set_vertex_buffer(1, extrinsic_instance_buffer.slice(..));
				render_pass.draw_indexed(indicies_cube.clone(), 0, 0..extrinsic_instance_data.len() as _);
				render_pass.set_vertex_buffer(1, event_instance_buffer.slice(..));
				render_pass.draw_indexed(indicies_cube.clone(), 0, 0..event_instance_data.len() as _);

				render_pass.set_vertex_buffer(1, selected_instance_buffer.slice(..));
				render_pass.draw_indexed(
					indicies_selected.clone(),
					// (36 + 36 + 36 + 36)..((36 + 36 + 36 + 36 + 36) as u32),
					0,
					0..selected_instance_data.len() as _,
				);

				render_pass.set_vertex_buffer(1, textured_instance_buffer.slice(..));
				// log!("render textured_instance_data.len() is {} ",textured_instance_data.len());
				render_pass.draw_indexed(
					indicies_textured.clone(),
					0,
					0..textured_instance_data.len() as _,
				);
			}
			queue.submit(std::iter::once(encoder.finish(
		// 		&wgpu::RenderBundleDescriptor {
        //     label: Some("main"),
        // }
			)));

			output_frame.present();

			if Utc::now().timestamp() - frame_time > 1 {
				fps = frames as u32;
				frames = 0;
				frame_time = Utc::now().timestamp();
			}
			// if Utc::now().timestamp() - tx_time > 12 {
				// tps = (tx / 12_u64) as u32;
				// tx = 0;
				let period = Utc::now().timestamp() - tx_time;
				if period > 0 {
					tps = (tx / (period as u64)) as u32;
				}
				// tx_time = Utc::now().timestamp();
			// }

			egui_rpass.remove_textures(tdelta).expect("remove texture ok");

			if camera_controller.rotate_horizontal_stack.len() +
					camera_controller.rotate_vertical_stack.len() > 0
				{
					// log!("make it");
					camera_controller.update_camera(&mut camera, chrono::Duration::milliseconds((1000. / fps as f32) as i64));
					camera_uniform.update_view_proj(&camera, &projection);
					queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));
				}
		}
	});
}

//TODO: use https://github.com/gfx-rs/wgpu/pull/2781
async fn load_textures(
	device: &wgpu::Device,
	queue: &wgpu::Queue,
	sample_count: u32,
) -> (wgpu::TextureView, wgpu::Sampler, HashMap<(u32, u32), (usize, usize)>) {
	// let chain_str = "0-2000";//details.doturl.chain_str();
	// let window = web_sys::window().unwrap();
	let mut width = 0;
	let mut height = 0;
	let mut map = HashMap::new();
	let mut index = 0;
	const H: usize = 40; // 40 images per col
	map.insert((0, 0), (index / H, index % H));
	index += 1;
	map.insert((0, 999), (index / H, index % H));
	index += 1;
	map.insert((0, 1000), (index / H, index % H));
	index += 1;
	map.insert((0, 1001), (index / H, index % H));
	index += 1;
	map.insert((0, 2000), (index / H, index % H));
	index += 1;
	map.insert((0, 2001), (index / H, index % H));
	index += 1;
	map.insert((0, 2004), (index / H, index % H));
	index += 1;
	map.insert((0, 2007), (index / H, index % H));
	index += 1;
	map.insert((0, 2011), (index / H, index % H));
	index += 1;
	map.insert((0, 2012), (index / H, index % H));
	index += 1;
	map.insert((0, 2015), (index / H, index % H));
	index += 1;
	map.insert((0, 2023), (index / H, index % H));
	index += 1;
	map.insert((0, 2048), (index / H, index % H));
	index += 1;
	map.insert((0, 2084), (index / H, index % H));
	index += 1;
	map.insert((0, 2085), (index / H, index % H));
	index += 1;
	map.insert((0, 2086), (index / H, index % H));
	index += 1;
	map.insert((0, 2087), (index / H, index % H));
	index += 1;
	map.insert((0, 2088), (index / H, index % H));
	index += 1;
	map.insert((0, 2090), (index / H, index % H));
	index += 1;
	map.insert((0, 2092), (index / H, index % H));
	index += 1;
	map.insert((0, 2095), (index / H, index % H));
	index += 1;
	map.insert((0, 2096), (index / H, index % H));
	index += 1;
	map.insert((0, 2097), (index / H, index % H));
	index += 1;
	map.insert((0, 2100), (index / H, index % H));
	index += 1;
	map.insert((0, 2102), (index / H, index % H));
	index += 1;
	map.insert((0, 2101), (index / H, index % H));
	index += 1;
	map.insert((0, 2105), (index / H, index % H));
	index += 1;
	map.insert((0, 2106), (index / H, index % H));
	index += 1;
	map.insert((0, 2107), (index / H, index % H));
	index += 1;
	map.insert((0, 2110), (index / H, index % H));
	index += 1;
	map.insert((0, 2113), (index / H, index % H));
	index += 1;
	map.insert((0, 2114), (index / H, index % H));
	index += 1;
	map.insert((0, 2115), (index / H, index % H));
	index += 1;
	map.insert((0, 2118), (index / H, index % H));
	index += 1;
	map.insert((0, 2119), (index / H, index % H));
	index += 1;
	map.insert((0, 2121), (index / H, index % H));
	index += 1;
	map.insert((0, 2123), (index / H, index % H));
	index += 1;
	map.insert((0, 2124), (index / H, index % H));
	index += 1;
	map.insert((0, 2125), (index / H, index % H));
	index += 1;
	map.insert((0, 2129), (index / H, index % H));
	index += 1;

	map.insert((1, 0), (index / H, index % H));
	index += 1;
	map.insert((1, 1000), (index / H, index % H));
	index += 1;
	map.insert((1, 1001), (index / H, index % H));
	index += 1;
	map.insert((1, 2000), (index / H, index % H));
	index += 1;
	map.insert((1, 2002), (index / H, index % H));
	index += 1;
	map.insert((1, 2004), (index / H, index % H));
	index += 1;
	map.insert((1, 2006), (index / H, index % H));
	index += 1;
	map.insert((1, 2007), (index / H, index % H));
	index += 1;
	map.insert((1, 2011), (index / H, index % H));
	index += 1;
	map.insert((1, 2012), (index / H, index % H));
	index += 1;
	map.insert((1, 2013), (index / H, index % H));
	index += 1;
	map.insert((1, 2019), (index / H, index % H));
	index += 1;
	map.insert((1, 2021), (index / H, index % H));
	index += 1;
	map.insert((1, 2026), (index / H, index % H));
	index += 1;
	map.insert((1, 2030), (index / H, index % H));
	index += 1;
	map.insert((1, 2031), (index / H, index % H));
	index += 1;
	map.insert((1, 2032), (index / H, index % H));
	index += 1;
	map.insert((1, 2034), (index / H, index % H));
	index += 1;
	map.insert((1, 2035), (index / H, index % H));
	index += 1;
	map.insert((1, 2037), (index / H, index % H));
	index += 1;
	map.insert((1, 2039), (index / H, index % H));
	index += 1;
	map.insert((1, 2043), (index / H, index % H));
	index += 1;
	map.insert((1, 2046), (index / H, index % H));
	index += 1;
	map.insert((1, 2048), (index / H, index % H));
	index += 1;
	map.insert((1, 2051), (index / H, index % H));
	index += 1;
	map.insert((1, 2052), (index / H, index % H));
	index += 1;
	map.insert((1, 2086), (index / H, index % H)); //index += 1;

	//TODO: MAX height achieved!!! need to go wide...
	// or have another texture buffer.
	// MAX: 16384 for chrome, 8192 for firefox and iOS, but android limits to 4096!

	// images must be inserted in same order as they are in the map.

	//sips -s format jpeg s.png --out ./assets/branding/0-2129.jpeg
	//sips -z 100 300 *.jpeg to format them all to same aspect.
	let mut images = vec![];
	#[cfg(feature = "raw_images")]
	{
		images.push(include_bytes!("../assets/branding/0.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-999.jpeg").to_vec()); //https://text2image.com/en/
		images.push(include_bytes!("../assets/branding/0-1000.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-1001.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2000.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2001.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2004.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2007.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2011.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2012.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2015.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2023.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2048.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2084.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2085.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2086.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2087.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2088.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2090.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2092.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2095.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2096.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2097.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2100.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2102.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2101.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2105.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2106.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2107.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2110.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2113.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2114.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2115.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2118.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2119.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2121.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2123.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2124.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2125.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/0-2129.jpeg").to_vec());

		images.push(include_bytes!("../assets/branding/1.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-1000.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-1001.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2000.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2002.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2004.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2006.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2007.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2011.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2012.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2013.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2019.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2021.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2026.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2030.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2031.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2032.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2034.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2035.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2037.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2039.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2043.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2046.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2048.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2051.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2052.jpeg").to_vec());
		images.push(include_bytes!("../assets/branding/1-2086.jpeg").to_vec());
	}

	#[cfg(not(feature = "raw_images"))]
	{
		images.push(include_bytes!("../assets/branding/baked.jpeg").to_vec());
	}

	//MAX: 16k for chrome, safari. 8k height for firefox.

	let mut diffuse_rgba2 = Vec::new();

	let mut found = 0;
	for bytes in &images {
		// let url = &format!("https://bafybeif4gcbt2q3stnuwgipj2g4tc5lvvpndufv2uknaxjqepbvbrvqrxm.ipfs.dweb.link/{}.jpeg", chain_str);
		// log!("try get {}", url);
		//  let mut opts = RequestInit::new();
		// opts.method("GET");
		// opts.mode(RequestMode::Cors);
		//  let request = Request::new_with_str_and_init(&banner_url, &opts)?;

		// let response = JsFuture::from(window.fetch_with_str(url))
		// .await
		// .map(|r| r.dyn_into::<web_sys::Response>().unwrap())
		// .map_err(|e| e.dyn_into::<js_sys::TypeError>().unwrap());

		// if let Err(err) = &response {
		// 	log!("Failed to fetch asset {url}: {err:?}");
		// }
		// let response = response.unwrap();
		// //.map_err(|_| AssetIoError::NotFound(path.to_path_buf()))?;

		// let data = JsFuture::from(response.array_buffer().unwrap())
		// 	.await
		// 	.unwrap();

		// let bytes = js_sys::Uint8Array::new(&data).to_vec();

		let mut decoder = Decoder::new(std::io::Cursor::new(bytes));
		let diffuse_rgb: Vec<u8> = decoder.decode().expect("failed to decode image");

		let metadata = decoder.info().unwrap();

		// let diffuse_rgba2 = vec![
		// 	255, 0, 0, 255,
		// 	0, 255, 0, 255,
		// 	0, 0, 255, 255,
		// 	255, 0, 255, 255,
		// ];

		if width == 0 {
			width += metadata.width as u32;
			for (i, byte) in diffuse_rgb.iter().enumerate() {
				diffuse_rgba2.push(*byte);
				// Add alpha channel
				if i % 3 == 2 {
					diffuse_rgba2.push(255);
				}
			}
		} else {
			if width == metadata.width as u32 {
				found += 1;

				for (i, byte) in diffuse_rgb.iter().enumerate() {
					diffuse_rgba2.push(*byte);
					// Add alpha channel
					if i % 3 == 2 {
						diffuse_rgba2.push(255);
					}
				}
			}
		}
		height += metadata.height as u32;

		// let width = 2;
		// let height = 2;

		// assert_eq!(diffuse_rgba.len() as u32, width * height * 4);
		// log!("first 100 bytes: {:?}", &diffuse_rgba[..100]);

		log!("metadata: {:?}", metadata);
		// let diffuse = [150_u8;4 * 10 * 10];
		// let diffuse_rgba = &diffuse[..];
	}

	#[cfg(not(feature = "bake"))]
	assert!(height < 4096, "Android phones don't allow textures longer than that");

	log!("found images {found}");
	let diffuse_rgba = diffuse_rgba2.as_slice();

	#[cfg(feature = "bake")]
	{
		const img_height: u16 = 100;
		const img_width: u16 = 300;
		const bake_height: u16 = 4000;
		let image_columns = 1 + dbg!((found * img_height) / bake_height);
		let bake_width = dbg!(img_width * image_columns);
		let images_on_last_column = dbg!(((found * img_height) % bake_height) / img_height);
		let extra_blank_images_needed = (bake_height / img_height) - images_on_last_column;

		// Remove alpha channel as jpegs don't do alpha.
		let mut diffuse_rgba: Vec<u8> = diffuse_rgba2
			.as_slice()
			.iter()
			.enumerate()
			.filter_map(|(i, pixel)| if i % 4 == 3 { None } else { Some(*pixel) })
			.collect();

		// Add blank images to make long enough
		// to be refactored into perfect rectangle:
		for _img in 0..extra_blank_images_needed {
			for _width in 0..img_width {
				for _height in 0..img_height {
					for _color in 0..3 {
						diffuse_rgba.push(0_u8);
					}
				}
			}
		}

		let mut bake_img = vec![]; //0u8; (bake_height as u32 * bake_width as u32) as usize];

		let colors_per_pixel = 3_u32;
		let img_width_px = img_width as u32 * colors_per_pixel; // 3 colors per pixel.
		for y in 0..(bake_height as u32) {
			for x in 0..(bake_width as u32 * colors_per_pixel) {
				let col = x / img_width_px as u32;
				let x_source = x % img_width_px as u32;
				let y_source = y + (col * bake_height as u32);
				let z = ((y_source * img_width_px as u32) + x_source) as usize;
				if z >= diffuse_rgba.len() {
					println!("{col} , {x_source}, {y_source}, {z}");
				}
				bake_img.push(diffuse_rgba[z]);
			}
		}

		use jpeg_encoder::{ColorType, Encoder};
		let mut encoder = Encoder::new_file("some.jpeg", 90).unwrap();
		encoder.encode(&bake_img[..], bake_width, bake_height, ColorType::Rgb).unwrap();
		println!("done initial bake");

		load_textures_emoji(device, queue).await;
		panic!("done");
	}

	let texture_size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

	let usage: wgpu::TextureUsages = texture_usage(sample_count);

	let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
		// All textures are stored as 3D, we represent our 2D texture
		// by setting depth to 1.
		size: texture_size,
		mip_level_count: 1, // We'll talk about this a little later
		sample_count,
		dimension: wgpu::TextureDimension::D2,
		// Most images are stored using sRGB so we need to reflect that here.
		format: wgpu::TextureFormat::Rgba8UnormSrgb,
		// TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
		// COPY_DST means that we want to copy data to this texture
		usage,
		label: Some("diffuse_texture"),
	});

	queue.write_texture(
		// Tells wgpu where to copy the pixel data
		wgpu::ImageCopyTexture {
			texture: &diffuse_texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
			aspect: wgpu::TextureAspect::All,
		},
		// The actual pixel data
		diffuse_rgba,
		// The layout of the texture
		wgpu::ImageDataLayout {
			offset: 0, //TODO for different layouts
			bytes_per_row: std::num::NonZeroU32::new(4 * width),
			rows_per_image: None,
		},
		texture_size,
	);

	let diffuse_texture_view = diffuse_texture.create_view(&default());
	let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: wgpu::AddressMode::Repeat, //Repeat
		address_mode_v: wgpu::AddressMode::Repeat, //ClampToEdge,
		address_mode_w: wgpu::AddressMode::Repeat,
		mag_filter: wgpu::FilterMode::Linear, //Linear,
		min_filter: wgpu::FilterMode::Nearest,
		mipmap_filter: wgpu::FilterMode::Nearest,
		..default()
	});

	(diffuse_texture_view, diffuse_sampler, map)
}

fn emoji_index(emoji_name: char) -> u8 {
	match emoji_name {
		'💀' => 0,
		'👍' => 1,
		'👎' => 2,
		'⚠' => 3,
		'⚓' => 4,
		'🏦' => 5,
		'✒' => 6,
		'🧠' => 7,
		'👤' => 8,
		'👥' => 9,
		'📅' => 10,
		'☑' => 11,
		'🔄' => 12,
		'📩' => 13,
		'♠' => 14,
		'🤠' => 15,
		'🔮' => 16,
		'💱' => 17,
		'🕵' => 18,
		'💵' => 19,
		'🧐' => 20,
		'🌾' => 21,
		'🔥' => 22,
		'🖋' => 23,
		'🖼' => 24,
		'🎲' => 25,
		'⚙' => 26,
		'🐣' => 27,
		'🪦' => 28,
		'💲' => 29,
		'🪪' => 30,
		'📨' => 31,
		'💬' => 32,
		'🔒' => 33,
		'📢' => 34,
		// '🎨" => 35,
		'💟' => 36,
		'🔐' => 37,
		'💰' => 38,
		'📰' => 39,
		'🕘' => 40,
		'🗝' => 41,
		'⏰' => 42,
		'⚗' => 43,
		'📶' => 44,
		'🎨' => 45,
		'👶' => 46,
		'⚖' => 47,
		'💓' => 48,
		'🖤' => 49,
		'🧹' => 50,
		'🥕' => 51,
		'📉' => 52,
		'📈' => 53,
		'🏛' => 54,
		'💥' => 55,
		'🦀' => 56,
		'❌' => 57,
		'😋' => 58,
		'💎' => 59,
		'👻' => 60,
		'🫴' => 61,
		'🥳' => 62,
		'⏸' => 63,
		'⛏' => 64,
		'🐖' => 65,
		'💊' => 66,
		'🤖' => 67,
		'🚀' => 68,
		'🍰' => 69,
		'🔀' => 70,
		'❄' => 71,
		'⭐' => 72,
		'⏱' => 73,
		'🔓' => 74,
		'🪵' => 75,
		'🥶' => 76,
		'🔧' => 77,
		'👾' => 78,
		'🦄' => 79,
		'🛁' => 80,
		_ => 255,
	}
}

/// cargo run --features raw_images,bake to bake
async fn load_textures_emoji(
	device: &wgpu::Device,
	queue: &wgpu::Queue,
	sample_count: u32,
) -> (
	wgpu::TextureView,
	wgpu::Sampler,
	//	HashMap<(&'static str, &'static str), (usize,usize)>
) {
	// let chain_str = "0-2000";//details.doturl.chain_str();
	// let window = web_sys::window().unwrap();
	let mut width = 0;
	let mut height = 0;
	// let mut map = HashMap::new();
	#[cfg(feature = "raw_images")]
	let mut index = 0;
	// const H: usize = 2;// 32; // 32 images per col - 128x128
	// index += 1;
	// map.insert(("balances", "withdraw"), (index / H, index % H)); //index += 1;
	// index += 1;
	// map.insert(("balances", "deposit"), (index / H, index % H)); //index += 1;
	// index += 1;
	// map.insert(("parainclusion", "candidateincluded"), (index / H, index % H)); //index += 1;
	// index += 1;
	// map.insert(("ethereum", "transact"), (index / H, index % H)); //index += 1;

	//TODO: MAX height achieved!!! need to go wide...
	// or have another texture buffer.
	// MAX: 16384 for chrome, 8192 for firefox and iOS, but android limits to 4096!

	// images must be inserted in same order as they are in the map.

	//sips -s format jpeg s.png --out ./assets/branding/0-2129.jpeg
	//sips -z 100 300 *.jpeg to format them all to same aspect.
	let mut images = vec![];
	#[cfg(feature = "raw_images")]
	{
		//rsvg-convert -h 128 ./src/anchor.svg > anchor.png
		let prefix = "/Users/bit/p/dotsamatown/assets/emoji/";

		let v = [
			"skull",
			"thumbs_up",
			"thumbs_down",
			"warning",
			"anchor",
			"bank",
			"black_nib",
			"brain",
			"bust",
			"busts",
			"calendar",
			"check_box_with_check",
			"counter_clockwise_arrows",
			"envelope_with_arrow",
			"spade_suit",
			"cowboy_hat_face",
			"crystal_ball",
			"currency_exchange",
			"detective",
			"dollar",
			"face_with_monocle",
			"farmer",
			"fire",
			"fountain_pen",
			"framed_picture",
			"game_die",
			"gear",
			"hatching_chick",
			"headstone",
			"heavy_dollar",
			"identification_card",
			"incoming_envelope",
			"left_speach_bubble",
			"locked",
			"loudspeaker",
			"woman_artist",
			"heart_decoration",
			"locked_with_key",
			"money_bag",
			"newspaper",
			"nine_oclock",
			"old_key",
			"alarm_clock",
			"alembic",
			"antenna",
			"artistic_palette",
			"baby_symbol",
			"balance_scale",
			"beating_heart",
			"black_heart",
			"broom",
			"carrot",
			"chart_decreasing",
			"chart_increasing",
			"classical_building",
			"collision",
			"crab",
			"cross",
			"face_savoring_food",
			"gem_stone",
			"ghost",
			"palm_up_hand",
			"partying_face",
			"pause",
			"pick",
			"pig",
			"pill",
			"robot",
			"rocket",
			"shortcake",
			"shuffle_tracks",
			"snowflake",
			"star",
			"stopwatch",
			"unlocked",
			"log",
			"cold_face",
			"wrench",
			"alien_monster",
			"unicorn",
			"bathtub",
		];

		for (i, im) in v.iter().enumerate() {
			images.push(format!("{}{}_{}.png", prefix, i, im));
		}
		// images.push(format!("{}{}", prefix, )); //https://text2image.com/en/
		// images.push(format!("{}{}", prefix, "2_thumbs_down.png"));
		// images.push(format!("{}{}", prefix, "0-skull.png"));
	}

	#[cfg(not(feature = "raw_images"))]
	{
		images.push(include_bytes!("../assets/branding/baked-emojis.jpeg").to_vec());
	}

	//MAX: 16k for chrome, safari. 8k height for firefox.

	let mut diffuse_rgba2 = Vec::new();

	let mut found = 0;
	// let mut i = 0;
	for bytes in &images {
		// i += 1;
		// println!("{i} {bytes}");
		#[cfg(feature = "bake")]
		let (img_data, img_width, img_height, add_alpha) = load_png_image(bytes).unwrap();
		#[cfg(not(feature = "bake"))]
		let (img_data, img_width, img_height, add_alpha) = {
			let mut decoder = Decoder::new(std::io::Cursor::new(bytes));
			let diffuse_rgb: Vec<u8> = decoder.decode().expect("failed to decode image");

			let metadata = decoder.info().unwrap();
			(diffuse_rgb, metadata.width, metadata.height, true)
		};

		// let url = &format!("https://bafybeif4gcbt2q3stnuwgipj2g4tc5lvvpndufv2uknaxjqepbvbrvqrxm.ipfs.dweb.link/{}.jpeg", chain_str);
		// log!("try get {}", url);
		//  let mut opts = RequestInit::new();
		// opts.method("GET");
		// opts.mode(RequestMode::Cors);
		//  let request = Request::new_with_str_and_init(&banner_url, &opts)?;

		// let response = JsFuture::from(window.fetch_with_str(url))
		// .await
		// .map(|r| r.dyn_into::<web_sys::Response>().unwrap())
		// .map_err(|e| e.dyn_into::<js_sys::TypeError>().unwrap());

		// if let Err(err) = &response {
		// 	log!("Failed to fetch asset {url}: {err:?}");
		// }
		// let response = response.unwrap();
		// //.map_err(|_| AssetIoError::NotFound(path.to_path_buf()))?;

		// let data = JsFuture::from(response.array_buffer().unwrap())
		// 	.await
		// 	.unwrap();

		// let bytes = js_sys::Uint8Array::new(&data).to_vec();

		// let mut decoder = Decoder::new(std::io::Cursor::new(bytes));
		// let diffuse_rgb: Vec<u8> = decoder.decode().expect("failed to decode image");

		// let decoder = image_png::Decoder::new(bytes);

		// let metadata = decoder.info().unwrap();

		// let diffuse_rgba2 = vec![
		// 	255, 0, 0, 255,
		// 	0, 255, 0, 255,
		// 	0, 0, 255, 255,
		// 	255, 0, 255, 255,
		// ];

		if width == 0 {
			width += img_width as u32;
			for (i, byte) in img_data.iter().enumerate() {
				diffuse_rgba2.push(*byte);
				// Add alpha channel
				if add_alpha && i % 3 == 2 {
					diffuse_rgba2.push(255);
				}
			}
		} else {
			if width == img_width as u32 {
				found += 1;

				for (i, byte) in img_data.iter().enumerate() {
					diffuse_rgba2.push(*byte);
					// Add alpha channel
					if add_alpha && i % 3 == 2 {
						diffuse_rgba2.push(255);
					}
				}
			}
		}
		//	assert!(img.format == png::ColorType::GreyScale);
		height += img_height as u32;

		// let width = 2;
		// let height = 2;

		// assert_eq!(diffuse_rgba.len() as u32, width * height * 4);
		// log!("first 100 bytes: {:?}", &diffuse_rgba[..100]);

		// log!("metadata: {:?}", metadata);
		// let diffuse = [150_u8;4 * 10 * 10];
		// let diffuse_rgba = &diffuse[..];
	}

	#[cfg(not(feature = "bake"))]
	assert!(height < 4096, "Android phones don't allow textures longer than that");

	log!("found images {found}");
	let diffuse_rgba = diffuse_rgba2.as_slice();

	#[cfg(feature = "bake")]
	{
		const grid_height: u16 = 9u16;
		const img_height: u16 = 128;
		const img_width: u16 = 128;
		const bake_height: u16 = 128 * grid_height;
		let image_columns = 1 + dbg!((found * img_height) / bake_height);
		let bake_width = dbg!(img_width * image_columns);
		let images_on_last_column = dbg!(((found * img_height) % bake_height) / img_height);
		let extra_blank_images_needed = (bake_height / img_height) - images_on_last_column;

		// Remove alpha channel as jpegs don't do alpha.
		let mut diffuse_rgba: Vec<u8> = diffuse_rgba2
			.as_slice()
			.iter()
			.enumerate()
			.filter_map(|(i, pixel)| if i % 4 == 3 { None } else { Some(*pixel) })
			.collect();

		// Add blank images to make long enough
		// to be refactored into perfect rectangle:
		for _img in 0..extra_blank_images_needed {
			for _width in 0..img_width {
				for _height in 0..img_height {
					for _color in 0..3 {
						diffuse_rgba.push(0_u8);
					}
				}
			}
		}

		let mut bake_img = vec![]; //0u8; (bake_height as u32 * bake_width as u32) as usize];

		let colors_per_pixel = 3_u32;
		let img_width_px = img_width as u32 * colors_per_pixel; // 3 colors per pixel.
		for y in 0..(bake_height as u32) {
			for x in 0..(bake_width as u32 * colors_per_pixel) {
				let col = x / img_width_px as u32;
				let x_source = x % img_width_px as u32;
				let y_source = y + (col * bake_height as u32);
				let z = ((y_source * img_width_px as u32) + x_source) as usize;
				if z >= diffuse_rgba.len() {
					println!("{col} , {x_source}, {y_source}, {z}");
				}
				bake_img.push(diffuse_rgba[z]);
			}
		}

		use jpeg_encoder::{ColorType, Encoder};
		let mut encoder = Encoder::new_file("some-emojis.jpeg", 90).unwrap();
		encoder.encode(&bake_img[..], bake_width, bake_height, ColorType::Rgb).unwrap();
		println!("hi");
		panic!("done");
	}

	let texture_size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

	let usage: wgpu::TextureUsages = texture_usage(sample_count);

	let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
		// All textures are stored as 3D, we represent our 2D texture
		// by setting depth to 1.
		size: texture_size,
		mip_level_count: 1, // We'll talk about this a little later
		sample_count,
		dimension: wgpu::TextureDimension::D2,
		// Most images are stored using sRGB so we need to reflect that here.
		format: wgpu::TextureFormat::Rgba8UnormSrgb,
		// TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
		// COPY_DST means that we want to copy data to this texture
		usage,
		label: Some("diffuse_texture"),
	});

	queue.write_texture(
		// Tells wgpu where to copy the pixel data
		wgpu::ImageCopyTexture {
			texture: &diffuse_texture,
			mip_level: 0,
			origin: wgpu::Origin3d::ZERO,
			aspect: wgpu::TextureAspect::All,
		},
		// The actual pixel data
		diffuse_rgba,
		// The layout of the texture
		wgpu::ImageDataLayout {
			offset: 0, //TODO for different layouts
			bytes_per_row: std::num::NonZeroU32::new(4 * width),
			rows_per_image: None,
		},
		texture_size,
	);

	let diffuse_texture_view = diffuse_texture.create_view(&default());
	let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
		address_mode_u: wgpu::AddressMode::Repeat, //Repeat
		address_mode_v: wgpu::AddressMode::Repeat, //ClampToEdge,
		address_mode_w: wgpu::AddressMode::Repeat,
		mag_filter: wgpu::FilterMode::Linear, //Linear,
		min_filter: wgpu::FilterMode::Nearest,
		mipmap_filter: wgpu::FilterMode::Nearest,
		..default()
	});

	(diffuse_texture_view, diffuse_sampler)
}

fn texture_usage(sample_count: u32) -> wgpu::TextureUsages {
	if sample_count == 1 {
		wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
	} else {
		wgpu::TextureUsages::TEXTURE_BINDING
		// wgpu::TextureUsages::COPY_DST |
		| wgpu::TextureUsages::RENDER_ATTACHMENT
	}
}

#[cfg(feature = "raw_images")]
fn load_png_image(path: &str) -> std::io::Result<(Vec<u8>, u16, u16, bool)> {
	use png::ColorType::*;
	use std::{borrow::Cow, fs::File};
	let mut decoder = png::Decoder::new(File::open(path)?);
	decoder.set_transformations(png::Transformations::normalize_to_color8());
	let mut reader = decoder.read_info()?;
	let mut img_data = vec![0; reader.output_buffer_size()];
	let info = reader.next_frame(&mut img_data)?;

	let data = match info.color_type {
		// Rgb => (img_data, png::ClientFormat::U8U8U8),
		Rgba => img_data,
		// Grayscale => (
		//     {
		//         let mut vec = Vec::with_capacity(img_data.len() * 3);
		//         for g in img_data {
		//             vec.extend([g, g, g].iter().cloned())
		//         }
		//         vec
		//     },
		//     png::ClientFormat::U8U8U8,
		// ),
		// GrayscaleAlpha => (
		//     {
		//         let mut vec = Vec::with_capacity(img_data.len() * 3);
		//         for ga in img_data.chunks(2) {
		//             let g = ga[0];
		//             let a = ga[1];
		//             vec.extend([g, g, g, a].iter().cloned())
		//         }
		//         vec
		//     },
		//     png::ClientFormat::U8U8U8U8,
		// ),
		_ => unreachable!("uncovered color type"),
	};

	Ok((data, info.width as u16, info.height as u16, false))
	// format: format,
}

fn try_select(
	camera: &camera::Camera,
	projection: &camera::Projection,
	opengl_to_wgpu_matrix_mat4: glam::Mat4,
	extrinsic_instance_data: &[Instance],
	event_instance_data: &[Instance],
	selected_instance_data: &mut Vec<Instance>,
	scale_x: f32,
	scale_y: f32,
	size: &PhysicalSize<u32>,
	position: &PhysicalPosition<f64>,
) {
	let matrix = camera.calc_matrix();
	let x: glam::Vec4 = glam::Vec4::new(matrix.x.x, matrix.x.y, matrix.x.z, matrix.x.w);
	let y: glam::Vec4 = glam::Vec4::new(matrix.y.x, matrix.y.y, matrix.y.z, matrix.y.w);
	let z: glam::Vec4 = glam::Vec4::new(matrix.z.x, matrix.z.y, matrix.z.z, matrix.z.w);
	let w: glam::Vec4 = glam::Vec4::new(matrix.w.x, matrix.w.y, matrix.w.z, matrix.w.w);
	let view = glam::Mat4::from_cols(x, y, z, w);

	let matrix =
		cgmath::perspective(projection.fovy, projection.aspect, projection.znear, projection.zfar);
	let x: glam::Vec4 = glam::Vec4::new(matrix.x.x, matrix.x.y, matrix.x.z, matrix.x.w);
	let y: glam::Vec4 = glam::Vec4::new(matrix.y.x, matrix.y.y, matrix.y.z, matrix.y.w);
	let z: glam::Vec4 = glam::Vec4::new(matrix.z.x, matrix.z.y, matrix.z.z, matrix.z.w);
	let w: glam::Vec4 = glam::Vec4::new(matrix.w.x, matrix.w.y, matrix.w.z, matrix.w.w);
	let proj = opengl_to_wgpu_matrix_mat4 * glam::Mat4::from_cols(x, y, z, w);

	let far_ndc = projection.zfar; //proj.project_point3(glam::Vec3::NEG_Z).z;
	let near_ndc = projection.znear; //camera.position.z;// proj.project_point3(glam::Vec3::Z).z;
	let ndc_to_world: glam::Mat4 = view.inverse() * proj.inverse();

	// log!("new pos: {:?}", position);
	let clicked1 = glam::Vec2::new(position.x as f32, position.y as f32);
	let clicked2 = glam::Vec2::new(
		clicked1.x - size.width as f32 / 2.0,
		size.height as f32 / 2.0 - clicked1.y,
	);
	// log!("new add: {:?}", clicked);
	let clicked = glam::Vec2::new(clicked2.x / scale_x, clicked2.y / scale_y);
	// log!("new adj: {:?}  {:?}  {:?}", clicked1, clicked2, clicked);

	let near_clicked = ndc_to_world.project_point3(clicked.extend(near_ndc));
	let far_clicked = ndc_to_world.project_point3(clicked.extend(far_ndc));
	let ray_direction_clicked = near_clicked - far_clicked;
	let pos_clicked: glam::Vec3 = near_clicked;

	let selected = get_selected(
		pos_clicked,
		ray_direction_clicked,
		event_instance_data,
		glam::Vec3::new(CUBE_WIDTH, CUBE_WIDTH, CUBE_WIDTH),
	);
	log!("selected = {:?}", selected);
	if let Some((index, instance)) = selected {
		// ground_instance_data.push(Instance { position: near_clicked.into(), color:
		// as_rgba_u32(0.3, 0.3, 0.3, 1.) });
		let mut pos = instance.position;
		pos[0] += -0.1;
		pos[1] += -0.1;
		pos[2] += -0.1;
		selected_instance_data.clear();

		// This alpha selects the cold face emoji which the shader special cases to
		// be selected cube.
		selected_instance_data
			.push(Instance { position: pos, color: as_rgba_u32(0.1, 0.1, 0.9, 0.3) });

		(*REQUESTS.lock().unwrap()).push(BridgeMessage::GetEventDetails(index));
	} else {
		let selected = get_selected(
			pos_clicked,
			ray_direction_clicked,
			extrinsic_instance_data,
			glam::Vec3::new(CUBE_WIDTH, CUBE_WIDTH, CUBE_WIDTH),
		);
		if let Some((index, instance)) = selected {
			selected_instance_data.clear();
			selected_instance_data.push(create_selected_instance(&instance));

			(*REQUESTS.lock().unwrap()).push(BridgeMessage::GetExtrinsicDetails(index));
		}
	}
}

fn create_selected_instance(picked_instance: &Instance) -> Instance {
	let mut pos = picked_instance.position;
		pos[0] += -0.1;
		pos[1] += -0.1;
		pos[2] += -0.1;

	// This alpha selects the cold face emoji which the shader special cases to
	// be selected cube.
	Instance { position: pos, color: as_rgba_u32(0.1, 0.1, 0.9, 0.3) }
}

fn get_selected(
	r_org: glam::Vec3,
	mut r_dir: glam::Vec3,
	instances: &[Instance],
	to_rt: glam::Vec3,
) -> Option<(u32, Instance)> {
	r_dir = r_dir.normalize();
	//From:
	//https://gamedev.stackexchange.com/questions/18436/most-efficient-aabb-vs-ray-collision-algorithms
	// r_dir is unit direction vector of ray
	let dirfrac = glam::Vec3::new(1.0f32 / r_dir.x, 1.0f32 / r_dir.y, 1.0f32 / r_dir.z);

	let mut best = None;
	let mut distance = f32::MAX;

	for (i, instance) in instances.iter().enumerate() {
		let lb: glam::Vec3 = Into::<glam::Vec3>::into(instance.position); // position is axis aligned bottom left.
		let rt: glam::Vec3 = Into::<glam::Vec3>::into(instance.position) + to_rt; // + size.

		// lb is the corner of AABB with minimal coordinates - left bottom, rt is maximal corner
		// r_org is origin of ray
		let t1 = (lb.x - r_org.x) * dirfrac.x;
		let t2 = (rt.x - r_org.x) * dirfrac.x;
		let t3 = (lb.y - r_org.y) * dirfrac.y;
		let t4 = (rt.y - r_org.y) * dirfrac.y;
		let t5 = (lb.z - r_org.z) * dirfrac.z;
		let t6 = (rt.z - r_org.z) * dirfrac.z;

		let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
		// let tmin = t1.clamp(t3.min(t4), t2).max(t5.min(t6));
		let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));
		// let tmax = t1.clamp(t2, t3.max(t4)).min(t5.max(t6));

		// if tmax < 0, ray (line) is intersecting AABB, but the whole AABB is behind us
		if tmax < 0. {
			continue
		}

		// if tmin > tmax, ray doesn't intersect AABB
		if tmin > tmax {
			continue
		}

		let is_closest = tmin < distance;
		if is_closest {
			distance = tmin;
			best = Some((i as u32, *instance));
		}
	}
	best
}

/// Call after changing size.
fn resize(
	size: &PhysicalSize<u32>,
	device: &wgpu::Device,
	surface_config: &mut wgpu::SurfaceConfiguration,
	projection: &mut camera::Projection,
	surface: &mut wgpu::Surface,
	depth_texture: &mut texture::Texture,
	hidpi_factor_f64: f64,
	camera_uniform: &mut camera::CameraUniform,
	camera: &mut camera::Camera,
	sample_count: u32,
	platform: &mut Platform,
	window_id: &WindowId,
) {
	// let window = web_sys::window().expect("no global `window` exists");
	// let document = window.document().expect("should have a document on window");
	// let canvas = document
	// .get_element_by_id("bevycanvas")
	// .unwrap()
	// .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
	// canvas.set_width(size.width);
	// canvas.set_height(size.height);

	let hidpi_factor = hidpi_factor_f64 as f32;
	let dpi_width = size.width as f32 * hidpi_factor;
	let dpi_height = size.height as f32 * hidpi_factor;
	surface_config.width = size.width; //dpi_width as u32;
	surface_config.height = size.height; //dpi_height as u32;
	surface.configure(device, surface_config);

	projection.resize(dpi_width as u32, dpi_height as u32);

	camera_uniform.update_view_proj(camera, projection);

	*depth_texture = texture::Texture::create_depth_texture(
		device,
		surface_config,
		"depth_texture",
		sample_count,
	);

	// Resize egui:
	use winit::event::Event::WindowEvent;
	// use winit::event::WindowEvent;
	//TODO: egui thinks screen is 2x size that it is.
	platform.handle_event::<winit::event::WindowEvent>(&WindowEvent {
		window_id: *window_id,
		event: winit::event::WindowEvent::Resized(PhysicalSize {
			width: dpi_width as u32,
			height: dpi_height as u32,
		}),
	});

	let mut s = *size;
	platform.handle_event::<winit::event::WindowEvent>(&WindowEvent {
		window_id: *window_id,
		event: winit::event::WindowEvent::ScaleFactorChanged {
			scale_factor: hidpi_factor_f64,
			new_inner_size: &mut s,
		},
	});
}

// struct DataSourceStreamEvent(ChainInfo, datasource::DataUpdate);

fn chain_name_to_url(chain_names: &Vec<&str>) -> Vec<String> {
	let mut results = Vec::new();
	for chain_name in chain_names {
		let mut chain_name = chain_name.to_string();
		if !chain_name.starts_with("ws:") && !chain_name.starts_with("wss:") {
			chain_name = format!("wss://{}", chain_name);
		}

		results.push(if chain_name[5..].contains(':') {
			chain_name
		} else {
			format!("{chain_name}:443")
		});
	}
	results
}

// // fn start_background_audio(asset_server: Res<AssetServer>, audio: Res<Audio>) {
// //     audio.play_looped(asset_server.load("sounds/backtrack.ogg"));
// // }
// fn source_data<'a, 'b, 'c, 'd, 'e, 'f,'g,'h,'i,'j>(
// 	mut datasource_events: EventReader<'a, 'b, DataSourceChangedEvent>,
// 	mut commands: Commands<'c,'d>,
// 	mut sovereigns: ResMut<'e, Sovereigns>,
// 	details: Query<'f, 'g, Entity, With<ClearMeAlwaysVisible>>,
// 	clean_me: Query<'h, 'i, Entity, With<ClearMe>>,
// 	// mut dest: ResMut<Destination>,
// 	mut spec: ResMut<'j, UrlBar>,
// ) {
// 	// async_std::task::block_on(source_data_async(datasource_events, commands, sovereigns,
// 	// 	 details, clean_me, spec));
// }

// #[cfg(target_arch="wasm32")]
// #[derive(Component)]
// struct SourceDataTask(bevy_tasks::FakeTask);

#[cfg(not(target_arch = "wasm32"))]
async fn send_it_to_desktop(update: (RenderUpdate, RenderDetails)) {
	// log!("Got some results....! yay they're already in the right place. {}", blocks.len());
	UPDATE_QUEUE.lock().unwrap().extend(update.0);
}

#[cfg(not(target_arch = "wasm32"))]
// #[derive(Component)]
struct SourceDataTask(Result<(), std::boxed::Box<dyn std::error::Error + Send + Sync>>);

// fn send_it_to_main(_blocks: Vec<datasource::DataUpdate>) //+ Send + Sync + 'static
// {
// 	log!("got a block!!!");
// }

fn source_data(
	event: DataSourceChangedEvent,
	// mut commands: Commands,
	mut sovereigns: Sovereigns,
	// details: Query<Entity, With<ClearMeAlwaysVisible>>,
	// clean_me: Query<Entity, With<ClearMe>>,
	mut spec: &mut UrlBar,
	// handles: Res<ResourceHandles>,
	// #[cfg(not(target_arch="wasm32"))]
	// writer: EventWriter<DataSourceStreamEvent>,
) {
	// for event in datasource_events.iter() {
	log!("data source changes to {} {:?}", event.source, event.timestamp);

	// clear_world(&details, &mut commands, &clean_me);

	if event.source.is_empty() {
		log!("Datasources cleared epoc {}", DATASOURCE_EPOC.load(Ordering::Relaxed));
		return
	}

	let dot_url = DotUrl::parse(&event.source);
	log!("dot_url parsed {:?}", &dot_url);

	let is_live = if let Some(timestamp) = event.timestamp {
		// if time is now or in future then we are live mode.
		let is_live = timestamp >= (Utc::now().timestamp() * 1000);
		// log!("basetime set by source_data={}", timestamp);
		if is_live {
			*BASETIME.lock().unwrap() = Utc::now().timestamp() * 1000;
			spec.timestamp = Utc::now().naive_utc();
			spec.reset_changed();
		} else {
			*BASETIME.lock().unwrap() = timestamp;
		}
		is_live
	} else {
		event.source.ends_with("live")
	};
	// if is_live {
	// 	event.timestamp = None;
	// }

	log!("event source {}", event.source);
	#[cfg(target_arch = "wasm32")]
	const HIST_SPEED: f32 = 0.05;
	#[cfg(not(target_arch = "wasm32"))]
	const HIST_SPEED: f32 = 0.7;
	log!("is live {}", is_live);
	sovereigns.default_track_speed = if is_live { 0.1 } else { HIST_SPEED };

	log!("tracking speed set to {}", sovereigns.default_track_speed);
	let (dot_url, as_of): (DotUrl, Option<DotUrl>) = if is_live {
		let env = event.source.split(':').collect::<Vec<_>>()[0].to_string();
		let env = Env::try_from(env.as_str()).unwrap_or(Env::Prod);
		(DotUrl { env, ..default() }, None)
	} else {
		(dot_url.clone().unwrap(), Some(dot_url.unwrap()))
	};

	let selected_env = &dot_url.env;
	log!("dot url {:?}", &dot_url);
	//let as_of = Some(dot_url.clone());
	log!("Block number selected for relay chains: {:?}", &as_of);

	let networks = networks::get_network(selected_env);

	// let is_self_sovereign = selected_env.is_self_sovereign();
	let relays = networks
		.into_iter()
		.enumerate()
		.map(|(relay_index, relay)| {
			let relay_url = DotUrl {
				sovereign: Some(if relay_index == 0 { -1 } else { 1 }),
				block_number: None,
				..dot_url.clone()
			};
			//relay.as_slice().par_iter().
			relay
				.as_slice()
				.iter()
				.enumerate()
				.map(|(chain_index, (para_id, chain_name, chain_names))| {
					let url = chain_name_to_url(chain_names);

					// #[cfg(not(target_arch="wasm32"))]
					// let para_id =
					// async_std::task::block_on(datasource::get_parachain_id_from_url(&mut
					// source)); #[cfg(target_arch="wasm32")]
					// let para_id:  Result<Option<NonZeroU32>, polkapipe::Error> = if
					// datasource::is_relay_chain(&url) { Ok(None) } else
					// {Ok(Some(NonZeroU32::try_from(7777u32).unwrap()))}; if para_id.is_err() {
					// 	return None;
					// }
					//let para_id = para_id.unwrap();

					// Chain {
					// shared: send_it_to_main,
					// // name: chain_name.to_string(),
					// info:
					ChainInfo {
						chain_ws: url,
						// +2 to skip 0 and relay chain.
						chain_index: if relay_url.is_darkside() {
							-((chain_index + 2) as isize)
						} else {
							(chain_index + 2) as isize
						},
						chain_name: chain_name.clone(),
						chain_url: DotUrl { para_id: *para_id, ..relay_url.clone() },
						// chain_name: parachain_name,
					}
				})
				.collect::<Vec<ChainInfo>>()
		})
		.collect::<Vec<Vec<_>>>();

	sovereigns.relays.truncate(0);
	for relay in relays.iter() {
		let mut sov_relay = vec![];
		for chain in relay.iter() {
			// log!("set soverign index to {} {}", chain.chain_index, chain.chain_url);
			sov_relay.push(chain.clone());
		}
		sovereigns.relays.push(sov_relay);
	}

	#[cfg(not(target_arch = "wasm32"))]
	do_datasources(sovereigns, as_of);

	#[cfg(target_family = "wasm")]
	let t = async move || {
		log("send to bridge");

		let bridge: WorkerBridge<IOWorker> = crate::webworker::IOWorker::spawner()
			.callback(|result| match result {
				WorkerResponse::RenderUpdate(update, free_txs) => {
					// log!("free tx {}", free_txs);
					FREE_TXS.store(free_txs, Ordering::Relaxed);
					let mut pending = UPDATE_QUEUE.lock().unwrap();
					pending.extend(update);
				},
				WorkerResponse::Details(selected_details) => {//index, details, chain_info
					log!("got selected from backend");
					*SELECTED.lock().unwrap() = selected_details;
				},
			})
			.spawn("./worker.js");

		let bridge = Box::leak(Box::new(bridge));
		bridge.send(BridgeMessage::SetDatasource(
			sovereigns.clone(),
			as_of,
			DATASOURCE_EPOC.load(Ordering::Relaxed),
		));

		loop {
			if let Some(msg) = REQUESTS.lock().unwrap().pop() {
				bridge.send(msg);
			} else {
				bridge.send(BridgeMessage::GetNewBlocks);
			}
			async_std::task::sleep(Duration::from_millis(300)).await;
			// log!("asking bridge msg...");
		}
	};

	#[cfg(target_family = "wasm")]
	wasm_bindgen_futures::spawn_local(t());
	#[cfg(target_family = "wasm")]
	log!("sent to bridge");
}

#[cfg(not(target_arch = "wasm32"))]
fn do_datasources(sovereigns: Sovereigns, as_of: Option<DotUrl>) {
	for relay in sovereigns.relays.into_iter() {
		let mut relay2: Vec<(ChainInfo, _)> = vec![];
		let mut send_map: HashMap<
			u32,
			async_std::channel::Sender<(datasource::RelayBlockNumber, i64, H256)>,
		> = default();
		for chain in relay.into_iter() {
			let (tx, rc) = async_std::channel::unbounded();
			if let Some(para_id) = chain.chain_url.para_id {
				send_map.insert(para_id, tx);
			}
			relay2.push((chain, rc));
		}

		let mut send_map = Some(send_map);
		for (chain, rc) in relay2 {
			// log!("listening to {}", chain.info.chain_ws);

			let maybe_sender = if chain.chain_url.is_relay() { send_map.take() } else { None };

			let as_of = as_of.clone();
			// log!("as of for chain {:?} index {}", &as_of, chain.chain_index);
			let chain_info = chain.clone();

			let block_watcher = datasource::BlockWatcher {
				tx: Some(send_it_to_desktop),
				chain_info,
				as_of,
				receive_channel: Some(rc),
				sender: maybe_sender,
				forwards: true
			};

			std::thread::spawn(
				//thread_pool.spawn_local
				move || {
					async_std::task::block_on(block_watcher.watch_blocks());
				},
			);
		}
	}
}

#[cfg(target_arch = "wasm32")]
async fn do_datasources<F, R>(
	sovereigns: Sovereigns,
	as_of: Option<DotUrl>,
	callback: &'static F,
) where
	F: (Fn((RenderUpdate, RenderDetails)) -> R) + Send + Sync + 'static,
	R: Future<Output = ()> + 'static,
{
	for relay in sovereigns.relays.iter() {
		let mut relay2: Vec<(ChainInfo, _)> = vec![];
		let mut send_map: HashMap<
			u32,
			async_std::channel::Sender<(datasource::RelayBlockNumber, i64, H256)>,
		> = default();
		for chain in relay.iter() {
			let (tx, rc) = async_std::channel::unbounded();
			if let Some(para_id) = chain.chain_url.para_id {
				send_map.insert(para_id, tx);
			}
			relay2.push((chain.clone(), rc));
		}

		let mut send_map = Some(send_map);
		for (chain, rc) in relay2 {
			// log!("listening to {}", chain.info.chain_ws);

			let maybe_sender = if chain.chain_url.is_relay() { send_map.take() } else { None };

			let as_of = as_of.clone();
			let chain_info = chain.clone();
			// log!("as of for chain {:?} index {}", &as_of, chain.chain_index);

			let block_watcher = datasource::BlockWatcher {
				tx: Some(callback),
				chain_info,
				as_of,
				receive_channel: Some(rc),
				sender: maybe_sender,
				forwards: true,
			};

			#[cfg(target_arch = "wasm32")]
			wasm_bindgen_futures::spawn_local(block_watcher.watch_blocks());

			#[cfg(not(target_arch = "wasm32"))]
			block_watcher.watch_blocks().await;
		}
	}
}

fn draw_chain_rect(
	// handles: &ResourceHandles,
	chain_info: &ChainInfo,
	// commands: &mut Commands,
	chain_instances: &mut Vec<Instance>,
) {
	let rfip = chain_info.chain_url.rflip();
	let chain_index = chain_info.chain_index.unsigned_abs();
	// let encoded: String = url::form_urlencoded::Serializer::new(String::new())
	// 	.append_pair("rpc", &chain_info.chain_ws)
	// 	.finish();
	let is_relay = chain_info.chain_url.is_relay();
	// commands
	// 	.spawn_bundle(PbrBundle {
	// 		mesh: handles.chain_rect_mesh.clone(),
	// 		material: if chain_info.chain_url.is_darkside() {
	// 			handles.darkside_rect_material.clone()
	// 		} else {
	// 			handles.lightside_rect_material.clone()
	// 		},
	// 		transform: Transform::from_translation(Vec3::new(
	// 			(10000. / 2.) - 35.,
	// 			if is_relay { 0. } else { LAYER_GAP },
	// 			((RELAY_CHAIN_CHASM_WIDTH - 5.) +
	// 				(BLOCK / 2. + BLOCK_AND_SPACER * chain_index as f32)) *
	// 				rfip,
	// 		)),

	// 		..Default::default()
	// 	})
	// 	.insert(Details {
	// 		doturl: DotUrl { block_number: None, ..chain_info.chain_url.clone() },
	// 		flattern: chain_info.chain_ws.to_string(),
	// 		url: format!("https://polkadot.js.org/apps/?{}", &encoded),
	// 		..default()
	// 	})
	// 	.insert(Name::new("Blockchain"))
	// 	.insert(ClearMeAlwaysVisible)
	// 	.insert(bevy::render::view::NoFrustumCulling);

	// let mat = if chain_info.chain_url.is_darkside() {
	// 	handles.darkside_rect_material.clone()
	// } else {
	// 	handles.lightside_rect_material.clone()
	// };

	chain_instances.push(Instance {
		position: glam::Vec3::new(
			0. - 35., //(1000. / 2.) - 35.,
			if is_relay { 0. } else { LAYER_GAP } - CHAIN_HEIGHT / 2.0,
			((RELAY_CHAIN_CHASM_WIDTH - 5.) + (BLOCK / 2. + BLOCK_AND_SPACER * chain_index as f32)) *
				rfip,
		)
		.into(),
		// scale: 0.,
		color: if chain_info.chain_url.is_darkside() {
			as_rgba_u32(0.2, 0.2, 0.2, 1.)
		} else {
			as_rgba_u32(0.4, 0.4, 0.4, 1.)
		},
		// flags: 0,
	});
	// chain_instances.1.push(true);
}

fn as_rgba_u32(red: f32, green: f32, blue: f32, alpha: f32) -> u32 {
	u32::from_le_bytes([
		(red * 255.0) as u8,
		(green * 255.0) as u8,
		(blue * 255.0) as u8,
		(alpha * 255.0) as u8,
	])
}

fn as_rgbemoji_u32(red: f32, green: f32, blue: f32, alpha: u8) -> u32 {
	u32::from_le_bytes([(red * 255.0) as u8, (green * 255.0) as u8, (blue * 255.0) as u8, alpha])
}

// fn clear_world(// details: &Query<Entity, With<ClearMeAlwaysVisible>>,
// 	// commands: &mut Commands,
// 	// clean_me: &Query<Entity, With<ClearMe>>,
// ) {
// 	// Stop previous data sources...
// 	DATASOURCE_EPOC.fetch_add(1, Ordering::Relaxed);
// 	log!("incremet epoc to {}", DATASOURCE_EPOC.load(Ordering::Relaxed));

// 	// for detail in details.iter() {
// 	// 	commands.entity(detail).despawn();
// 	// }
// 	// for detail in clean_me.iter() {
// 	// 	commands.entity(detail).despawn();
// 	// }
// 	*BASETIME.lock().unwrap() = 0;
// }

// #[derive(Clone, Copy)]
// enum BuildDirection {
// 	Up,
// 	// Down,
// }

#[derive(Clone, Serialize, Deserialize)]
pub enum DataEntity {
	Event(DataEvent),
	Extrinsic {
		// id: (u32, u32),
		// pallet: String,
		// variant: String,
		args: Vec<String>,
		contains: Vec<DataEntity>,
		// raw: Vec<u8>,
		/// pseudo-unique id to link to some other node(s).
		/// There can be multiple destinations per block! (TODO: need better resolution)
		/// Is this true of an extrinsic - system ones plus util batch could do multiple msgs.
		msg_count: u32,
		start_link: Vec<(String, LinkType)>,
		/// list of links that we have finished
		end_link: Vec<(String, LinkType)>,
		details: Details,
	},
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DataEvent {
	details: Details,
	start_link: Vec<(String, LinkType)>,
	end_link: Vec<(String, LinkType)>,
}

/// A tag to identify an entity as being the source of a message.
// #[derive(Component)]
pub struct MessageSource {
	/// Currently sending block id + hash of beneficiary address.
	pub id: String,
	pub link_type: LinkType,
	pub source: Option<[f32;3]>,
	pub source_index: usize,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum LinkType {
	Teleport,
	ReserveTransfer,
	ReserveTransferMintDerivative,
	ParaInclusion,
	Balances,
}

static EMPTY_SLICE: Vec<DataEntity> = vec![];
// static EMPTY_BYTE_SLICE: Vec<u8> = vec![];

impl DataEntity {
	pub fn details(&self) -> &Details {
		match self {
			Self::Event(DataEvent { details, .. }) => details,
			Self::Extrinsic { details, .. } => details,
		}
	}

	pub fn details_mut(&mut self) -> &mut Details {
		match self {
			Self::Event(DataEvent { details, .. }) => details,
			Self::Extrinsic { details, .. } => details,
		}
	}

	pub fn pallet(&self) -> &str {
		match self {
			Self::Event(DataEvent { details, .. }) => details.pallet.as_ref(),
			Self::Extrinsic { details, .. } => &details.pallet,
		}
	}
	pub fn dot(&self) -> &DotUrl {
		match self {
			Self::Event(DataEvent { details, .. }) => &details.doturl,
			Self::Extrinsic { details, .. } => &details.doturl,
		}
	}
	pub fn variant(&self) -> &str {
		match self {
			Self::Event(DataEvent { details, .. }) => details.variant.as_ref(),
			Self::Extrinsic { details, .. } => &details.variant,
		}
	}

	pub fn contains(&self) -> &[DataEntity] {
		match self {
			Self::Event(DataEvent { .. }) => EMPTY_SLICE.as_slice(),
			Self::Extrinsic { contains, .. } => contains.as_slice(),
		}
	}

	// pub fn as_bytes(&self) -> &[u8] {
	// 	self.details().raw.as_slice()
	// 	// match self {
	// 	// 	Self::Event(DataEvent { .. }) => EMPTY_BYTE_SLICE.as_slice(),
	// 	// 	Self::Extrinsic { raw, .. } => raw.as_slice(),
	// 	// }
	// }

	pub fn start_link(&self) -> &Vec<(String, LinkType)> {
		match self {
			Self::Extrinsic { start_link, .. } => start_link,
			Self::Event(DataEvent { .. }) => &EMPTY_VEC,
		}
	}

	pub fn end_link(&self) -> &Vec<(String, LinkType)> {
		match self {
			Self::Extrinsic { end_link, .. } => end_link,
			Self::Event(DataEvent { .. }) => &EMPTY_VEC,
		}
	}
}

static EMPTY_VEC: Vec<(String, LinkType)> = vec![];

const BLOCK: f32 = 10.;
const BLOCK_AND_SPACER: f32 = BLOCK + 4.;
const RELAY_CHAIN_CHASM_WIDTH: f32 = 10.;

#[derive(Serialize, Deserialize, Clone)]
pub struct Sovereigns {
	//                            name    para_id             url
	pub relays: Vec<Vec<ChainInfo>>,
	pub default_track_speed: f32,
}

impl Sovereigns {
	fn chain_info(&self, doturl: &DotUrl) -> ChainInfo {
		//TODO work for 3+ sovs
		let sov = &self.relays[usize::from(!doturl.is_darkside())];
		if let Some(para_id) = doturl.para_id {
			for chain_info in sov {
				if chain_info.chain_url.para_id == Some(para_id) {
					return chain_info.clone()
				}
			}
			panic!("chain info not found for para id: {}", para_id);
		} else {
			sov[0].clone()
		}
	}
}

// #[derive(Component)]
// struct ClearMe;

// #[derive(Component)]
// struct ClearMeAlwaysVisible;

// fn pad_system(gamepads: Res<Gamepads>) {
//     // iterates every active game pad
//     for gamepad in gamepads.iter() {
//         println!("pad found");
//     }
// }

// Convert from x to timestamp
pub fn x_to_timestamp(x: f32) -> i64 {
	let zero = *BASETIME.lock().unwrap();
	(zero + (x as f64 * 400.) as i64) / 1000
}

pub fn timestamp_to_x(timestamp: i64) -> f32 {
	let zero = *BASETIME.lock().unwrap();
	(((timestamp - zero) as f64) / 400.) as f32
}

#[derive(Serialize, Deserialize, Default)]
pub struct RenderUpdate {
	chain_instances: Vec<Instance>,
	block_instances: Vec<Instance>,
	extrinsic_instances: Vec<(Instance, f32)>,
	event_instances: Vec<(Instance, f32)>,
	textured_instances: Vec<Instance>,
	basetime: Option<NonZeroI64>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct RenderDetails {
	chain_instances: Vec<Details>,
	block_instances: Vec<Details>,
	extrinsic_instances: Vec<Details>,
	event_instances: Vec<Details>,
}

impl RenderUpdate {
	fn extend(&mut self, update: RenderUpdate) {
		self.chain_instances.extend(update.chain_instances);
		self.block_instances.extend(update.block_instances);
		self.extrinsic_instances.extend(update.extrinsic_instances);
		self.event_instances.extend(update.event_instances);
		self.textured_instances.extend(update.textured_instances);
		if update.basetime.is_some() {
			self.basetime = update.basetime;
		}
	}

	fn any(&self) -> bool {
		self.count() > 0
	}

	fn count(&self) -> usize {
		self.chain_instances.len() +
			self.block_instances.len() +
			self.extrinsic_instances.len() +
			self.event_instances.len() +
			self.textured_instances.len()
	}
}

impl RenderDetails {
	fn extend(&mut self, update: RenderDetails) {
		self.chain_instances.extend(update.chain_instances);
		self.block_instances.extend(update.block_instances);
		self.extrinsic_instances.extend(update.extrinsic_instances);
		self.event_instances.extend(update.event_instances);
	}
}

fn render_block(
	data_update: DataUpdate,
	// mut commands: Commands,
	// mut materials: ResMut<Assets<StandardMaterial>>,
	chain_info: &ChainInfo,
	// asset_server: Res<AssetServer>,
	links: &mut Vec<MessageSource>,
	// mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
	// mut polylines: ResMut<Assets<Polyline>>,
	// mut event: EventWriter<RequestRedraw>,
	// mut handles: ResMut<ResourceHandles>,
	render: &mut RenderUpdate,
	// mut event_instances: &mut Vec<(Instance, f32)>,
	// mut block_instances: &mut Vec<Instance>,
	// mut chain_instances: &mut Vec<Instance>,
	render_details: &mut RenderDetails,
) {
	// for mut extrinsic_instances in extrinsic_instances.iter_mut() {
	// 	for mut event_instances in event_instances.iter_mut() {
	// 		for mut block_instances in block_instances.iter_mut() {

	if links.len() > 0 {
		log!("links {}", links.len());
	}

	match data_update {
		DataUpdate::NewBlock(block) => {
			//TODO optimise!
			// let mut chain_info = None;
			// 'outer: for r in &relays.relays {
			// 	for rchain_info in r {
			// 		if rchain_info.chain_url.contains(&block.blockurl) {
			// 			// web_sys::console::log_1(&format!("{} contains {}",
			// 			// rchain_info.chain_url, block.blockurl).into());
			// 			chain_info = Some(rchain_info);
			// 			if !rchain_info.chain_url.is_relay() {
			// 				break 'outer
			// 			}
			// 		}
			// 	}
			// }

			// let chain_info = chain_info.unwrap();

			// println!( - can see from instance counts now if needed.
			// 	"chains {} blocks {} txs {} events {}",
			// 	CHAINS.load(Ordering::Relaxed),
			// 	BLOCKS.load(Ordering::Relaxed),
			// 	EXTRINSICS.load(Ordering::Relaxed),
			// 	EVENTS.load(Ordering::Relaxed)
			// );
			// log!("block rend chain index {}", chain_info.chain_index);

			// Skip data we no longer care about because the datasource has changed
			let now_epoc = DATASOURCE_EPOC.load(Ordering::Relaxed);
			if block.data_epoc != now_epoc {
				log!(
					"discarding out of date block made at {} but we are at {}",
					block.data_epoc,
					now_epoc
				);
				return
			}

			let mut base_time = *BASETIME.lock().unwrap();
			if base_time == 0 {
				base_time = block.timestamp.unwrap_or(0);
				if base_time != 0 {
					// log!("BASETIME set to {}", base_time);
					*BASETIME.lock().unwrap() = base_time;
					render.basetime = Some(NonZeroI64::new(base_time).unwrap());
				}
			}

			// let block_num = if is_self_sovereign {
			//     block.blockurl.block_number.unwrap() as u32
			// } else {

			//     if base_time == 0
			//     if rcount == 0 {
			//         if chain == 0 &&  {
			//             //relay
			//             RELAY_BLOCKS.store(
			//                 RELAY_BLOCKS.load(Ordering::Relaxed) + 1,
			//                 Ordering::Relaxed,
			//             );
			//         }
			//         RELAY_BLOCKS.load(Ordering::Relaxed)
			//     } else {
			//         if chain == 0 {
			//             //relay
			//             RELAY_BLOCKS2.store(
			//                 RELAY_BLOCKS2.load(Ordering::Relaxed) + 1,
			//                 Ordering::Relaxed,
			//             );
			//         }
			//         RELAY_BLOCKS2.load(Ordering::Relaxed)
			//     }
			// };

			let rflip = chain_info.chain_url.rflip();
			// let encoded: String = form_urlencoded::Serializer::new(String::new())
			// 	.append_pair("rpc", &chain_info.chain_ws)
			// 	.finish();

			let is_relay = chain_info.chain_url.is_relay();
			let details = Details {
				doturl: DotUrl { extrinsic: None, event: None, ..block.blockurl.clone() },

				// url: format!(
				// 	"https://polkadot.js.org/apps/?{}#/explorer/query/{}",
				// 	&encoded,
				// 	block.blockurl.block_number.unwrap()
				// ),
				..default()
			};
			// log!("rendering block from {}", details.doturl);

			// println!("block.timestamp {:?}", block.timestamp);
			// println!("base_time {:?}",base_time);
			let block_num = timestamp_to_x(block.timestamp.unwrap_or(base_time));

			if block_num < 0. {
				// negative blocks look messy - let's not show those.
				return
			}

			// Add the new block as a large square on the ground:
			{
				let timestamp_color = if chain_info.chain_url.is_relay() {
					// log!("skiping relay block from {} as has no timestamp", details.doturl);
					if block.timestamp.is_none() {
						return
					}
					block.timestamp.unwrap()
				} else {
					if block.timestamp_parent.is_none() && block.timestamp.is_none() {
						// log!("skiping block from {} as has no timestamp", details.doturl);
						return
					}
					block.timestamp_parent.unwrap_or_else(|| block.timestamp.unwrap())
				} / 400;

				// let transform = Transform::from_translation(Vec3::new(
				// 	0. + (block_num as f32),
				// 	if is_relay { 0. } else { LAYER_GAP },
				// 	(RELAY_CHAIN_CHASM_WIDTH +
				// 		BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32) *
				// 		rflip,
				// ));
				// println!("block created at {:?} blocknum {}", transform,
				// block_num);

				// let mut bun = commands.spawn_bundle(PbrBundle {
				// 	mesh: handles.block_mesh.clone(),
				// 	material: materials.add(StandardMaterial {
				// 		base_color: style::color_block_number(
				// 			timestamp_color, /* TODO: material needs to be cached by
				// 			                  * color */
				// 			chain_info.chain_url.is_darkside(),
				// 		), // Color::rgba(0., 0., 0., 0.7),
				// 		alpha_mode: AlphaMode::Blend,
				// 		perceptual_roughness: 0.08,
				// 		unlit: block.blockurl.is_darkside(),
				// 		..default()
				// 	}),
				// 	transform,
				// 	..Default::default()
				// });
				// bun.insert(ClearMe);

				render.block_instances.push(Instance {
					position: glam::Vec3::new(
						0. + block_num - 5.,
						if is_relay { -0.1 } else { -0.1 + LAYER_GAP },
						(RELAY_CHAIN_CHASM_WIDTH +
							BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32) *
							rflip,
					)
					.into(),
					// scale: 0.,
					color: style::color_block_number(
						timestamp_color,
						chain_info.chain_url.is_darkside(),
					),
					// flags: 0,
				});
				render_details.block_instances.push(details);
				// block_instances.1.push(false);

				// let chain_str = details.doturl.chain_str();
				// &format!("https://bafybeif4gcbt2q3stnuwgipj2g4tc5lvvpndufv2uknaxjqepbvbrvqrxm.ipfs.dweb.link/{}.jpeg", chain_str)

				render.textured_instances.push(Instance {
					position: glam::Vec3::new(
						0. + block_num - 8.5,
						if is_relay { -0.1 } else { -0.1 + LAYER_GAP },
						(0.1 + RELAY_CHAIN_CHASM_WIDTH +
							BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32) *
							chain_info.chain_url.rflip(),
					)
					.into(),
					// Encode the chain / parachain instead of the instance.color data.
					// This will get translated to
					color: if chain_info.chain_url.is_darkside() { 0 } else { 100_000 } +
						chain_info.chain_url.para_id.unwrap_or(0),
				});
				// bun.insert(details)
				// .insert(Name::new("Block"))
				// .with_children(|parent| {
				// 	let material_handle =
				// handles.banner_materials.entry(chain_info.chain_index).
				// or_insert_with(|| { 		// You can use https://cid.ipfs.tech/#Qmb1GG87ufHEvXkarzYoLn9NYRGntgZSfvJSBvdrbhbSNe
				// 		// to convert from CID v0 (starts Qm) to CID v1 which most
				// gateways use. 		#[cfg(target_arch="wasm32")]
				// 		let texture_handle = asset_server.load(&format!("https://bafybeif4gcbt2q3stnuwgipj2g4tc5lvvpndufv2uknaxjqepbvbrvqrxm.ipfs.dweb.link/{}.jpeg", chain_str));
				// 		#[cfg(not(target_arch="wasm32"))]
				// 		let texture_handle =
				// asset_server.load(&format!("branding/{}.jpeg", chain_str));

				// 		materials.add(StandardMaterial {
				// 			base_color_texture: Some(texture_handle),
				// 			alpha_mode: AlphaMode::Blend,
				// 			unlit: true,
				// 			..default()
				// 		})
				// 	}).clone();

				// 	// textured quad - normal
				// 	let rot =
				// 		Quat::from_euler(EulerRot::XYZ, -PI / 2., -PI, PI / 2.); //
				// to_radians()

				// 	let transform = Transform {
				// 		translation: Vec3::new(
				// 			-7.,
				// 			0.1,
				// 			0.,
				// 		),
				// 		rotation: rot,
				// 		..default()
				// 	};

				// 	parent
				// 		.spawn_bundle(PbrBundle {
				// 			mesh: handles.banner_mesh.clone(),
				// 			material: material_handle.clone(),
				// 			transform,
				// 			..default()
				// 		})
				// 		.insert(Name::new("BillboardDown"))
				// 		.insert(ClearMe);

				// 	// textured quad - normal
				// 	let rot =
				// 		Quat::from_euler(EulerRot::XYZ, -PI / 2., 0., -PI / 2.); //
				// to_radians() 	let transform = Transform {
				// 		translation: Vec3::new(-7.,0.1,0.),
				// 		rotation: rot,
				// 		..default()
				// 	};

				// 	parent
				// 		.spawn_bundle(PbrBundle {
				// 			mesh: handles.banner_mesh.clone(),
				// 			material: material_handle,
				// 			transform,
				// 			..default()
				// 		})
				// 		.insert(Name::new("BillboardUp"))
				// 		.insert(ClearMe);
				// })
				// .insert_bundle(PickableBundle::default());
			}
			// return;
			let ext_with_events =
				datasource::associate_events(block.extrinsics.clone(), block.events);

			// // Leave infrastructure events underground and show user activity above
			// // ground.
			// let (_boring, fun): (Vec<_>, Vec<_>) =
			// 	ext_with_events.into_iter().partition(|(e, _)| {
			// 		// if let Some(ext) = e {
			// 		// 	content::is_utility_extrinsic(ext)
			// 		// } else {
			// 			false
			// 		//}
			// 	});

			add_blocks(
				chain_info,
				block_num,
				ext_with_events,
				// &mut commands,
				// &mut materials,
				// BuildDirection::Up,
				links,
				// &mut polyline_materials,
				// &mut polylines,
				// &encoded,
				// &mut handles,
				&mut render.extrinsic_instances,
				&mut render.event_instances,
				render_details, // &mut event_dest, // &mut event_instances,
			);

			// add_blocks(
			// 	chain_info,
			// 	block_num,
			// 	boring,
			// 	// &mut commands,
			// 	// &mut materials,
			// 	BuildDirection::Down,
			// 	links,
			// 	// &mut polyline_materials,
			// 	// &mut polylines,
			// 	// &encoded,
			// 	// &mut handles,
			// 	&mut render.extrinsic_instances,
			// 	&mut render.event_instances,
			// 	render_details, // &mut event_dest, // &mut event_instances,
			// );
			//event.send(RequestRedraw);
		},
		DataUpdate::NewChain(chain_info, sudo) => {
			let is_relay = chain_info.chain_url.is_relay();
			// log!("adding new chain");
			render.textured_instances.push(Instance {
				position: glam::Vec3::new(
					0. - 8.5 - 28.,
					if is_relay { -0.13 } else { -0.13 + LAYER_GAP },
					(0.1 + RELAY_CHAIN_CHASM_WIDTH +
						BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32) *
						chain_info.chain_url.rflip(),
				)
				.into(),
				color: if chain_info.chain_url.is_darkside() { 0 } else { 100_000 } +
					chain_info.chain_url.para_id.unwrap_or(0),
			});

			render.textured_instances.push(Instance {
				position: glam::Vec3::new(
					0. - 8.5 - 28. - 3.3,
					if is_relay { -0.13 } else { -0.13 + LAYER_GAP },
					(0.1 + RELAY_CHAIN_CHASM_WIDTH +
						BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32) *
						chain_info.chain_url.rflip(),
				)
				.into(),
				color: if chain_info.chain_url.is_darkside() { 0 } else { 100_000 },
			});

			if sudo {
				render.textured_instances.push(Instance {
					position: glam::Vec3::new(
						0. - 8.5 - 28. + 3.3,
						if is_relay { -0.13 } else { -0.13 + LAYER_GAP },
						(0.1 + RELAY_CHAIN_CHASM_WIDTH +
							BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32) *
							chain_info.chain_url.rflip(),
					)
					.into(),
					color: 999,
				});
			}

			// for mut chain_instances in chain_instances.iter_mut() {
			draw_chain_rect(
				// handles.as_ref(),
				&chain_info,
				// &mut commands,
				&mut render.chain_instances,
			)
			// }
		},
	}
	//}
	// 		}
	// 	}
	// }
}

// TODO allow different block building strategies. maybe dependent upon quantity of blocks in the
// space?
fn add_blocks(
	chain_info: &ChainInfo,
	block_num: f32,
	block_events: Vec<(Option<DataEntity>, Vec<(usize, DataEvent)>)>,
	// build_direction: BuildDirection,
	links: &mut Vec<MessageSource>,
	// polyline_materials: &mut ResMut<Assets<PolylineMaterial>>,
	// polylines: &mut ResMut<Assets<Polyline>>,
	// encoded: &str,
	extrinsic_instances: &mut Vec<(Instance, f32)>,
	event_instances: &mut Vec<(Instance, f32)>,
	render_details: &mut RenderDetails,
) {
	let rflip = chain_info.chain_url.rflip();
	let build_dir = 1.0; //if let BuildDirection::Up = build_direction { 1.0 } else { -1.0 };
	// Add all the useful blocks

	let layer = chain_info.chain_url.layer() as f32;
	let (base_x, base_y, base_z) = (
		(block_num) - 4.,
		LAYER_GAP * layer,
		RELAY_CHAIN_CHASM_WIDTH + BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32 - 4.,
	);

	// if let BuildDirection::Down = build_direction {
	// 	base_y -= 0.5;
	// }

	const DOT_HEIGHT: f32 = 1.;
	const HIGH: f32 = 100.;
	let mut rain_height: [f32; 81] = [HIGH; 81];
	let mut next_y: [f32; 81] = [0.5; 81]; // Always positive.

	for (event_num, (block, events)) in block_events.iter().enumerate() {
		let z = event_num % 9;
		let x = (event_num / 9) % 9;

		rain_height[event_num % 81] += fastrand::f32() * HIGH;

		let (px, py, pz) = (base_x + x as f32, rain_height[event_num % 81], (base_z + z as f32));

		// let transform = Transform::from_translation(Vec3::new(px, py * build_dir, pz * rflip));

		if let Some(block @ DataEntity::Extrinsic { .. }) = block {
			for block in std::iter::once(block).chain(block.contains().iter()) {
				let target_y = next_y[event_num % 81];
				next_y[event_num % 81] += DOT_HEIGHT;
				// let dark = block.details().doturl.is_darkside();
				let style = style::style_event(block);
				// let material = mat_map.entry(style.clone()).or_insert_with(|| {
				// 	materials.add(if dark {
				// 		StandardMaterial {
				// 			base_color: style.color,
				// 			emissive: style.color,
				// 			perceptual_roughness: 0.3,
				// 			metallic: 1.0,
				// 			..default()
				// 		}
				// 	} else {
				// 		style.color.into()
				// 	})
				// });
				// let mesh = if content::is_message(block) {
				// 	handles.xcm_torus_mesh.clone()
				// } else if matches!(block, DataEntity::Extrinsic { .. }) {
				// 	handles.extrinsic_mesh.clone()
				// } else {
				// 	handles.sphere_mesh.clone()
				// };

				// let call_data = format!("0x{}", hex::encode(block.as_bytes()));

				// let mut create_source = vec![];

				for (link, _link_type) in block.end_link() {
					log!("end link typw {:?}!", _link_type);
					//if this id already exists then this is the destination, not the source...
					for id in links.iter() {
						if id.id == *link {
							log!("creating rainbow!");

							// let mut vertices = vec![
							// 	source_global.translation(),
							// 	Vec3::new(px, base_y + target_y * build_dir, pz * rflip),
							// ];
							// rainbow(&mut vertices, 50);

							// let colors = vec![
							// 	Color::PURPLE,
							// 	Color::BLUE,
							// 	Color::CYAN,
							// 	Color::YELLOW,
							// 	Color::RED,
							// ];
							// for color in colors.into_iter() {
							// 	// Create rainbow from entity to current extrinsic location.
							// 	// commands
							// 	// 	.spawn_bundle(PolylineBundle {
							// 	// 		polyline: polylines
							// 	// 			.add(Polyline { vertices: vertices.clone() }),
							// 	// 		material: polyline_materials.add(PolylineMaterial {
							// 	// 			width: 10.0,
							// 	// 			color,
							// 	// 			perspective: true,
							// 	// 			..default()
							// 	// 		}),
							// 	// 		..default()
							// 	// 	})
							// 	// 	.insert(ClearMe);

							// 	for v in vertices.iter_mut() {
							// 		v.y += 0.5;
							// 	}
							// }

							// commands.entity(entity).remove::<MessageSource>();
						}
					}
				}

				for (link, link_type) in block.start_link() {
					log!("inserting source of rainbow!");
					links.push(MessageSource { source_index:render_details.extrinsic_instances.len(), id: link.to_string(), link_type: *link_type,
						source:Some(glam::Vec3::new(px, py * build_dir, 5. + pz * rflip).into()) });
				}

				// let mut bun = commands.spawn_bundle(PbrBundle {
				// 	mesh,
				// 	/// * event.blocknum as f32
				// 	material: material.clone(),
				// 	transform,
				// 	..Default::default()
				// });

				// bun.insert_bundle(PickableBundle::default())
				// 	.insert(block.details().clone())
				// 	// .insert(Details {
				// 	// 	// hover: format_entity(block),
				// 	// 	// data: (block).clone(),http://192.168.1.241:3000/#/extrinsics/decode?calldata=0
				// 	// 	doturl: block.dot().clone(),
				// 	// 	flattern: block.details().flattern.clone(),
				// 	// 	url: format!(
				// 	// 		"https://polkadot.js.org/apps/?{}#/extrinsics/decode/{}",
				// 	// 		&encoded, &call_data
				// 	// 	),
				// 	// 	parent: None,
				// 	// 	success: ui::details::Success::Happy,
				// 	// 	pallet: block.pallet().to_string(),
				// 	// 	variant: block.variant().to_string(),
				// 	// 	raw: block.as_bytes().to_vec(),
				// 	// 	value: block.details().value
				// 	// })
				// 	.insert(ClearMe)
				// 	// .insert(Rainable { dest: base_y + target_y * build_dir, build_direction })
				// 	.insert(Name::new("Extrinsic"))
				// 	.insert(MedFi);

				extrinsic_instances.push((
					Instance {
						position: glam::Vec3::new(px, py * build_dir, 5. + pz * rflip).into(),
						// scale: base_y + target_y * build_dir,
						color: style.color,
						// flags: 0,
					},
					base_y + target_y * build_dir,
				));

				// let extrinsic_details = block.details().clone();
				// TODO: link to decode for extrinsics.
				// extrinsic_details.url = format!("https://polkadot.subscan.io/extrinsic/{}-{}",
				// extrinsic_details.doturl.block_number.unwrap(),
				// extrinsic_details.doturl.extrinsic.unwrap());
				render_details.extrinsic_instances.push(block.details().clone());

				// for source in create_source {
				// 	bun.insert(source);
				// }
			}
		}

		//let mut events = events.clone();
		//events.sort_unstable_by_key(|e| e.details.pallet.to_string() + &e.details.variant);
		//TODO keep original order a bit

		// for event_group in events.group_by(|a, b| {
		//     a.details.pallet == b.details.pallet && a.details.variant == b.details.variant
		// }) {
		//     let event_group: Vec<_> = event_group.iter().collect();

		//     let height = event_group.len() as f32;
		//     let annoying = DataEvent {
		//         details: Details {
		//             pallet: event_group[0].details.pallet.clone(),
		//             doturl: event_group[0].details.doturl.clone(),
		//             parent: event_group[0].details.parent.clone(),
		//             variant: event_group[0].details.variant.clone(),
		//             success: event_group[0].details.success.clone(),
		//             hover: event_group[0].details.hover.clone(),
		//             flattern: event_group[0].details.flattern.clone(),
		//             url: event_group[0].details.url.clone(),
		//         },
		//         start_link: vec![],
		//     };
		//     let event_group = if event_group.len() == 1 {
		//         event_group
		//     } else {
		//         vec![&annoying]
		//     };
		// let blocklink =
		// 	format!("https://polkadot.js.org/apps/?{}#/explorer/query/{}", &encoded, block_num);

		for (_block_event_index, event) in events {
			let style = style::style_data_event(event);
			//TODO: map should be a resource.
			// let material = mat_map.entry(style.clone()).or_insert_with(|| {
			// 	materials.add(if dark {
			// 		StandardMaterial {
			// 			base_color: style.color,
			// 			emissive: style.color,
			// 			perceptual_roughness: 0.3,
			// 			metallic: 1.0,
			// 			..default()
			// 		}
			// 	} else {
			// 		style.color.into()
			// 	})
			// });

			// let mesh = if content::is_event_message(&entity) {
			// 	handles.xcm_torus_mesh.clone()
			// } else {
			// 	handles.sphere_mesh.clone()
			// };
			rain_height[event_num % 81] += DOT_HEIGHT; // * height;
			let target_y = next_y[event_num % 81];
			next_y[event_num % 81] += DOT_HEIGHT; // * height;

			let (x, y, z) = (px, rain_height[event_num % 81] * build_dir, pz * rflip);
			event_instances.push((
				Instance {
					position: glam::Vec3::new(x, (5. * build_dir) + y, 5. + z).into(),
					// scale: base_y + target_y * build_dir,
					color: style.color,
					// flags: 0,
				},
				base_y + target_y * build_dir,
			));

			// let details = entity.details.clone();
			// details.url = format!("https://polkadot.subscan.io/extrinsic/{}-{}",
			// details.doturl.block_number.unwrap(),
			// block_event_index);

			let event_index = render_details.event_instances.len();
			render_details.event_instances.push(event.details.clone());

			for (link, link_type) in &event.start_link {
				// println!("inserting source of rainbow (an event)!");
				links.push(MessageSource { source_index: event_index, id: link.to_string(), link_type: *link_type , source: Some(glam::Vec3::new(x, (5. * build_dir) + y, 5. + z).into())});
			}

			let end_loc : [f32;3] = glam::Vec3::new(x, (5. * build_dir) + y, 5. + z).into();
			for (link, link_type) in &event.end_link {
				log!("checking links: {}", links.len());
				for MessageSource { source_index, id, link_type, source } in links.iter() {
					// double link:
					render_details.event_instances[event_index].links.push(*source_index);
					if *source_index < render_details.event_instances.len() {
						render_details.event_instances[*source_index].links.push(event_index);
					}
					else {
						log!("link found fin first?!!!!! from {source:?} {source_index} to {event_index} {end_loc:?}");

					}
					if *id == *link {
						log!("link found start to fin!!!!! from {source:?} to {end_loc:?}");

						// rainbow_instances.push((
						// 	RainbowInstance {
						// 		position: source,
						// 		destination: end_loc,
						// 		color: style.color,
						// 	},
						// 	0,
						// ));
					}
				}
			}
		}
	}
}

/// Yes this is now a verb. Who knew?
fn _rainbow(vertices: &mut Vec<glam::Vec3>, points: usize) {
	let start = vertices[0];
	let end = vertices[1];
	let diff = end - start;
	// println!("start {:#?}", start);
	// println!("end {:#?}", end);
	// println!("diff {:#?}", diff);
	// x, z are linear interpolations, it is only y that goes up!

	let center = (start + end) / 2.;
	// println!("center {:#?}", center);
	let r = end - center;
	// println!("r {:#?}", r);
	let radius = (r.x * r.x + r.y * r.y + r.z * r.z).sqrt(); // could be approximate
														 // println!("vertst {},{},{}", start.x, start.y, start.z);
														 // println!("verten {},{},{}", end.x, end.y, end.z);
	vertices.truncate(0);
	let fpoints = points as f32;
	for point in 0..=points {
		let proportion = point as f32 / fpoints;
		let x = start.x + proportion * diff.x;
		let y = (proportion * PI).sin() * radius;
		let z = start.z + proportion * diff.z;
		// println!("vertex {},{},{}", x, y, z);
		vertices.push(glam::Vec3::new(x, y, z));
	}
}

// #[derive(Deref, DerefMut)]
// struct AnimationTimer(Timer);

// https://stackoverflow.com/questions/53706611/rust-max-of-multiple-floats
macro_rules! max {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => {{
        let y = max!($($z),*);
        if $x > y {
            $x
        } else {
            y
        }
    }}
}
macro_rules! min {
    ($x: expr) => ($x);
    ($x: expr, $($z: expr),+) => {{
        let y = min!($($z),*);
        if $x < y {
            $x
        } else {
            y
        }
    }}
}

fn rain(
	// time: Res<Time>,
	drops: &mut [Instance],
	drops_target: &mut [f32], // mut timer: ResMut<UpdateTimer>,
) {
	let delta = 1.;
	// if timer.timer.tick(time.delta()).just_finished() {
	// for mut rainable in drops.iter_mut() {
	for (i, r) in drops.iter_mut().enumerate() {
		let dest = drops_target[i]; //TODO zip
		if dest != 0. {
			let y = r.position[1];
			if dest > 0. {
				if y > dest {
					let todo = y - dest;
					let delta = min!(1., delta * (todo / dest));

					r.position[1] = max!(dest, y - delta);
					// Stop raining...
					if delta < f32::EPSILON {
						drops_target[i] = 0.;
					}
				}
			} else {
				// Austrialian down under world. Balls coming up from the depths...
				if y < dest {
					r.position[1] = min!(dest, y + delta);
					// Stop raining...
					if delta < f32::EPSILON {
						drops_target[i] = 0.;
					}
				}
			}
		}
	}
}

// pub struct UpdateTimer {
// 	timer: Timer,
// }
// use bevy_egui::EguiContext;
// pub fn print_events(
// 	mut events: EventReader<PickingEvent>,
// 	mut query2: Query<(Entity, &Details, &GlobalTransform)>,
// 	mut urlbar: ResMut<ui::UrlBar>,
// 	mut inspector: ResMut<Inspector>,
// 	mut custom: EventWriter<DataSourceChangedEvent>,
// 	mut dest: ResMut<Destination>,
// 	mut anchor: ResMut<Anchor>,

// 	// Is egui using the mouse?
// 	// mut egui_context: ResMut<EguiContext>, // TODO: this doesn't need to be mut.
// ) {
// 	// let ctx = &mut egui_context.ctx_mut();
// 	// // If we're over an egui area we should not be trying to select anything.
// 	// if ctx.is_pointer_over_area() {
// 	// 	return
// 	// }
// 	// if urlbar.changed() {
// 	// 	urlbar.reset_changed();
// 	// 	let timestamp = urlbar.timestamp();

// 	// 	custom.send(DataSourceChangedEvent { source: urlbar.location.clone(), timestamp });
// 	// }
// 	// for event in events.iter() {
// 	// 	match event {
// 	// 		PickingEvent::Selection(selection) => {
// 	// 			if let SelectionEvent::JustSelected(_entity) = selection {
// 	// 				//  let mut inspector_window_data = inspector_windows.window_data::<Details>();
// 	// 				//   let window_size =
// 	// 				// &world.get_resource::<ExtractedWindowSizes>().unwrap().0[&self.window_id];

// 	// 				// let selection = query.get_mut(*entity).unwrap();

// 	// 				// Unspawn the previous text:
// 	// 				// query3.for_each_mut(|(entity, _)| {
// 	// 				//     commands.entity(entity).despawn();
// 	// 				// });

// 	// 				// if inspector.active == Some(details) {
// 	// 				//     print!("deselected current selection");
// 	// 				//     inspector.active = None;
// 	// 				// } else {

// 	// 				// }

// 	// 				// info!("{}", details.hover.as_str());
// 	// 				// decode_ex!(events, crate::polkadot::ump::events::UpwardMessagesReceived,
// 	// 				// value, details);
// 	// 			}
// 	// 		},
// 	// 		PickingEvent::Hover(e) => {
// 	// 			// info!("Egads! A hover event!? {:?}", e)

// 	// 			match e {
// 	// 				HoverEvent::JustEntered(entity) => {
// 	// 					let (_entity, details, _global_location) = query2.get_mut(*entity).unwrap();
// 	// 					inspector.hovered = Some(if details.doturl.extrinsic.is_some() {
// 	// 						format!("{} - {} ({})", details.doturl, details.variant, details.pallet)
// 	// 					} else {
// 	// 						format!("{}", details.doturl)
// 	// 					});
// 	// 				},
// 	// 				HoverEvent::JustLeft(_) => {
// 	// 					//	inspector.hovered = None;
// 	// 				},
// 	// 			}
// 	// 		},
// 	// 		PickingEvent::Clicked(entity) => {
// 	// 			let now = Utc::now().timestamp_millis() as i32;
// 	// 			let prev = LAST_CLICK_TIME.swap(now as i32, Ordering::Relaxed);
// 	// 			let (_entity, details, global_location) = query2.get_mut(*entity).unwrap();
// 	// 			if let Some(selected) = &inspector.selected {
// 	// 				if selected.doturl == details.doturl && now - prev >= 400 {
// 	// 					inspector.selected = None;
// 	// 					return
// 	// 				}
// 	// 			}
// 	// 			inspector.selected = Some(details.clone());
// 	// 			inspector.texture = None;

// 	// 			// info!("Gee Willikers, it's a click! {:?}", e)

// 	// 			// use async_std::task::block_on;
// 	// 			// 				use serde_json::json;
// 	// 			// 				let metad = block_on(datasource::get_desub_metadata(&url, &mut source,
// 	// 			// None)).unwrap(); 				if let Ok(extrinsic) =
// 	// 			// 					decoder::decode_unwrapped_extrinsic(&metad, &mut details.raw.as_slice())
// 	// 			// 				{
// 	// 			// 					println!("{:#?}", extrinsic);
// 	// 			// 				} else {
// 	// 			// 					println!("could not decode.");
// 	// 			// 				}
// 	// 			// 				serde_json::to_value(&value);

// 	// 			if now - prev < 400 {
// 	// 				println!("double click {}", now - prev);
// 	// 				// if you double clicked on just a chain then you really don't want to get sent
// 	// 				// to the middle of nowhere!
// 	// 				if details.doturl.block_number.is_some() {
// 	// 					println!("double clicked to {}", details.doturl);
// 	// 					anchor.follow_chain = false; // otherwise when we get to the destination then we will start
// moving away 	// 						 // from it.
// 	// 					dest.location = Some(global_location.translation());
// 	// 				}
// 	// 			}
// 	// 		},
// 	// 	}
// 	// }
// }

// struct Width(f32);

// static LAST_CLICK_TIME: AtomicI32 = AtomicI32::new(0);
// static LAST_KEYSTROKE_TIME: AtomicI32 = AtomicI32::new(0);

// fn update_visibility(
// 	// mut entity_low_midfi: Query<(
// 	// 	&mut Visibility,
// 	// 	&GlobalTransform,
// 	// 	With<ClearMe>,
// 	// 	Without<HiFi>,
// 	// 	Without<MedFi>,
// 	// )>,
// 	// mut entity_medfi: Query<(&mut Visibility, &GlobalTransform, With<MedFi>, Without<HiFi>)>,
// 	// mut entity_hifi: Query<(&mut Visibility, &GlobalTransform, With<HiFi>, Without<MedFi>)>,
// 	// player_query: Query<&Transform, With<Viewport>>,
// 	frustum: Query<&Frustum, With<Viewport>>,
// 	mut instances: Query<&mut InstanceMaterialData, Without<ChainInstances>>,
// 	// #[cfg(feature = "adaptive-fps")] diagnostics: Res<'_, Diagnostics>,
// 	// #[cfg(feature = "adaptive-fps")] mut visible_width: ResMut<Width>,
// 	// #[cfg(not(feature = "adaptive-fps"))] visible_width: Res<Width>,
// ) {
// 	// TODO: have a lofi zone and switch visibility of the lofi and hifi entities

// 	let frustum: &Frustum = frustum.get_single().unwrap();
// 	for mut instance_data in instances.iter_mut() {
// 		let mut new_vis = Vec::with_capacity(instance_data.0.len());

// 		//HOT!
// 		for instance in instance_data.0.iter() {
// 			let mut vis = true;
// 			for plane in &frustum.planes {
// 				if plane.normal_d().dot(instance.position.extend(1.0)) //+ sphere.radius
// 				 <= 0.0 {
// 					vis = false;
// 					break;
// 				}
// 			}

// 			new_vis.push(vis);
// 		}
// 		instance_data.1 = new_vis;
// 	}

// 	// let transform: &Transform = player_query.get_single().unwrap();
// 	// let x = transform.translation.x;
// 	// let y = transform.translation.y;

// 	// let user_y = y.signum();

// 	// // If nothing's visible because we're far away make a few things visible so you know which
// 	// dir // to go in and can double click to get there...
// 	// #[cfg(feature = "adaptive-fps")]
// 	// if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
// 	// 	let min = diag.history_len();
// 	// 	if let Some(avg) = diag.values().map(|&i| i as u32).min() {
// 	// 		// println!("avg {}\t{}", avg, visible_width.0);
// 	// 		let target = 30.;
// 	// 		let avg = avg as f32;
// 	// 		if avg < target && visible_width.0 > 100. {
// 	// 			visible_width.0 -= (target - avg) / 4.;
// 	// 		}
// 	// 		// Because of frame rate differences it will go up much faster than it will go down!
// 	// 		else if avg > target && visible_width.0 < 1000. {
// 	// 			visible_width.0 += (avg - target) / 32.;
// 	// 		}
// 	// 	}
// 	// }

// 	// let width = visible_width.0;
// 	// let (min, max) = (x - width, x + width);

// 	// let mut vis_count = 0;
// 	// for (mut vis, transform, _, _, _) in entity_low_midfi.iter_mut() {
// 	// 	let loc = transform.translation();
// 	// 	vis.is_visible = min < loc.x && loc.x < max && loc.y.signum() == user_y;
// 	// 	if vis.is_visible {
// 	// 		vis_count += 1;
// 	// 	}
// 	// }
// 	// for (mut vis, transform, _, _) in entity_hifi.iter_mut() {
// 	// 	let loc = transform.translation();
// 	// 	vis.is_visible = min < loc.x && loc.x < max && loc.y.signum() == user_y;
// 	// 	if y > 500. {
// 	// 		vis.is_visible = false;
// 	// 	}
// 	// }
// 	// for (mut vis, transform, _, _) in entity_medfi.iter_mut() {
// 	// 	let loc = transform.translation();
// 	// 	vis.is_visible = min < loc.x && loc.x < max && loc.y.signum() == user_y;
// 	// 	if y > 800. {
// 	// 		vis.is_visible = false;
// 	// 	}
// 	// }

// 	// if vis_count == 0 {
// 	// 	for (mut vis, _, _, _, _) in entity_low_midfi.iter_mut().take(1000) {
// 	// 		vis.is_visible = true;
// 	// 	}
// 	// }

// 	// println!("viewport x = {},    {}  of   {} ", x, count_vis, count);
// }

// pub fn right_click_system(
// 	mouse_button_input: Res<Input<MouseButton>>,
// 	touches_input: Res<Touches>,
// 	// hover_query: Query<
// 	//     (Entity, &Hover, ChangeTrackers<Hover>),
// 	//     (Changed<Hover>, With<PickableMesh>),
// 	// >,
// 	// selection_query: Query<
// 	//     (Entity, &Selection, ChangeTrackers<Selection>),
// 	//     (Changed<Selection>, With<PickableMesh>),
// 	// >,
// 	// _query_details: Query<&Details>,
// 	click_query: Query<(Entity, &Hover)>,
// ) {
// 	if mouse_button_input.just_pressed(MouseButton::Right) ||
// 		touches_input.iter_just_pressed().next().is_some()
// 	{
// 		for (_entity, hover) in click_query.iter() {
// 			if hover.hovered() {
// 				// Open browser.
// 				// #[cfg(not(target_arch = "wasm32"))]
// 				// let details = query_details.get(entity).unwrap();
// 				// #[cfg(not(target_arch = "wasm32"))]
// 				// open::that(&details.url).unwrap();
// 				// picking_events.send(PickingEvent::Clicked(entity));
// 			}
// 		}
// 	}
// }

// 	// Kick off the live mode automatically so people have something to look at
// 	datasource_events.send(DataSourceChangedEvent {
// 		//source: "dotsama:/1//10504599".to_string(),
// 		// source: "local:live".to_string(),
// 		source: "dotsama:live".to_string(),
// 		timestamp: None,
// 	});
// }

#[derive(Default)]
pub struct Inspector {
	// #[inspectable(deletable = false)]
	// #[inspectable(collapse)]
	// start_location: UrlBar,
	// timestamp: DateTime,
	// #[inspectable(deletable = false)]
	// selected: Option<Details>,
	hovered: Option<String>,
	// texture: Option<egui::TextureHandle>,
}

// struct DateTime(NaiveDateTime, bool);

// impl DateTime {
// 	fn timestamp(&self) -> Option<i64> {
// 		if self.1 {
// 			Some(self.0.timestamp() as i64 * 1000)
// 		} else {
// 			None
// 		}
// 	}
// }

// impl Default for DateTime {
// 	fn default() -> Self {
// 		Self(chrono::offset::Utc::now().naive_utc(), false)
// 	}
// }

// impl Inspectable for DateTime {
// 	type Attributes = ();

// 	fn ui(
// 		&mut self,
// 		ui: &mut Ui,
// 		_: <Self as Inspectable>::Attributes,
// 		_: &mut bevy_inspector_egui::Context<'_>,
// 	) -> bool {
// 		// let mut changed = false;
// 		ui.checkbox(&mut self.1, "Point in time:");
// 		ui.add(
// 			DatePicker::<std::ops::Range<NaiveDateTime>>::new("noweekendhighlight", &mut self.0)
// 				.highlight_weekend(true),
// 		);
// 		true
// 		//        true // todo inefficient?
// 	}
// }

// #[derive(Component)]
pub struct Viewport;

#[cfg(target_arch = "wasm32")]
pub mod html_body {
	use web_sys::HtmlElement;
	// use web_sys::Document;

	pub fn get() -> HtmlElement {
		// From https://www.webassemblyman.com/rustwasm/how_to_add_mouse_events_in_rust_webassembly.html
		let window = web_sys::window().expect("no global `window` exists");
		let document = window.document().expect("should have a document on window");

		document.body().expect("document should have a body")
	}

	// Browser provides esc as an escape anyhow.
	// pub fn document() -> Document {
	//     // From https://www.webassemblyman.com/rustwasm/how_to_add_mouse_events_in_rust_webassembly.html
	//     let window = web_sys::window().expect("no global `window` exists");
	//     window.document().expect("should have a document on window")
	// }
}

#[derive(Deserialize, Serialize)]
pub enum BridgeMessage {
	SetDatasource(Sovereigns, Option<DotUrl>, u32), //data epoc
	GetNewBlocks,
	GetExtrinsicDetails(u32),
	GetEventDetails(u32),
}
