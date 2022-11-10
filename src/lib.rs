#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(slice_pattern)]
#![feature(slice_group_by)]
#![feature(option_get_or_insert_default)]
#![feature(async_closure)]
#![feature(stmt_expr_attributes)]
#![feature(let_chains)]

#[cfg(target_arch = "wasm32")]
use {
	core::future::Future, gloo_worker::Spawnable, gloo_worker::WorkerBridge, wasm_bindgen::JsCast,
	winit::platform::web::WindowBuilderExtWebSys,
};
use winit::dpi::PhysicalPosition;
use crate::{
	camera::CameraUniform,
	movement::Destination,
	ui::{ui_bars_system, Details, DotUrl, UrlBar},
};
use ::egui::FontDefinitions;
use chrono::prelude::*;
use datasource::DataUpdate;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use lazy_static::lazy_static;
use primitive_types::H256;
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	convert::AsRef,
	f32::consts::PI,
	iter,
	num::NonZeroU32,
	sync::{
		atomic::{AtomicI32, AtomicU32, Ordering},
		Arc, Mutex,
	},
	time::Duration,
};
use webworker::WorkerResponse;
use wgpu::{util::DeviceExt, TextureFormat};
use winit::{
	dpi::PhysicalSize,
	event::{WindowEvent, *},
	event_loop::EventLoop,
	window::Window,
};

// Define macros before mods
macro_rules! log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

mod camera;
mod content;
mod datasource;
mod movement;
mod style;
mod texture;
mod ui;

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

// The time by which all times should be placed relative to each other on the x axis.
lazy_static! {
	static ref BASETIME: Arc<Mutex<i64>> = Arc::new(Mutex::new(0_i64));
}

lazy_static! {
	static ref UPDATE_QUEUE: Arc<std::sync::Mutex<RenderUpdate>> =
		Arc::new(std::sync::Mutex::new(RenderUpdate::default()));
}

lazy_static! {
	static ref SELECTED: Arc<std::sync::Mutex<Option<(u32, Details)>>> =
		Arc::new(std::sync::Mutex::new(None));
}

lazy_static! {
	static ref DETAILS: Arc<std::sync::Mutex<RenderDetails>> =
		Arc::new(std::sync::Mutex::new(RenderDetails::default()));
}

lazy_static! {
	static ref REQUESTS: Arc<std::sync::Mutex<Vec<BridgeMessage>>> =
		Arc::new(std::sync::Mutex::new(Vec::default()));
}

/// Bump this to tell the current datasources to stop.
static DATASOURCE_EPOC: AtomicU32 = AtomicU32::new(0);

/// if you need bestest fps...
static PAUSE_DATA_FETCH: AtomicU32 = AtomicU32::new(0);

/// Immutable once set up.
#[derive(Clone, Serialize, Deserialize)]
pub struct ChainInfo {
	pub chain_ws: String,
	// Negative is other direction from center.
	pub chain_index: isize,
	pub chain_url: DotUrl,
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

use crate::camera::CameraController;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
	position: [f32; 3],
	color: [f32; 3],
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
			],
		}
	}
}

/// https://www.researchgate.net/profile/John-Sheridan-7/publication/253573419/figure/fig1/AS:298229276135426@1448114808488/A-volume-is-subdivided-into-cubes-The-vertices-are-numbered-0-7.png

fn rectangle(z_width: f32, y_height: f32, x_depth: f32, r: f32, g: f32, b: f32) -> [Vertex; 8] {
	let col = |bump: f32| -> [f32; 3] {
		[(r + bump).clamp(0., 2.), (g + bump).clamp(0., 2.), (b + bump).clamp(0., 2.)]
	};
	let bump = 0.10;
	[
		Vertex { position: [0.0, y_height, 0.0], color: col(bump) }, // C
		Vertex { position: [0.0, y_height, z_width], color: col(bump) }, // D
		Vertex { position: [0., 0., z_width], color: col(-bump) },   // B
		Vertex { position: [0., 0.0, 0.0], color: col(-bump) },      // A
		Vertex { position: [x_depth, y_height, 0.0], color: col(bump) }, // C
		Vertex { position: [x_depth, y_height, z_width], color: col(bump) }, // D
		Vertex { position: [x_depth, 0., z_width], color: col(bump * 2.0) }, // B
		Vertex { position: [x_depth, 0.0, 0.0], color: col(bump * 2.0) }, // A
	]
}

/*
		1,1,0    1,1,1       6 7

		1,0,0  1,0,1//MIN    4  5

0,1,0    0,1,1               2  3

0,0,0  0,0,1//MIN            0  1

*/

/// Counter clockwise to show up as looking from outside at cube.
// const INDICES: &[u16] = &cube_indicies(0);

const fn cube_indicies(offset: u16) -> [u16; 36] {
	[
		//TOP
		// 6,5,4,
		// 4,7,6,
		offset + 6,
		offset + 7,
		offset + 4, //TODO only need external faces
		offset + 4,
		offset + 5,
		offset + 6, // // //BOTTOM
		// 0,1,2,
		// 2,3,0,
		offset + 0,
		offset + 3,
		offset + 2,
		offset + 2,
		offset + 1,
		offset + 0, //right
		// 5,6,2,
		// 2,1,5,
		offset + 5,
		offset + 1,
		offset + 2,
		offset + 2,
		offset + 6,
		offset + 5, // //left
		// 7,4,0,
		// 0,3,7,
		offset + 7,
		offset + 3,
		offset + 0,
		offset + 0,
		offset + 4,
		offset + 7, // //front
		// 7,3,2,
		// 2,6,7,
		offset + 7,
		offset + 6,
		offset + 2,
		offset + 2,
		offset + 3,
		offset + 7,
		//back
		offset + 4,
		offset + 0,
		offset + 1,
		offset + 1,
		offset + 5,
		offset + 4,
		// 4,5,1,
		// 1,0,4,
	]
}

// struct Instance {
//     position: cgmath::Vector3<f32>,
//     //color: u32,
//     //flags: u32,
// }
// impl Instance {
//     fn to_raw(&self) -> InstanceRaw {
//         InstanceRaw {
//             model: self.position.into()
//         }
//     }
// }

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Deserialize, Serialize, Debug)]
struct Instance {
	position: [f32; 3],
	color: u32,
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
					shader_location: 5,
					format: wgpu::VertexFormat::Float32x3,
				},
				// A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a
				// slot for each vec4. We'll have to reassemble the mat4 in
				// the shader.
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
					shader_location: 6,
					format: wgpu::VertexFormat::Uint32,
				},
				// wgpu::VertexAttribute {
				//     offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
				//     shader_location: 7,
				//     format: wgpu::VertexFormat::Float32x4,
				// },
				// wgpu::VertexAttribute {
				//     offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
				//     shader_location: 8,
				//     format: wgpu::VertexFormat::Float32x4,
				// },
			],
		}
	}
}

async fn async_main() -> std::result::Result<(), ()> {
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

	// let mut app = App::new();
	// app
	// app.insert_resource(Msaa { samples: 4 });

	// #[cfg(target_family = "wasm")]
	// app.insert_resource(WindowDescriptor {
	// 	canvas: Some("canvas".into()), // CSS selector of the first canvas on the page.
	// 	..default()
	// });

	// The web asset plugin must be inserted before the `AssetPlugin` so
	// that the asset plugin doesn't create another instance of an asset
	// server. In general, the AssetPlugin should still run so that other
	// aspects of the asset system are initialized correctly.
	//app.add_plugin(bevy_web_asset::WebAssetPlugin);

	// #[cfg(target_arch = "wasm32")]
	// app.add_plugins_with(DefaultPlugins, |group| {
	// 	// The web asset plugin must be inserted in-between the
	// 	// `CorePlugin' and `AssetPlugin`. It needs to be after the
	// 	// CorePlugin, so that the IO task pool has already been constructed.
	// 	// And it must be before the `AssetPlugin` so that the asset plugin
	// 	// doesn't create another instance of an assert server. In general,
	// 	// the AssetPlugin should still run so that other aspects of the
	// 	// asset system are initialized correctly.
	// 	group.add_before::<bevy::asset::AssetPlugin, _>(bevy_web_asset::WebAssetPlugin)
	// });
	// #[cfg(not(target_arch = "wasm32"))]
	// app.add_plugins(DefaultPlugins);

	// CustomMaterialPlugin needs the shader handle set up:
	// load_internal_asset!(
	// 	app,
	// 	SHADER_HANDLE,
	// 	"../assets/shaders/instancing.wgsl",
	// 	Shader::from_wgsl
	// );
	// app.add_plugin(CustomMaterialPlugin);

	// Plugins related to instance rendering...
	// app.add_plugin(IndirectRenderingPlugin);
	// app.add_plugin(BasicMaterialPlugin)
	//       .add_plugin(CustomMaterialPlugin)
	//       .add_plugin(TextureMaterialPlugin);

	//  .insert_resource(WinitSettings::desktop_app()) - this messes up the 3d space mouse?
	// app.add_event::<DataSourceChangedEvent>();
	// app.add_event::<DataSourceStreamEvent>();
	// app.insert_resource(MovementSettings {
	// 	sensitivity: 0.00020, // default: 0.00012
	// 	speed: 12.0,          // default: 12.0
	// 	boost: 5.,
	// });

	// app.insert_resource(Sovereigns { relays: vec![], default_track_speed: 1. })

	// #[cfg(target_family = "wasm")]
	// app.add_plugin(bevy_web_fullscreen::FullViewportPlugin);

	// #[cfg(feature = "normalmouse")]
	// app.add_plugin(NoCameraPlayerPlugin);
	// app.insert_resource(movement::MouseCapture::default());
	// app.insert_resource(Anchor::default());
	// #[cfg(not(target_family = "wasm"))]
	// app.insert_resource(Width(750.));
	// #[cfg(target_family = "wasm")]
	// app.insert_resource(Width(500.));
	// app.insert_resource(Inspector::default());

	// #[cfg(feature = "spacemouse")]
	// app.add_plugin(SpaceMousePlugin);

	// // Continuous rendering for games - bevy's default.
	// // app.insert_resource(WinitSettings::game())
	// // Power-saving reactive rendering for applications.
	// if low_power_mode {
	// 	app.insert_resource(WinitSettings::desktop_app());
	// }
	// You can also customize update behavior with the fields of [`WinitConfig`]
	// .insert_resource(WinitSettings {
	//     focused_mode: bevy::winit::UpdateMode::ReactiveLowPower { max_wait:
	// Duration::from_millis(20), },     unfocused_mode: bevy::winit::UpdateMode::ReactiveLowPower {
	//         max_wait: Duration::from_millis(20),
	//     },
	//     ..default()
	// })
	// Turn off vsync to maximize CPU/GPU usage
	// .insert_resource(WindowDescriptor {
	//     present_mode: PresentMode::Immediate,
	//     ..default()
	// });
	// app.add_plugins(HighlightablePickingPlugins);

	// app.add_plugin(PickingPlugin)
	// .insert_resource(camera_rig)
	// .insert_resource(movement::Destination::default());
	// app.add_system(ui::ui_bars_system);
	// .add_plugin(recorder::RecorderPlugin)
	// .add_system(movement::rig_system)
	// app.add_plugin(InteractablePickingPlugin);
	// .add_plugin(HighlightablePickingPlugin);
	// .add_plugin(DebugCursorPickingPlugin) // <- Adds the green debug cursor.
	// .add_plugin(InspectorPlugin::<Inspector>::new())
	// .register_inspectable::<Details>()
	// .add_plugin(DebugEventsPickingPlugin)
	// app.add_plugin(PolylinePlugin);
	// app.add_plugin(EguiPlugin);
	// app.insert_resource(ui::OccupiedScreenSpace::default());

	// app.add_startup_system(setup);
	// app.add_startup_system(load_assets_initial);
	// #[cfg(feature = "spacemouse")]
	// app.add_startup_system(move |mut scale: ResMut<bevy_spacemouse::Scale>| {
	// 	scale.rotate_scale = 0.00010;
	// 	scale.translate_scale = 0.004;
	// });
	// app.add_system(movement::player_move_arrows)
	// .add_system(rain)
	// .add_system(source_data)

	// // .add_system(pad_system)
	// // .add_plugin(LogDiagnosticsPlugin::default())
	// app.add_plugin(FrameTimeDiagnosticsPlugin::default());
	// // .add_system(ui::update_camera_transform_system)
	// app.add_system(right_click_system);
	// app.add_system_to_stage(CoreStage::PostUpdate, update_visibility);
	// app.add_startup_system(ui::details::configure_visuals);

	// #[cfg(feature = "atmosphere")]
	// app.insert_resource(Atmosphere::default()); // Default Earth sky

	// #[cfg(feature = "atmosphere")]
	// app.add_plugin(AtmospherePlugin::default());
	//  {
	// 	// dynamic: false, // Set to false since we aren't changing the sky's appearance
	// 	sky_radius: 1000.0,
	// }

	// app.add_system(capture_mouse_on_click);
	//  app.add_system(get_mouse_movement )
	//     .init_resource::<WasmMouseTracker>();

	// app.add_system(render_block);
	// app.add_system_to_stage(CoreStage::PostUpdate, print_events);

	// #[cfg(target_arch = "wasm32")]
	// html_body::get().request_pointer_lock();

	//	app.run();

	let event_loop = winit::event_loop::EventLoopBuilder::<()>::with_user_event().build();

	let mut winit_window_builder = winit::window::WindowBuilder::new();

	#[cfg(target_family = "wasm")]
	{
		let window = web_sys::window().unwrap();
		let document = window.document().unwrap();
		let canvas = document.query_selector(&"canvas").expect("Cannot query for canvas element.");
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
	wasm_bindgen_futures::spawn_local(run(event_loop, window));
	#[cfg(not(target_family = "wasm"))]
	run(event_loop, window).await;

	log!("event loop finished");
	Ok::<(), ()>(())
}

async fn run(event_loop: EventLoop<()>, window: Window) {
	// let movement_settings = MovementSettings {
	// 	sensitivity: 0.00020, // default: 0.00012
	// 	speed: 12.0,          // default: 12.0
	// 	boost: 5.,
	// };
	let ground_width = 1000000.0f32;
	let touch_sensitivity = 4.0f64;

	let mut urlbar =
		ui::UrlBar::new("dotsama:/1//10504599".to_string(), Utc::now().naive_utc(), Env::Local);
	// app.insert_resource();
	let sovereigns = Sovereigns { relays: vec![], default_track_speed: 1. };

	// let mouse_capture = movement::MouseCapture::default();
	let mut anchor = Anchor::default();
	let mut destination = movement::Destination::default();
	let mut inspector = Inspector::default();
	let mut occupied_screen_space = ui::OccupiedScreenSpace::default();

	ui::details::configure_visuals();

	let instance = wgpu::Instance::new(wgpu::Backends::all());
	// SAFETY: `window` Handle must be a valid object to create a surface upon
	// and must remain valid for the lifetime of the returned surface.
	let mut surface = unsafe { instance.create_surface(&window) };

	let adapter = instance
		.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::default(),
			compatible_surface: Some(&surface),
			force_fallback_adapter: false,
		})
		.await
		.unwrap();

	//TODO: can we await instead of block_on here?
	let (device, queue) = pollster::block_on(adapter.request_device(
		&wgpu::DeviceDescriptor {
			features: wgpu::Features::default(),
			limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
			label: None,
		},
		None,
	))
	.unwrap();

	let mut size = window.inner_size();
	let hidpi_factor = window.scale_factor(); // 2.0 <-- this is why quaters!
	log!("hidpi factor {:?}", hidpi_factor);

	// size.width *= hidpi_factor as u32;//todo!
	// size.height *= hidpi_factor as u32;

	log!("Initial size: width:{} height:{}", size.width as u32, size.height as u32);
	// size.width = 1024; - seems double this so 4x pixels
	// size.height = 768;

	// let channel = std::sync::mpsc::channel();
	// let resize_sender: OnResizeSender = channel.0;
	// let resize_receiver = Mutex::new(channel.1);
	// setup_viewport_resize_system(Mutex::new(resize_sender));

	let surface_format = surface.get_supported_formats(&adapter)[0];
	let mut surface_config = wgpu::SurfaceConfiguration {
		usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
		format: surface_format,
		width: size.width as u32,
		height: size.height as u32,
		present_mode: wgpu::PresentMode::Fifo, //Immediate not supported on web
	};
	surface.configure(&device, &surface_config);

	assert!(size.width > 0);
	// We use the egui_winit_platform crate as the platform.
	let mut platform = Platform::new(PlatformDescriptor {
		physical_width: size.width as u32,
		physical_height: size.height as u32,
		scale_factor: window.scale_factor(),
		font_definitions: FontDefinitions::default(),
		style: Default::default(),
	});

	// We use the egui_wgpu_backend crate as the render backend.
	let mut egui_rpass = RenderPass::new(&device, surface_format, 1);

	// Display the application

	let mut frame_time = Utc::now().timestamp();
	let frames = 0;
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
	let mut camera = camera::Camera::new((0.0, 100.0, 10.0), cgmath::Deg(0.0), cgmath::Deg(-20.0));
	let mut projection =
		camera::Projection::new(size.width, size.height, cgmath::Deg(45.0), 0.1, 4000.0);
	let mut camera_controller = camera::CameraController::new(4.0, 0.4);

	let mut camera_uniform = CameraUniform::new();
	camera_uniform.update_view_proj(&camera, &projection);

	let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: Some("Camera Buffer"),
		contents: bytemuck::cast_slice(&[camera_uniform]),
		usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
	});

	let camera_bind_group_layout =
		device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::VERTEX,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: None,
				},
				count: None,
			}],
			label: Some("camera_bind_group_layout"),
		});

	let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout: &camera_bind_group_layout,
		entries: &[wgpu::BindGroupEntry {
			binding: 0,
			resource: camera_buffer.as_entire_binding(),
		}],
		label: Some("camera_bind_group"),
	});

	let mut depth_texture =
		texture::Texture::create_depth_texture(&device, &surface_config, "depth_texture");

	let mut vertices = vec![]; //cube
	vertices.extend(rectangle(CUBE_WIDTH, CUBE_WIDTH, CUBE_WIDTH, 0., 0., 0.));
	let offset1 = vertices.len(); //block
	vertices.extend(rectangle(10., 0.5, 10., 0., 0.0, 0.));
	let offset2 = vertices.len(); //chain
	vertices.extend(rectangle(10., CHAIN_HEIGHT, 100000., 0.0, 0.0, 0.));
	let offset3 = vertices.len(); //ground
	vertices.extend(rectangle(ground_width, 10., ground_width, 0.0, 0.0, 0.));
	let offset4 = vertices.len(); //selected
	vertices.extend(rectangle(CUBE_WIDTH + 0.2, CUBE_WIDTH + 0.2, CUBE_WIDTH + 0.2, 0., 0., 0.));

	// vertices.extend(rectangle(ground_width, 0.00001, ground_width, 0.0, 0.0, 0.));

	let mut indicies: Vec<u16> = vec![];
	indicies.extend(&cube_indicies(0));
	indicies.extend(&cube_indicies(offset1 as u16));
	indicies.extend(&cube_indicies(offset2 as u16));
	indicies.extend(&cube_indicies(offset3 as u16));
	indicies.extend(&cube_indicies(offset4 as u16));

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

	let ground_instance_data: Vec<Instance> = vec![
	// Instance{ position: [-ground_width/2.0,-100.,-ground_width/2.0], color: as_rgba_u32(-1.0, -1.0, -1.0, 1.0) },
	// Instance{ position: [-ground_width/2.0,1000.,-ground_width/2.0], color: 344411 }

	];
	let mut chain_instance_data = vec![];
	let mut block_instance_data = vec![];
	let mut cube_instance_data = vec![];
	let mut selected_instance_data = vec![];
	let mut cube_target_heights: Vec<f32> = vec![];
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
				blend: Some(wgpu::BlendState::REPLACE),
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
			stencil: wgpu::StencilState::default(),
			bias: wgpu::DepthBiasState::default(),
		}),
		multisample: wgpu::MultisampleState {
			count: 1,
			mask: !0,
			alpha_to_coverage_enabled: false,
		},
		multiview: None,
	});

	let mut last_render_time = Utc::now();

	let mut frames = 0u64;
	let mut fps = 0;

	let initial_event = DataSourceChangedEvent {
		//source: "dotsama:/1//10504599".to_string(),
		// source: "local:live".to_string(),
		source: "dotsama:live".to_string(),
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
	let mut old_width = 0u32;
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
	> = Default::default();

	let mut last_mouse_position = None;
	event_loop.run(move |event, _, _control_flow| {
		let now = Utc::now();

		let scale_x = size.width as f32 / hidpi_factor as f32;
		let scale_y = size.height as f32 / hidpi_factor as f32;

		// Pass the winit events to the platform integration.
		platform.handle_event(&event);

		frames += 1;

		let selected_details = SELECTED.lock().unwrap().clone();

		// viewport_resize_system(&resize_receiver);
		// if let Some(new_size) = viewport_resize_system(&resize_receiver) {
		// 	log!("set new size width: {} height: {}", new_size.width, new_size.height);
		// 	window.set_inner_size(new_size);
		// 	window.set_inner_size(LogicalSize::new(new_size.width, new_size.height));
		// 	projection.resize(new_size.width, new_size.height);
		// 	size = new_size;
		// 	surface.configure(&device, &surface_config);
		// 	depth_texture =
		// 		texture::Texture::create_depth_texture(&device, &surface_config, "depth_texture");
		// }

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
				redraw = input(&mut camera_controller, &event, &mut mouse_pressed);
				log!("event: {:?}", &event);
				if let WindowEvent::CursorMoved{ position, .. } = event {
					last_mouse_position = Some(position.clone());
				}

				// WindowEvent::TouchpadMagnify and WindowEvent::TouchpadRotate events are
				// only available on macos, so build up from touch events:

				if let WindowEvent::Touch(Touch{ location, phase, id, .. }) = event {
					let mut our_finger = None;
					let mut other_finger = None;
					for (other_id, (last_touch_location, last_time, previous)) in last_touch_location.iter() {
						if let Some((prev_loc, prev_time)) = previous {
							if (now - *last_time).num_milliseconds() < 500
							&& (now - *prev_time).num_milliseconds() < 1000 {
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
							let y_diff = loc2.y - loc2.y;
							(x_diff*x_diff + y_diff * y_diff).sqrt()
						};
						let cur_dist = dist(cur1, cur2);
						let prev_dist = dist(prev1, prev2);

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
					} else {
						log!("Touch!");
						// if *id == 0 {
						if let Some((last_touch_location, last_time, _prev)) = last_touch_location.get(id) {
							if let TouchPhase::Moved = phase {
								let x_diff = last_touch_location.x - location.x;
								let y_diff = last_touch_location.y - location.y;
								if x_diff.abs() + y_diff.abs() < 200. {
									camera_controller.rotate_horizontal -= (x_diff / touch_sensitivity) as f32;
									camera_controller.rotate_vertical += (y_diff / touch_sensitivity) as f32;
								}
							}
						}
					}

					let val = last_touch_location.entry(*id).or_insert((location.clone(), now, None));
					*val = (location.clone(), now, Some((val.0, val.1)));
				}

				if let WindowEvent::MouseInput { button: winit::event::MouseButton::Left, state, .. } = event {
					if let Some(position) = last_mouse_position {
						let matrix = camera.calc_matrix();
						let x: glam::Vec4 = glam::Vec4::new(matrix.x.x, matrix.x.y, matrix.x.z, matrix.x.w);
						let y: glam::Vec4 = glam::Vec4::new(matrix.y.x, matrix.y.y, matrix.y.z, matrix.y.w);
						let z: glam::Vec4 = glam::Vec4::new(matrix.z.x, matrix.z.y, matrix.z.z, matrix.z.w);
						let w: glam::Vec4 = glam::Vec4::new(matrix.w.x, matrix.w.y, matrix.w.z, matrix.w.w);
						let view = glam::Mat4::from_cols(x, y, z, w);

						let matrix = cgmath::perspective(
							projection.fovy,
							projection.aspect,
							projection.znear,
							projection.zfar,
						);
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
						let clicked2 = glam::Vec2::new( clicked1.x - size.width as f32 / 2.0,
							size.height as f32 / 2.0 -  clicked1.y);
						// log!("new add: {:?}", clicked);
						let clicked = glam::Vec2::new(clicked2.x / scale_x,
							clicked2.y/scale_y);
						// log!("new adj: {:?}  {:?}  {:?}", clicked1, clicked2, clicked);

						let near_clicked = ndc_to_world.project_point3(clicked.extend(near_ndc));
						let far_clicked = ndc_to_world.project_point3(clicked.extend(far_ndc));
						let ray_direction_clicked = near_clicked - far_clicked;
						let pos_clicked: glam::Vec3 = near_clicked.into();

						let selected = get_selected(pos_clicked, ray_direction_clicked, &mut cube_instance_data,
							glam::Vec3::new(CUBE_WIDTH, CUBE_WIDTH, CUBE_WIDTH)
						);
						log!("selected = {:?}", selected);
						if let Some((index, instance)) = selected {
							// ground_instance_data.push(Instance { position: near_clicked.into(), color: as_rgba_u32(0.3, 0.3, 0.3, 1.) });
							let mut pos = instance.position.clone();
							pos[0] += -0.1;
							pos[1] += -0.1;
							pos[2] += -0.1;
							selected_instance_data.clear();
							selected_instance_data.push(Instance { position: pos, color: as_rgba_u32(0.1, 0.1, 0.9, 0.7) });

							(*REQUESTS.lock().unwrap()).push(BridgeMessage::GetDetails(index));
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
					resize(&size, &device, &mut surface_config, &mut projection, &mut surface, &mut depth_texture);
				} else if let WindowEvent::ScaleFactorChanged { new_inner_size, .. } = event {
					size = **new_inner_size;
					resize(&size, &device, &mut surface_config, &mut projection, &mut surface, &mut depth_texture);
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
			// let mut data_update: Option<DataUpdate> = None;
			if let Ok(render_update) = &mut UPDATE_QUEUE.lock() {
				for (instance, height) in &(**render_update).cube_instances {
					cube_instance_data.push(instance.clone());
					cube_target_heights.push(*height);
				}
				//TODO: drain not clone!
				block_instance_data.extend((**render_update).block_instances.clone());
				chain_instance_data.extend((**render_update).chain_instances.clone());

				//todo: possibly not needed?
				render_update.chain_instances.truncate(0);
				render_update.block_instances.truncate(0);
				render_update.cube_instances.truncate(0);
			}

			rain(&mut cube_instance_data, &mut cube_target_heights);

			// TODO don't create each time!!!
			let ground_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Instance Buffer"),
					contents: bytemuck::cast_slice(&ground_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});
			let chain_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Instance Buffer"),
					contents: bytemuck::cast_slice(&chain_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});
			let block_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Instance Buffer"),
					contents: bytemuck::cast_slice(&block_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});
			let cube_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Instance Buffer"),
					contents: bytemuck::cast_slice(&cube_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});
			let selected_instance_buffer =
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Instance Buffer"),
					contents: bytemuck::cast_slice(&selected_instance_data),
					usage: wgpu::BufferUsages::VERTEX,
				});

			let output = surface.get_current_texture().unwrap();
			let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

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
				scale_factor: window.scale_factor() as f32,
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
							load: wgpu::LoadOp::Clear(1.0),
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

				let side = (occupied_screen_space.left * hidpi_factor as f32) as u32;

				// log!("size {:?}", size);
				let (x, y) = (0 + side, 50);
				let (width, height) =
					(size.width /*+ 600*/ - (side), (size.height - y) - 50 /* + 400 */);

				//if old_width != width as u32 {
				// log!(
				// 	"set scissor rect: x: {} y: {}, width: {} height: {}, was {}",
				// 	x,
				// 	y,
				// 	width,
				// 	height,
				// 	old_width
				// );
				old_width = width as u32;
				//}

				render_pass.set_scissor_rect(x as u32, y as u32, width, height);

				// render_pass.set_viewport(x as f32,y as f32,width as f32, height as f32, 0., 1.);
				render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

				// Draw ground
				render_pass.set_vertex_buffer(1, ground_instance_buffer.slice(..));
				render_pass.draw_indexed(
					(36 + 36 + 36)..((36 + 36 + 36 + 36) as u32),
					0,
					0..ground_instance_data.len() as _,
				);

				// Draw chains
				render_pass.set_vertex_buffer(1, chain_instance_buffer.slice(..));
				render_pass.draw_indexed(
					(36 + 36)..((36 + 36 + 36) as u32),
					0,
					0..chain_instance_data.len() as _,
				);

				// Draw blocks
				render_pass.set_vertex_buffer(1, block_instance_buffer.slice(..));
				render_pass.draw_indexed(
					(36)..((36 + 36) as u32),
					0,
					0..block_instance_data.len() as _,
				);

				// Draw cubes
				render_pass.set_vertex_buffer(1, cube_instance_buffer.slice(..));
				render_pass.draw_indexed((0)..((36) as u32), 0, 0..cube_instance_data.len() as _);

				render_pass.set_vertex_buffer(1, selected_instance_buffer.slice(..));
				render_pass.draw_indexed(
					(36 + 36 + 36 + 36)..((36 + 36 + 36 + 36 + 36) as u32),
					0,
					0..selected_instance_data.len() as _,
				);
			}
			queue.submit(std::iter::once(encoder.finish()));

			output_frame.present();

			frames += 1;
			if Utc::now().timestamp() - frame_time > 1 {
				fps = frames as u32;
				frames = 0;
				frame_time = Utc::now().timestamp();
			}

			egui_rpass.remove_textures(tdelta).expect("remove texture ok");
		}
	});
}

fn get_selected(
	r_org: glam::Vec3,
	mut r_dir: glam::Vec3,
	instances: &mut Vec<Instance>,
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
		let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

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
			best = Some((i as u32, instance.clone()));
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
) {
	surface_config.width = size.width;
	surface_config.height = size.height;
	projection.resize(size.width, size.height);
	//TODO: might need to update CameraUniform here
	surface.configure(&device, &surface_config);
	*depth_texture =
		texture::Texture::create_depth_texture(&device, &surface_config, "depth_texture");
}

fn input(
	camera_controller: &mut CameraController,
	event: &WindowEvent,
	mouse_pressed: &mut bool,
) -> bool {
	match event {
		WindowEvent::KeyboardInput {
			input: KeyboardInput { virtual_keycode: Some(key), state, .. },
			..
		} => camera_controller.process_keyboard(*key, *state),
		WindowEvent::MouseWheel { delta, .. } => {
			camera_controller.process_scroll(delta);
			true
		},
		WindowEvent::MouseInput { button: winit::event::MouseButton::Left, state, .. } => {
			*mouse_pressed = *state == ElementState::Pressed;
			true
		},
		WindowEvent::Resized(new_size) => {
			log!("Window event: new size: width {} height {}", new_size.width, new_size.height);
			true
		},
		_ => false,
	}
}

// struct DataSourceStreamEvent(ChainInfo, datasource::DataUpdate);

fn chain_name_to_url(chain_name: &str) -> String {
	let mut chain_name = chain_name.to_string();
	if !chain_name.starts_with("ws:") && !chain_name.starts_with("wss:") {
		chain_name = format!("wss://{}", chain_name);
	}

	if chain_name[5..].contains(':') {
		chain_name
	} else {
		format!("{chain_name}:443")
	}
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
async fn send_it_to_desktop(update: RenderUpdate) {
	// log!("Got some results....! yay they're already in the right place. {}", blocks.len());
	UPDATE_QUEUE.lock().unwrap().extend(update);
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

	// commands
	// 	.spawn()
	// 	.insert_bundle((
	// 		handles.extrinsic_mesh.clone(), //todo xcm different? block_mesh
	// 		InstanceMaterialData(vec![], vec![]),
	// 		// SpatialBundle::VISIBLE_IDENTITY, - later bevy can just do this rather than next
	// 		// lines
	// 		Transform::from_xyz(0., 0., 0.),
	// 		GlobalTransform::default(),
	// 		Visibility::default(),
	// 		ComputedVisibility::default(),
	// 		// NOTE: Frustum culling is done based on the Aabb of the Mesh and the
	// 		// GlobalTransform. As the cube is at the origin, if its Aabb moves outside the
	// 		// view frustum, all the instanced cubes will be culled.
	// 		// The InstanceMaterialData contains the 'GlobalTransform' information for this
	// 		// custom instancing, and that is not taken into account with the built-in frustum
	// 		// culling. We must disable the built-in frustum culling by adding the
	// 		// `NoFrustumCulling` marker component to avoid incorrect culling.
	// 		NoFrustumCulling,
	// 	))
	// 	//	.insert_bundle(PickableBundle::default())
	// 	.insert(Name::new("BlockEvent"))
	// 	.insert(ClearMe)
	// 	.insert(HiFi)
	// 	.insert(EventInstances);

	// commands
	// 	.spawn()
	// 	.insert_bundle((
	// 		handles.extrinsic_mesh.clone(), //todo xcm different? block_mesh
	// 		InstanceMaterialData(vec![], vec![]),
	// 		// SpatialBundle::VISIBLE_IDENTITY, - later bevy can just do this rather than next
	// 		// lines
	// 		Transform::from_xyz(0., 0., 0.),
	// 		GlobalTransform::default(),
	// 		Visibility::default(),
	// 		ComputedVisibility::default(),
	// 		// NOTE: Frustum culling is done based on the Aabb of the Mesh and the
	// 		// GlobalTransform. As the cube is at the origin, if its Aabb moves outside the
	// 		// view frustum, all the instanced cubes will be culled.
	// 		// The InstanceMaterialData contains the 'GlobalTransform' information for this
	// 		// custom instancing, and that is not taken into account with the built-in frustum
	// 		// culling. We must disable the built-in frustum culling by adding the
	// 		// `NoFrustumCulling` marker component to avoid incorrect culling.
	// 		NoFrustumCulling,
	// 	))
	// 	//			.insert_bundle(PickableBundle::default())
	// 	.insert(Name::new("BlockExtrinsic"))
	// 	.insert(ClearMe)
	// 	.insert(MedFi)
	// 	.insert(ExtrinsicInstances);

	// commands
	// 	.spawn()
	// 	.insert_bundle((
	// 		handles.block_mesh.clone(), //todo xcm different? block_mesh
	// 		InstanceMaterialData(vec![], vec![]),
	// 		// SpatialBundle::VISIBLE_IDENTITY, - later bevy can just do this rather than next
	// 		// lines
	// 		Transform::from_xyz(0., 0., 0.),
	// 		GlobalTransform::default(),
	// 		Visibility::default(),
	// 		ComputedVisibility::default(),
	// 		// NOTE: Frustum culling is done based on the Aabb of the Mesh and the
	// 		// GlobalTransform. As the cube is at the origin, if its Aabb moves outside the
	// 		// view frustum, all the instanced cubes will be culled.
	// 		// The InstanceMaterialData contains the 'GlobalTransform' information for this
	// 		// custom instancing, and that is not taken into account with the built-in frustum
	// 		// culling. We must disable the built-in frustum culling by adding the
	// 		// `NoFrustumCulling` marker component to avoid incorrect culling.
	// 		NoFrustumCulling,
	// 	))
	// 	.insert(Name::new("Block"))
	// 	.insert(ClearMe)
	// 	.insert(BlockInstances);

	// commands
	// 	.spawn()
	// 	.insert_bundle((
	// 		handles.chain_rect_mesh.clone(), //todo xcm different? block_mesh
	// 		InstanceMaterialData(vec![], vec![]),
	// 		// SpatialBundle::VISIBLE_IDENTITY, - later bevy can just do this rather than next
	// 		// lines
	// 		Transform::from_xyz(0., 0., 0.),
	// 		GlobalTransform::default(),
	// 		Visibility::default(),
	// 		ComputedVisibility::default(),
	// 		// NOTE: Frustum culling is done based on the Aabb of the Mesh and the
	// 		// GlobalTransform. As the cube is at the origin, if its Aabb moves outside the
	// 		// view frustum, all the instanced cubes will be culled.
	// 		// The InstanceMaterialData contains the 'GlobalTransform' information for this
	// 		// custom instancing, and that is not taken into account with the built-in frustum
	// 		// culling. We must disable the built-in frustum culling by adding the
	// 		// `NoFrustumCulling` marker component to avoid incorrect culling.
	// 		NoFrustumCulling,
	// 	))
	// 	//			.insert_bundle(PickableBundle::default())
	// 	.insert(Name::new("Chain"))
	// 	.insert(ClearMe)
	// 	// .insert(LoFi)
	// 	.insert(ChainInstances);

	if event.source.is_empty() {
		log!("Datasources cleared epoc {}", DATASOURCE_EPOC.load(Ordering::Relaxed));
		return
	}

	let dot_url = DotUrl::parse(&event.source);

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
		let env = event.source.split(":").collect::<Vec<_>>()[0].to_string();
		let env = Env::try_from(env.as_str()).unwrap();
		(DotUrl { env, ..Default::default() }, None)
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
				.map(|(chain_index, (para_id, chain_name))| {
					let url = chain_name_to_url(chain_name);

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
				WorkerResponse::RenderUpdate(update) => {
					let mut pending = UPDATE_QUEUE.lock().unwrap();
					pending.extend(update);
				},
				WorkerResponse::Details(index, details) => {
					*SELECTED.lock().unwrap() = Some((index, details));
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
			// log!("Sent bridge message");
			} else {
				bridge.send(BridgeMessage::GetNewBlocks);
			}
			async_std::task::sleep(Duration::from_millis(15)).await;
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
			NonZeroU32,
			async_std::channel::Sender<(datasource::RelayBlockNumber, i64, H256)>,
		> = Default::default();
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
	// relays: Vec<Vec<ChainInfo>>,
	as_of: Option<DotUrl>,
	callback: &'static F,
) where
	F: (Fn((RenderUpdate, RenderDetails)) -> R) + Send + Sync + 'static,
	R: Future<Output = ()> + 'static,
{
	for relay in sovereigns.relays.iter() {
		let mut relay2: Vec<(ChainInfo, _)> = vec![];
		let mut send_map: HashMap<
			NonZeroU32,
			async_std::channel::Sender<(datasource::RelayBlockNumber, i64, H256)>,
		> = Default::default();
		for chain in relay.iter() {
			let (tx, rc) = async_std::channel::unbounded();
			if let Some(para_id) = chain.chain_url.para_id {
				send_map.insert(para_id, tx);
			}
			relay2.push((chain.clone(), rc));
		}

		let mut send_map = Some(send_map);
		//let mut sov_relay = vec![];
		for (chain, rc) in relay2 {
			// log!("listening to {}", chain.info.chain_ws);

			let maybe_sender = if chain.chain_url.is_relay() { send_map.take() } else { None };

			// let lock_clone = chain.shared;
			let as_of = as_of.clone();
			// log!("as of for chain {:?} index {}", &as_of, chain.chain_index);
			let chain_info = chain.clone();

			let block_watcher = datasource::BlockWatcher {
				tx: Some(callback),
				chain_info,
				as_of,
				receive_channel: Some(rc),
				sender: maybe_sender,
				// source
			};

			//let block_watcher = Box::leak(Box::new(block_watcher));

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

fn clear_world(// details: &Query<Entity, With<ClearMeAlwaysVisible>>,
	// commands: &mut Commands,
	// clean_me: &Query<Entity, With<ClearMe>>,
) {
	// Stop previous data sources...
	DATASOURCE_EPOC.fetch_add(1, Ordering::Relaxed);
	log!("incremet epoc to {}", DATASOURCE_EPOC.load(Ordering::Relaxed));

	// for detail in details.iter() {
	// 	commands.entity(detail).despawn();
	// }
	// for detail in clean_me.iter() {
	// 	commands.entity(detail).despawn();
	// }
	*BASETIME.lock().unwrap() = 0;
}

#[derive(Clone, Copy)]

enum BuildDirection {
	Up,
	Down,
}

// fn format_entity(entity: &DataEntity) -> String {
// 	let res = match entity {
// 		DataEntity::Event(DataEvent { details, .. }) => {
// 			format!("{:#?}", details)
// 		},
// 		DataEntity::Extrinsic {
// 			// id: _,
// 			args,
// 			contains,
// 			details,
// 			..
// 		} => {
// 			let kids = if contains.is_empty() {
// 				String::new()
// 			} else {
// 				format!(" contains {} extrinsics", contains.len())
// 			};
// 			format!("{} {} {}\n{:#?}", details.pallet, details.variant, kids, args)
// 		},
// 	};

// 	// if let Some(pos) = res.find("data: Bytes(") {
// 	//     res.truncate(pos + "data: Bytes(".len());
// 	//     res.push_str("...");
// 	// }
// 	res
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
	// end_link: Vec<String>,
}

/// A tag to identify an entity as being the source of a message.
// #[derive(Component)]
pub struct MessageSource {
	/// Currently sending block id + hash of beneficiary address.
	pub id: String,
	pub link_type: LinkType,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum LinkType {
	Teleport,
	ReserveTransfer,
	ReserveTransferMintDerivative,
	ParaInclusion,
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
	cube_instances: Vec<(Instance, f32)>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct RenderDetails {
	chain_instances: Vec<Details>,
	block_instances: Vec<Details>,
	cube_instances: Vec<Details>,
}

impl RenderUpdate {
	fn extend(&mut self, update: RenderUpdate) {
		self.chain_instances.extend(update.chain_instances);
		self.block_instances.extend(update.block_instances);
		self.cube_instances.extend(update.cube_instances);
	}
}

impl RenderDetails {
	fn extend(&mut self, update: RenderDetails) {
		self.chain_instances.extend(update.chain_instances);
		self.block_instances.extend(update.block_instances);
		self.cube_instances.extend(update.cube_instances);
	}
}

fn render_block(
	data_update: DataUpdate,
	// mut commands: Commands,
	// mut materials: ResMut<Assets<StandardMaterial>>,
	chain_info: &ChainInfo,
	// asset_server: Res<AssetServer>,
	// links: Query<(Entity, &MessageSource, &GlobalTransform)>,
	// mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
	// mut polylines: ResMut<Assets<Polyline>>,
	// mut event: EventWriter<RequestRedraw>,
	// mut handles: ResMut<ResourceHandles>,
	render: &mut RenderUpdate,
	// mut event_instances: &mut Vec<(Instance, f32)>,
	// mut block_instances: &mut Vec<Instance>,
	// mut chain_instances: &mut Vec<Instance>,
	mut render_details: &mut RenderDetails,
) {
	// for mut extrinsic_instances in extrinsic_instances.iter_mut() {
	// 	for mut event_instances in event_instances.iter_mut() {
	// 		for mut block_instances in block_instances.iter_mut() {

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
				//log!("BASETIME set to {}", base_time);
				*BASETIME.lock().unwrap() = base_time;
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
			let encoded: String = form_urlencoded::Serializer::new(String::new())
				.append_pair("rpc", &chain_info.chain_ws)
				.finish();

			let is_relay = chain_info.chain_url.is_relay();
			let details = Details {
				doturl: DotUrl { extrinsic: None, event: None, ..block.blockurl.clone() },

				url: format!(
					"https://polkadot.js.org/apps/?{}#/explorer/query/{}",
					&encoded,
					block.blockurl.block_number.unwrap()
				),
				..Default::default()
			};
			// log!("rendering block from {}", details.doturl);

			// println!("block.timestamp {:?}", block.timestamp);
			// println!("base_time {:?}",base_time);
			let block_num = timestamp_to_x(block.timestamp.unwrap_or(base_time));

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
						0. + (block_num as f32) - 5.,
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
				datasource::associate_events(block.extrinsics.clone(), block.events.clone());

			// Leave infrastructure events underground and show user activity above
			// ground.
			let (boring, fun): (Vec<_>, Vec<_>) =
				ext_with_events.into_iter().partition(|(e, _)| {
					if let Some(ext) = e {
						content::is_utility_extrinsic(ext)
					} else {
						true
					}
				});

			add_blocks(
				chain_info,
				block_num,
				fun,
				// &mut commands,
				// &mut materials,
				BuildDirection::Up,
				// &links,
				// &mut polyline_materials,
				// &mut polylines,
				&encoded,
				// &mut handles,
				&mut render.cube_instances,
				&mut render_details, // &mut event_dest, // &mut event_instances,
			);

			add_blocks(
				chain_info,
				block_num,
				boring,
				// &mut commands,
				// &mut materials,
				BuildDirection::Down,
				// &links,
				// &mut polyline_materials,
				// &mut polylines,
				&encoded,
				// &mut handles,
				&mut render.cube_instances,
				&mut render_details, // &mut event_dest, // &mut event_instances,
			);
			//event.send(RequestRedraw);
		},
		DataUpdate::NewChain(chain_info) => {
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
	block_events: Vec<(Option<DataEntity>, Vec<DataEvent>)>,
	build_direction: BuildDirection,
	// links: &Query<(Entity, &MessageSource, &GlobalTransform)>,
	// polyline_materials: &mut ResMut<Assets<PolylineMaterial>>,
	// polylines: &mut ResMut<Assets<Polyline>>,
	encoded: &str,
	extrinsic_instances: &mut Vec<(Instance, f32)>,
	render_details: &mut RenderDetails,
) {
	let rflip = chain_info.chain_url.rflip();
	let build_dir = if let BuildDirection::Up = build_direction { 1.0 } else { -1.0 };
	// Add all the useful blocks

	let layer = chain_info.chain_url.layer() as f32;
	let (base_x, base_y, base_z) = (
		(block_num) - 4.,
		LAYER_GAP * layer,
		RELAY_CHAIN_CHASM_WIDTH + BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32 - 4.,
	);

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
				// for (link, _link_type) in block.end_link() {
				// 	//if this id already exists then this is the destination, not the source...
				// 	for (entity, id, source_global) in links.iter() {
				// 		if id.id == *link {
				// 			// println!("creating rainbow!");

				// 			let mut vertices = vec![
				// 				source_global.translation(),
				// 				Vec3::new(px, base_y + target_y * build_dir, pz * rflip),
				// 			];
				// 			rainbow(&mut vertices, 50);

				// 			let colors = vec![
				// 				Color::PURPLE,
				// 				Color::BLUE,
				// 				Color::CYAN,
				// 				Color::YELLOW,
				// 				Color::RED,
				// 			];
				// 			for color in colors.into_iter() {
				// 				// Create rainbow from entity to current extrinsic location.
				// 				// commands
				// 				// 	.spawn_bundle(PolylineBundle {
				// 				// 		polyline: polylines
				// 				// 			.add(Polyline { vertices: vertices.clone() }),
				// 				// 		material: polyline_materials.add(PolylineMaterial {
				// 				// 			width: 10.0,
				// 				// 			color,
				// 				// 			perspective: true,
				// 				// 			..default()
				// 				// 		}),
				// 				// 		..default()
				// 				// 	})
				// 				// 	.insert(ClearMe);

				// 				for v in vertices.iter_mut() {
				// 					v.y += 0.5;
				// 				}
				// 			}

				// 			// commands.entity(entity).remove::<MessageSource>();
				// 		}
				// 	}
				// }

				// for (link, link_type) in block.start_link() {
				// 	// println!("inserting source of rainbow!");
				// 	create_source
				// 		.push(MessageSource { id: link.to_string(), link_type: *link_type });
				// }

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
				render_details.cube_instances.push(block.details().clone());
				// extrinsic_instances.1.push(false);

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
		let blocklink =
			format!("https://polkadot.js.org/apps/?{}#/explorer/query/{}", &encoded, block_num);

		for event in events {
			let mut entity = event;

			let style = style::style_data_event(&entity);
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
			extrinsic_instances.push((
				Instance {
					position: glam::Vec3::new(x, (5. * build_dir) + y, 5. + z).into(),
					// scale: base_y + target_y * build_dir,
					color: style.color,
					// flags: 0,
				},
				base_y + target_y * build_dir,
			));

			let mut details = entity.details.clone();
			details.url = blocklink.clone();

			render_details.cube_instances.push(details);

			// let mut x = commands.spawn_bundle(PbrBundle {
			// 	mesh,
			// 	material: material.clone(),
			// 	transform: t,
			// 	..Default::default()
			// });
			// let event_bun = x
			// 	.insert_bundle(PickableBundle::default())
			// 	.insert(entity.details.clone())
			// 	.insert(Rainable { dest: base_y + target_y * build_dir, build_direction })
			// 	.insert(Name::new("BlockEvent"))
			// 	.insert(ClearMe)
			// 	.insert(HiFi);
			//
			// for (link, link_type) in &event.start_link {
			// 	// println!("inserting source of rainbow (an event)!");
			// 	event_bun.insert(MessageSource { id: link.to_string(), link_type: *link_type });
			// }
		}

		// commands.spawn().insert_bundle((
		// 	handles.block_mesh.clone(), //todo xcm different?
		// 	Transform::from_xyz(base_x, base_y, base_z),
		// 	GlobalTransform::default(),
		// 	InstanceMaterialData(instances),
		// 	Visibility::default(),
		// 	ComputedVisibility::default(),
		// 	// NOTE: Frustum culling is done based on the Aabb of the Mesh and the GlobalTransform.
		// 	// As the cube is at the origin, if its Aabb moves outside the view frustum, all the
		// 	// instanced cubes will be culled.
		// 	// The InstanceMaterialData contains the 'GlobalTransform' information for this custom
		// 	// instancing, and that is not taken into account with the built-in frustum culling.
		// 	// We must disable the built-in frustum culling by adding the `NoFrustumCulling` marker
		// 	// component to avoid incorrect culling.
		// 	NoFrustumCulling,
		// ))
		// .insert_bundle(PickableBundle::default())
		// 		.insert(Name::new("BlockEvent"))
		// 		.insert(ClearMe)
		// 		.insert(HiFi);

		// }
		// }
		//log!("hohohohohohhohoo");
	}
}

/// Yes this is now a verb. Who knew?
fn rainbow(vertices: &mut Vec<glam::Vec3>, points: usize) {
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
		vertices.push(glam::Vec3::new(x, y, z).into());
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
	drops: &mut Vec<Instance>,
	drops_target: &mut Vec<f32>, // mut timer: ResMut<UpdateTimer>,
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

static LAST_CLICK_TIME: AtomicI32 = AtomicI32::new(0);
static LAST_KEYSTROKE_TIME: AtomicI32 = AtomicI32::new(0);

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

// struct BlockHandles {

// 	// block_material: Handle<StandardMaterial>

// }

// struct ResourceHandles {
// 	block_mesh: Handle<Mesh>,
// 	banner_materials: HashMap<isize, Handle<StandardMaterial>>,
// 	banner_mesh: Handle<Mesh>,
// 	sphere_mesh: Handle<Mesh>,
// 	xcm_torus_mesh: Handle<Mesh>,
// 	extrinsic_mesh: Handle<Mesh>,

// 	chain_rect_mesh: Handle<Mesh>,
// 	darkside_rect_material: Handle<StandardMaterial>,
// 	lightside_rect_material: Handle<StandardMaterial>,
// }

/// set up a simple 3D scene
// fn setup(
// 	mut commands: Commands,
// 	mut meshes: ResMut<Assets<Mesh>>,
// 	mut materials: ResMut<Assets<StandardMaterial>>,
// 	mut datasource_events: EventWriter<DataSourceChangedEvent>,
// ) {
// 	let block_mesh = meshes.add(Mesh::from(shape::Box::new(10., 0.2, 10.)));
// 	let aspect = 1. / 3.;

// 	let handles = ResourceHandles {
// 		block_mesh,
// 		banner_materials: default(),
// 		banner_mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::new(BLOCK, BLOCK * aspect)))),
// 		sphere_mesh: meshes.add(Mesh::from(shape::Icosphere { radius: 0.40, subdivisions: 32 })),
// 		xcm_torus_mesh: meshes.add(Mesh::from(shape::Torus {
// 			radius: 0.6,
// 			ring_radius: 0.4,
// 			subdivisions_segments: 20,
// 			subdivisions_sides: 10,
// 		})),
// 		extrinsic_mesh: meshes.add(Mesh::from(shape::Box::new(0.8, 0.8, 0.8))),
// 		lightside_rect_material: materials.add(StandardMaterial {
// 			base_color: Color::rgba(0.5, 0.5, 0.5, 0.4),
// 			alpha_mode: AlphaMode::Blend,
// 			perceptual_roughness: 0.08,
// 			reflectance: 0.0,
// 			unlit: false,
// 			..default()
// 		}),
// 		darkside_rect_material: materials.add(StandardMaterial {
// 			base_color: Color::rgba(0., 0., 0., 0.4),
// 			alpha_mode: AlphaMode::Blend,
// 			perceptual_roughness: 1.0,
// 			reflectance: 0.5,
// 			unlit: true,
// 			..default()
// 		}),
// 		chain_rect_mesh: meshes.add(Mesh::from(shape::Box::new(10000., 0.1, 10.1))),
// 	};

// 	commands.insert_resource(handles);

// 	// add entities to the world
// 	// plane

// 	commands.spawn_bundle(PbrBundle {
// 		mesh: meshes.add(Mesh::from(shape::Box::new(50000., 0.1, 50000.))),
// 		material: materials.add(StandardMaterial {
// 			base_color: Color::rgba(0.2, 0.2, 0.2, 0.3),
// 			alpha_mode: AlphaMode::Blend,
// 			perceptual_roughness: 0.08,
// 			..default()
// 		}),
// 		transform: Transform { translation: Vec3::new(0., 0., -25000.), ..default() },
// 		..default()
// 	});
// 	commands.spawn_bundle(PbrBundle {
// 		mesh: meshes.add(Mesh::from(shape::Box::new(50000., 0.1, 50000.))),
// 		material: materials.add(StandardMaterial {
// 			base_color: Color::rgba(0.2, 0.2, 0.2, 0.3),
// 			alpha_mode: AlphaMode::Blend,
// 			perceptual_roughness: 0.08,
// 			unlit: true,
// 			..default()
// 		}),
// 		transform: Transform { translation: Vec3::new(0., 0., 25000.), ..default() },
// 		..default()
// 	});

// 	//somehow this can change the color
// 	//    mesh_highlighting(None, None, None);
// 	// camera
// 	let camera_transform =
// 		Transform::from_xyz(200.0, 50., 0.0).looking_at(Vec3::new(-1000., 1., 0.), Vec3::Y);
// 	commands.insert_resource(ui::OriginalCameraTransform(camera_transform));
// 	let mut entity_comands = commands.spawn_bundle(Camera3dBundle {
// 		transform: camera_transform,

// 		// perspective_projection: PerspectiveProjection {
// 		// 	// far: 1., // 1000 will be 100 blocks that you can s
// 		// 	//far: 10.,
// 		// 	far: f32::MAX,
// 		// 	near: 0.000001,
// 		// 	..default()
// 		// },
// 		// camera: Camera { //far: 10.,
// 		// 	far:f32::MAX,
// 		// 	near: 0.000001, ..default() },
// 		..default()
// 	});
// 	#[cfg(feature = "normalmouse")]
// 	entity_comands.insert(FlyCam);
// 	entity_comands
// 		.insert(Viewport)
// 		.insert_bundle(PickingCameraBundle { ..default() });

// 	// #[cfg(feature = "spacemouse")]
// 	// entity_comands.insert(SpaceMouseRelativeControllable);

// 	commands.insert_resource(UpdateTimer { timer: Timer::new(Duration::from_millis(15), true) });

// 	// light

// 	commands.insert_resource(AmbientLight { color: Color::WHITE, brightness: 0.9 });

// 	// commands.spawn_bundle(PointLightBundle {
// 	//     transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
// 	//     ..Default::default()
// 	// });
// 	// commands.spawn_bundle(PointLightBundle {
// 	// 	transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
// 	// 	..Default::default()
// 	// });

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

	texture: Option<egui::TextureHandle>,
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
	GetDetails(u32),
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

use std::sync::mpsc::{Receiver, Sender};
type OnResizeSender = Sender<()>;
type OnResizeReceiver = Receiver<()>;

//todo: needs to be in a mutex really?
fn setup_viewport_resize_system(resize_sender: Mutex<OnResizeSender>) {
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

// fn viewport_resize_system(
// 	// mut window: &mut Window,
// 	resize_receiver: &Mutex<OnResizeReceiver>,
// ) -> Option<winit::dpi::PhysicalSize<u32>> {
// 	if resize_receiver.lock().unwrap().try_recv().is_ok() {
// 		let new_size = get_viewport_size();
// 		//TODO: bugout if window size is already this.
// 		if new_size.width > 0 && new_size.height > 0 {
// 			return Some(new_size)
// 			// log!("I GOT CALLED with {}, {}", size.0, size.1);// width, height
// 		}
// 	}
// 	return None
// }

use core::cell::RefCell;
use futures::task::{Context, Poll};
// use core::future::Future;
use core::pin::Pin;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
// use wasm_bindgen::JsCast;
use core::cell::Cell;
use web_sys::HtmlImageElement;

pub struct ImageFuture {
	image: Option<HtmlImageElement>,
	load_failed: Rc<Cell<bool>>,
}

impl ImageFuture {
	pub fn new(path: &str) -> Self {
		let image = HtmlImageElement::new().unwrap();
		image.set_src(path);
		ImageFuture { image: Some(image), load_failed: Rc::new(Cell::new(false)) }
	}
}

impl Future for ImageFuture {
	type Output = Result<HtmlImageElement, ()>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match &self.image {
			Some(image) if image.complete() => {
				let image = self.image.take().unwrap();
				let failed = self.load_failed.get();

				if failed {
					Poll::Ready(Err(()))
				} else {
					Poll::Ready(Ok(image))
				}
			},
			Some(image) => {
				let waker = cx.waker().clone();
				let on_load_closure = Closure::wrap(Box::new(move || {
					waker.wake_by_ref();
				}) as Box<dyn FnMut()>);
				image.set_onload(Some(on_load_closure.as_ref().unchecked_ref()));
				on_load_closure.forget();

				let waker = cx.waker().clone();
				let failed_flag = self.load_failed.clone();
				let on_error_closure = Closure::wrap(Box::new(move || {
					failed_flag.set(true);
					waker.wake_by_ref();
				}) as Box<dyn FnMut()>);
				image.set_onerror(Some(on_error_closure.as_ref().unchecked_ref()));
				on_error_closure.forget();

				Poll::Pending
			},
			_ => Poll::Ready(Err(())),
		}
	}
}
