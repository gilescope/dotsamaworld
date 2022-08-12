#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(slice_pattern)]
#![feature(slice_group_by)]
#![feature(option_get_or_insert_default)]
#![feature(async_closure)]
use crate::ui::UrlBar;
use bevy::{ecs as bevy_ecs, prelude::*};
// use bevy::winit::WinitSettings;
use bevy_ecs::prelude::Component;
use bevy_egui::EguiPlugin;
#[cfg(feature = "normalmouse")]
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};
//use bevy_kira_audio::AudioPlugin;
// use bevy_inspector_egui::{Inspectable, InspectorPlugin};
use bevy_mod_picking::*;
//use bevy_egui::render_systems::ExtractedWindowSizes;
//use bevy::window::PresentMode;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
// use bevy::diagnostic::LogDiagnosticsPlugin;
use crate::movement::Destination;
#[cfg(feature = "adaptive-fps")]
use bevy::diagnostic::Diagnostics;
use bevy::window::RequestRedraw;
use bevy_polyline::{prelude::*, PolylinePlugin};
// use scale_info::build;
use std::{
	collections::HashMap,
	num::NonZeroU32,
	sync::{
		atomic::{AtomicU32, Ordering},
		Arc,
	},
};
use bevy::{
    tasks::{AsyncComputeTaskPool, Task},
};
// use rayon::prelude::*;
// use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
// use ui::doturl;
//use bevy_kira_audio::Audio;
use std::{sync::Mutex, time::Duration};
mod content;
use std::sync::atomic::AtomicI32;
mod datasource;
mod movement;
mod style;
mod ui;
use crate::ui::{Details, DotUrl};
// use bevy_inspector_egui::RegisterInspectable;
// use bevy_inspector_egui::WorldInspectorPlugin;
use bevy::winit::WinitSettings;
#[cfg(feature = "spacemouse")]
use bevy_spacemouse::{SpaceMousePlugin, SpaceMouseRelativeControllable};
use chrono::prelude::*;
use datasource::DataUpdate;
use primitive_types::H256;
use std::convert::{AsRef, TryInto};
// #[subxt::subxt(runtime_metadata_path = "polkadot_metadata.scale")]
// pub mod polkadot {}
pub mod recorder;

/// Pick a faster allocator.
#[cfg(all(not(target_env = "msvc"), not(target_arch="wasm32")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_env = "msvc"), not(target_arch="wasm32")))]
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
const LAYER_GAP: f32 = 10.;
use  lazy_static::lazy_static;
/// The time by which all times should be placed relative to each other on the x axis.
lazy_static! {
static ref BASETIME: Arc<Mutex<i64>> = Arc::new(Mutex::new(0_i64));
}
/// Bump this to tell the current datasources to stop.
static DATASOURCE_EPOC: AtomicU32 = AtomicU32::new(0);

/// if you need bestest fps...
static PAUSE_DATA_FETCH: AtomicU32 = AtomicU32::new(0);

static LIVE: &str = "dotsama:live";

/// Immutable once set up.
#[derive(Clone)]
pub struct ChainInfo {
	// pub chain_name: String,
	pub chain_ws: String,
	// pub chain_id: Option<NonZeroU32>,
	// pub chain_drawn: bool,
	// Negative is other direction from center.
	pub chain_index: isize,
	pub chain_url: DotUrl,
	// pub chain_name: String,
}

pub type ABlocks = Arc<
	Mutex<
		// Queue of new data to be processed.
		Vec<datasource::DataUpdate>,
	>,
>;

mod networks;
use networks::Env;

pub struct DataSourceChangedEvent {
	source: String,
	timestamp: Option<i64>,
}

#[derive(Default)]
pub struct Anchor {
	pub follow_chain: bool,
}

#[derive(Component)]
pub struct HiFi;
#[derive(Component)]
pub struct MedFi;


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

fn main() -> () {
	async_std::task::block_on(async_main());
}
macro_rules! log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

async fn async_main() -> color_eyre::eyre::Result<()> {
	// color_eyre::install()?;
//   console_log!("Hello {}!", "world");
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
	// let error = console_log::init_with_level(Level::Warn);
	//.expect("Failed to enable logging");
	//use log::{error, info, Level};

	// App assumes the target dir exists
	#[cfg(not(feature="wasm23"))]
	let _ = std::fs::create_dir_all("target");

	let low_power_mode = false;

	let mut app = App::new();
	// app
	app.insert_resource(Msaa { samples: 4 });
	
	// The web asset plugin must be inserted before the `AssetPlugin` so
    // that the asset plugin doesn't create another instance of an asset
    // server. In general, the AssetPlugin should still run so that other
    // aspects of the asset system are initialized correctly.
    //app.add_plugin(bevy_web_asset::WebAssetPlugin);

	app.add_plugins_with(DefaultPlugins, |group| {
    // The web asset plugin must be inserted in-between the
    // `CorePlugin' and `AssetPlugin`. It needs to be after the
    // CorePlugin, so that the IO task pool has already been constructed.
    // And it must be before the `AssetPlugin` so that the asset plugin
    // doesn't create another instance of an assert server. In general,
    // the AssetPlugin should still run so that other aspects of the
    // asset system are initialized correctly.
    group.add_before::<bevy::asset::AssetPlugin, _>(bevy_web_asset::WebAssetPlugin)
});
	// app.add_plugins(DefaultPlugins);

	
	//  .insert_resource(WinitSettings::desktop_app()) - this messes up the 3d space mouse?
	app.add_event::<DataSourceChangedEvent>();
	app.add_event::<DataSourceStreamEvent>();
	app.insert_resource(MovementSettings {
		sensitivity: 0.00020, // default: 0.00012
		speed: 12.0,          // default: 12.0
		boost: 5.,
	});
	app.insert_resource(Vec::<PollSource>::new());
	app.insert_resource(ui::UrlBar::new(
		"dotsama:/1//10504599".to_string(),
		Utc::now().naive_utc(),
	));
	app.insert_resource(Sovereigns { relays: vec![], default_track_speed: 1. });

	#[cfg(feature = "normalmouse")]
	app.add_plugin(NoCameraPlayerPlugin);
	app.insert_resource(movement::MouseCapture::default());
	app.insert_resource(Anchor::default());
	app.insert_resource(Width(500.));
	app.insert_resource(Inspector::default());

	#[cfg(feature = "spacemouse")]
	app.add_plugin(SpaceMousePlugin);

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
	app.add_plugin(HighlightablePickingPlugin);

	app.add_plugin(PickingPlugin)
		// .insert_resource(camera_rig)
		.insert_resource(movement::Destination::default());
		app.add_system(ui::ui_bars_system);
		// .add_plugin(recorder::RecorderPlugin)
		// .add_system(movement::rig_system)
		app.add_plugin(InteractablePickingPlugin);
		// .add_plugin(HighlightablePickingPlugin);
		// .add_plugin(DebugCursorPickingPlugin) // <- Adds the green debug cursor.
		// .add_plugin(InspectorPlugin::<Inspector>::new())
		// .register_inspectable::<Details>()
		// .add_plugin(DebugEventsPickingPlugin)
		app.add_plugin(PolylinePlugin);
		app.add_plugin(EguiPlugin);
		app.insert_resource(ui::OccupiedScreenSpace::default());
		app.add_system(movement::scroll);
		app.add_system(source_poll);
		app.add_startup_system(setup);
	#[cfg(feature = "spacemouse")]
	app.add_startup_system(move |mut scale: ResMut<bevy_spacemouse::Scale>| {
		scale.rotate_scale = 0.00010;
		scale.translate_scale = 0.004;
	});
	app.add_system(movement::player_move_arrows)
		.add_system(rain)
		.add_system(source_data);
		// // .add_system(pad_system)
		// // .add_plugin(LogDiagnosticsPlugin::default())
		app.add_plugin(FrameTimeDiagnosticsPlugin::default());
		// // .add_system(ui::update_camera_transform_system)
		app.add_system(right_click_system);
		app.add_system_to_stage(CoreStage::PostUpdate, update_visibility);
   		app.add_startup_system(ui::details::configure_visuals);
		app.insert_resource(bevy_atmosphere::AtmosphereMat::default()); // Default Earth sky
		app.add_plugin(bevy_atmosphere::AtmospherePlugin {
			dynamic: false, // Set to false since we aren't changing the sky's appearance
			sky_radius: 1000.0,
		});
		app.add_system(render_block);
		app.add_system_to_stage(CoreStage::PostUpdate, print_events);

	app.run();

	Ok(())
}

struct DataSourceStreamEvent(ChainInfo, datasource::DataUpdate);

fn chain_name_to_url(chain_name: &str) -> String {
	let mut chain_name = chain_name.to_string();
	if !chain_name.starts_with("ws:") && !chain_name.starts_with("wss:") {
		chain_name = format!("wss://{}", chain_name);
	}

	if chain_name[5..].contains(':') {
		format!("{chain_name}")
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

#[cfg(not(target_arch="wasm32"))]
#[derive(Component)]
struct SourceDataTask(bevy::tasks::Task<Result<(), std::boxed::Box<dyn std::error::Error + Send + Sync>>>);

#[cfg(target_arch="wasm32")]
#[derive(Clone)]
struct PollSource(Arc<Mutex<Option<polkapipe::ws_web::Backend>>>);

fn source_poll(sources: ResMut<Vec<PollSource>>, thread_pool: Res<AsyncComputeTaskPool>) {
	// log!("polling checking for incoming messages");
	let sources = (*sources).clone();
	let data_source_task3 = thread_pool.spawn_local((async move || 
	{
		for source in sources.iter() {
			if let Ok(source) = source.0.try_lock() {
				if let Some(source) = &*source {
					// log!("pulling in incoming messages");
					source.process_incoming_messages().await;
				}
			} 
		}
	}
	)());
	data_source_task3.detach();
}

fn source_data<'a, 'b, 'c, 'd, 'e, 'f,'g,'h,'i,'j>(
	mut datasource_events: EventReader<'a, 'b, DataSourceChangedEvent>,
	mut commands: Commands<'c,'d>,
	mut sovereigns: ResMut<'e, Sovereigns>,
	details: Query<'f, 'g, Entity, With<ClearMeAlwaysVisible>>,
	clean_me: Query<'h, 'i, Entity, With<ClearMe>>,
	// mut dest: ResMut<Destination>,
	mut spec: ResMut<'j, UrlBar>,
	writer: EventWriter<DataSourceStreamEvent>,
	thread_pool: Res<AsyncComputeTaskPool>,
	mut sources: ResMut<Vec<PollSource>>
) {
	for event in datasource_events.iter() {
		log!("data source changes to {} {:?}", event.source, event.timestamp);

		clear_world(&details, &mut commands, &clean_me);

		if event.source.is_empty() {
			log!("Datasources cleared epoc {}", DATASOURCE_EPOC.load(Ordering::Relaxed));
			return
		}

		let dot_url = DotUrl::parse(&event.source);

		let is_live = if let Some(timestamp) = event.timestamp {
			// if time is now or in future then we are live mode.
			let is_live = timestamp >= (Utc::now().timestamp() * 1000);
			if is_live {
				*BASETIME.lock().unwrap() = Utc::now().timestamp() * 1000;
				spec.timestamp = Utc::now().naive_utc();
				spec.reset_changed();
			} else {
				*BASETIME.lock().unwrap() = timestamp;
			}
			is_live
		} else {
			LIVE == event.source
		};
		// if is_live {
		// 	event.timestamp = None;
		// }

		log!("event source {}", event.source);
		sovereigns.default_track_speed = if is_live { 0.1 } else { 0.7 };
		log!("tracking speed set to {}", sovereigns.default_track_speed);
		let (dot_url, as_of): (DotUrl, Option<DotUrl>) = if is_live {
			(DotUrl::default(), None)
		} else {
			(dot_url.clone().unwrap(), Some(dot_url.unwrap()))
		};

		let selected_env = &dot_url.env; //if std::env::args().next().is_some() { Env::Test } else {Env::Prod};
		log!("dot url {:?}", &dot_url);
		//let as_of = Some(dot_url.clone());
		log!("Block number selected for relay chains: {:?}", &as_of);


		let networks = networks::get_network(&selected_env);

		// let is_self_sovereign = selected_env.is_self_sovereign();
		let relays = networks
			.into_iter()
			.enumerate()
			.map(|(relay_index, relay)| {				
				let relay_url = DotUrl {
					sovereign: Some(if relay_index == 0 { -1 } else { 1 }),
					..dot_url.clone()
				};
				//relay.as_slice().par_iter().
				relay.as_slice().iter().enumerate().filter_map( |(chain_index, (para_id, chain_name))| {
					let url = chain_name_to_url(&chain_name);
					#[cfg(not(target_arch="wasm32"))]
					let mut source = datasource::CachedDataSource::new(
						datasource::RawDataSource::new(&url),
					);
					#[cfg(target_arch="wasm32")]
					let mut source = datasource::RawDataSource::new(&url);
					// #[cfg(not(target_arch="wasm32"))]
					// let para_id = async_std::task::block_on(datasource::get_parachain_id_from_url(&mut source));
					// #[cfg(target_arch="wasm32")]
					// let para_id:  Result<Option<NonZeroU32>, polkapipe::Error> = if datasource::is_relay_chain(&url) { Ok(None) } else {Ok(Some(NonZeroU32::try_from(7777u32).unwrap()))};
					// if para_id.is_err() {
					// 	return None;
					// }
					//let para_id = para_id.unwrap();

					Some((
						Chain {
							shared: ABlocks::default(),
							// name: chain_name.to_string(),
							info: ChainInfo {
								chain_ws: url,
								// +2 to skip 0 and relay chain.
								chain_index: if relay_url.is_darkside() {
									-((chain_index + 2) as isize)
								} else {
									(chain_index + 2) as isize
								},
								chain_url: DotUrl { para_id: para_id.clone(), ..relay_url.clone() },
								// chain_name: parachain_name,
							},
						},
						source,
					))
				}).collect::<Vec<(Chain, _  )>>(
					)
			})
			.collect::<Vec<Vec<_>>>();

		for relay in relays.into_iter() {
			// let relay_url = DotUrl {
			//     sovereign: Some(relay_id as u32),
			//     ..dot_url.clone()
			// };
			let mut relay2: Vec<(
				(Chain, _),
				_,
			)> = vec![];
			let mut send_map: HashMap<
				NonZeroU32,
				async_std::channel::Sender<(datasource::RelayBlockNumber, i64, H256)>,
			> = Default::default();
			for (chain, source) in relay.into_iter() {
				let (tx, rc) = async_std::channel::unbounded();
				if let Some(para_id) = chain.info.chain_url.para_id {
					send_map.insert(para_id, tx);
				}
				relay2.push(((chain, source), rc));
			}

			let mut send_map = Some(send_map);
			let mut sov_relay = vec![];
			for ((chain, source), rc) in relay2 {
				log!("listening to {}", chain.info.chain_ws);

				let url_clone = chain.info.chain_ws.clone();
				let maybe_sender =
					if chain.info.chain_url.is_relay() { send_map.take() } else { None };

				let lock_clone = chain.shared.clone();
				let as_of = as_of.clone();
				log!("as of for chain {:?}", &as_of);
				let chain_info = chain.info.clone();

				let block_watcher = datasource::BlockWatcher{
					tx: Some(lock_clone.clone()),
					chain_info,
					as_of,
					receive_channel: Some(rc),
					sender: maybe_sender,
					// source
				};





				let data_source_task = thread_pool.spawn_local(
					block_watcher.watch_blocks(source));
				data_source_task.detach();

				use crate::datasource::Source;

				//let mut cloned = source.clone();
				//#[cfg(target_arch="wasm32")]
				// let data_source_task2 = thread_pool.spawn_local(async move {
				// 	log!("spawned local task run");
				// 	let to_poll = cloned.process_incoming_messages();
				// });
				//let poll_source = PollSource(Arc::new(Mutex::new(None)));
				//let clone = poll_source.0.clone();
				// let data_source_task3 = thread_pool.spawn_local((async move || 
				// 	{
				// 		let backend = cloned.process_incoming_messages().await;
				// 		// We can't really return things in wasm tasks so return via mutex.
				// 		*clone.lock().unwrap() = Some(backend);
				// 	}
				// )());
				// sources.push(poll_source);
				// data_source_task3.detach();

				// #[cfg(target_arch="wasm32")]
				// data_source_task2.detach();

				// 	sync move || {
				// 	// let mut reconnects = 0;

				// 	// while reconnects < 20 {
				// 	println!("Connecting to {} as of {:?}", &url_clone, as_of);
				// 	let _res = datasource::watch_blocks(
				// 		lock_clone.clone(),
				// 		chain_info,
				// 		as_of,
				// 		rc,
				// 		maybe_sender,
				// 		source,
				// 	).await;

				// 	// if res.is_ok() { break; }
				// 	// println!(
				// 	//     "Problem with {} blocks (retries left {})",
				// 	//     &url_clone, reconnects
				// 	// );
				// 	// std::thread::sleep(std::time::Duration::from_secs(20));
				// 	// reconnects += 1;
				// 	// }
				// 	// println!("finished listening to {} {:?}", url_clone, my_writer);
				// }

				// We do this to signal that they are long running tasks.
				// (essential for wasm32 as a FakeTask is returned)
				

				// #[cfg(not(target_arch="wasm32"))]
				// commands.spawn().insert(SourceDataTask(data_source_task));
				// commands.spawn().insert(SourceDataTask(data_source_task));
				
				sov_relay.push(chain);
			}
			sovereigns.relays.push(sov_relay);
		}
		//TODO: sovereigns.relays = relays;
	}
}

fn draw_chain_rect(
	chain_info: &ChainInfo,
	commands: &mut Commands,
	meshes: &mut ResMut<Assets<Mesh>>,
	materials: &mut ResMut<Assets<StandardMaterial>>,
) {
	let rfip = chain_info.chain_url.rflip();
	let chain_index = chain_info.chain_index.abs() as usize;
	let encoded: String = url::form_urlencoded::Serializer::new(String::new())
		.append_pair("rpc", &chain_info.chain_ws)
		.finish();
	let is_relay = chain_info.chain_url.is_relay();
	commands
		.spawn_bundle(PbrBundle {
			mesh: meshes.add(Mesh::from(shape::Box::new(10000., 0.1, 10.))),
			material: if chain_info.chain_url.is_darkside() {
				materials.add(StandardMaterial {
					base_color: Color::rgba(0., 0., 0., 0.4),
					alpha_mode: AlphaMode::Blend,
					perceptual_roughness: 1.0,
					reflectance: 0.5,
					unlit: true,
					..default()
				})
			} else {
				materials.add(StandardMaterial {
					base_color: Color::rgba(0.5, 0.5, 0.5, 0.4),
					alpha_mode: AlphaMode::Blend,
					perceptual_roughness: 0.08,
					reflectance: 0.0,
					unlit: false,
					..default()
				})
			},
			transform: Transform::from_translation(Vec3::new(
				(10000. / 2.) - 35.,
				if is_relay { 0. } else { LAYER_GAP },
				((RELAY_CHAIN_CHASM_WIDTH - 5.) +
					(BLOCK / 2. + BLOCK_AND_SPACER * chain_index as f32)) *
					rfip,
			)),
			..Default::default()
		})
		.insert(Details {
			doturl: DotUrl { block_number:None, ..chain_info.chain_url.clone()},
			flattern: chain_info.chain_ws.to_string(),
			url: format!("https://polkadot.js.org/apps/?{}", &encoded),
			..default()
		})
		.insert(Name::new("Blockchain"))
		.insert(ClearMeAlwaysVisible)
		// .insert_bundle(PickableBundle::default())
		;
}

fn clear_world(
	details: &Query<Entity, With<ClearMeAlwaysVisible>>,
	commands: &mut Commands,
	clean_me: &Query<Entity, With<ClearMe>>,
) {
	// Stop previous data sources...
	DATASOURCE_EPOC.fetch_add(1, Ordering::Relaxed);
	println!("incremet epoc to {}", DATASOURCE_EPOC.load(Ordering::Relaxed));

	for detail in details.iter() {
		commands.entity(detail).despawn();
	}
	for detail in clean_me.iter() {
		commands.entity(detail).despawn();
	}
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

#[derive(Clone)]
pub enum DataEntity {
	Event(DataEvent),
	Extrinsic {
		// id: (u32, u32),
		// pallet: String,
		// variant: String,
		args: Vec<String>,
		contains: Vec<DataEntity>,
		raw: Vec<u8>,
		/// pseudo-unique id to link to some other node(s).
		/// There can be multiple destinations per block! (TODO: need better resolution)
		/// Is this true of an extrinsic - system ones plus util batch could do multiple msgs.
		start_link: Vec<(String, LinkType)>,
		/// list of links that we have finished
		end_link: Vec<(String, LinkType)>,
		details: Details,
	},
}

#[derive(Clone)]
pub struct DataEvent {
	details: Details,
	start_link: Vec<(String, LinkType)>,
	// end_link: Vec<String>,
}

/// A tag to identify an entity as being the source of a message.
#[derive(Component)]
pub struct MessageSource {
	/// Currently sending block id + hash of beneficiary address.
	pub id: String,
	pub link_type: LinkType,
}

#[derive(Clone, Copy)]
pub enum LinkType {
	Teleport,
	ReserveTransfer,
	ReserveTransferMintDerivative,
	ParaInclusion,
}

static EMPTY_SLICE: Vec<DataEntity> = vec![];
static EMPTY_BYTE_SLICE: Vec<u8> = vec![];

impl DataEntity {
	pub fn details(&self) -> &Details {
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

	pub fn as_bytes(&self) -> &[u8] {
		match self {
			Self::Event(DataEvent { .. }) => EMPTY_BYTE_SLICE.as_slice(),
			Self::Extrinsic { raw, .. } => raw.as_slice(),
		}
	}

	pub fn start_link(&self) -> &Vec<(String, LinkType)> {
		match self {
			Self::Extrinsic { start_link, .. } => &start_link,
			Self::Event(DataEvent { .. }) => &EMPTY_VEC,
		}
	}
	pub fn end_link(&self) -> &Vec<(String, LinkType)> {
		match self {
			Self::Extrinsic { end_link, .. } => &end_link,
			Self::Event(DataEvent { .. }) => &EMPTY_VEC,
		}
	}
}

static EMPTY_VEC: Vec<(String, LinkType)> = vec![];

const BLOCK: f32 = 10.;
const BLOCK_AND_SPACER: f32 = BLOCK + 4.;
const RELAY_CHAIN_CHASM_WIDTH: f32 = 10.;

#[derive(Clone)]
pub struct Chain {
	shared: ABlocks,
	info: ChainInfo,
}

pub struct Sovereigns {
	//                            name    para_id             url
	pub relays: Vec<Vec<Chain>>,
	pub default_track_speed: f32,
}

#[derive(Component)]
struct ClearMe;

#[derive(Component)]
struct ClearMeAlwaysVisible;

static CHAINS: AtomicU32 = AtomicU32::new(0);

static BLOCKS: AtomicU32 = AtomicU32::new(0);

static EXTRINSICS: AtomicU32 = AtomicU32::new(0);

static EVENTS: AtomicU32 = AtomicU32::new(0);

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

fn render_block(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	relays: Res<Sovereigns>,
	asset_server: Res<AssetServer>,
	// effects: Res<Assets<EffectAsset>>,
	links: Query<(Entity, &MessageSource, &GlobalTransform)>,
	mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
	mut polylines: ResMut<Assets<Polyline>>,
	mut event: EventWriter<RequestRedraw>,
	
	reader: EventReader<DataSourceStreamEvent>,
) {
	// let is_self_sovereign = false; //TODO
	for relay in &relays.relays {
		for chain in relay.iter() {
	// for DataSourceStreamEvent(chain_info, data_update) in reader.iter() {
		// for chain in relay.iter() {
			 if let Ok(ref mut block_events) = chain.shared.try_lock() {
				let chain_info = &chain.info;
				if let Some(data_update) = (*block_events).pop() {
		match data_update {
			DataUpdate::NewBlock(block) => {
				BLOCKS.fetch_add(1, Ordering::Relaxed);

				// println!(
				// 	"chains {} blocks {} txs {} events {}",
				// 	CHAINS.load(Ordering::Relaxed),
				// 	BLOCKS.load(Ordering::Relaxed),
				// 	EXTRINSICS.load(Ordering::Relaxed),
				// 	EVENTS.load(Ordering::Relaxed)
				// );

				// Skip data we no longer care about because the datasource has changed
				let now_epoc = DATASOURCE_EPOC.load(Ordering::Relaxed);
				if block.data_epoc != now_epoc {
					println!(
						"discarding out of date block made at {} but we are at {}",
						block.data_epoc, now_epoc
					);
					continue
				}

				let mut base_time = *BASETIME.lock().unwrap();
				if base_time == 0 {
					base_time = block.timestamp.unwrap_or(0);
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
				let encoded: String =
					url::form_urlencoded::Serializer::new(String::new())
						.append_pair("rpc", &chain_info.chain_ws)
						.finish();

				let is_relay = chain_info.chain_url.is_relay();
				let details = Details {
					doturl: DotUrl {
						extrinsic: None,
						event: None,
						..block.blockurl.clone()
					},

					url: format!(
						"https://polkadot.js.org/apps/?{}#/explorer/query/0x{}",
						&encoded,
						hex::encode(block.blockhash)
					),
					..default()
				};

				// println!("block.timestamp {:?}", block.timestamp);
				// println!("base_time {:?}",base_time);
				let block_num = timestamp_to_x(block.timestamp.unwrap_or(base_time));

				// Add the new block as a large square on the ground:
				{
					let timestamp_color = if chain_info.chain_url.is_relay() {
						block.timestamp.unwrap()
					} else {
						block.timestamp_parent.unwrap_or(block.timestamp.unwrap())
					} / 400;

					let transform = Transform::from_translation(Vec3::new(
						0. + (block_num as f32),
						if is_relay { 0. } else { LAYER_GAP },
						(RELAY_CHAIN_CHASM_WIDTH +
							BLOCK_AND_SPACER * chain_info.chain_index.abs() as f32) *
							rflip,
					));
					// println!("block created at {:?} blocknum {}", transform,
					// block_num);

					let mut bun = commands.spawn_bundle(PbrBundle {
						mesh: meshes.add(Mesh::from(shape::Box::new(10., 0.2, 10.))),
						material: materials.add(StandardMaterial {
							base_color: style::color_block_number(
								timestamp_color,
								chain_info.chain_url.is_darkside(),
							), // Color::rgba(0., 0., 0., 0.7),
							alpha_mode: AlphaMode::Blend,
							perceptual_roughness: 0.08,
							unlit: if block.blockurl.is_darkside() {
								true
							} else {
								false
							},
							..default()
						}),
						transform,
						..Default::default()
					});

					bun.insert(ClearMe);
					// bun.insert(Aabb::from_min_max(
					//     Vec3::new(0., 0., 0.),
					//     Vec3::new(1., 1., 1.),
					// ));
					let chain_str = details.doturl.chain_str();

					bun.insert(details)
						.insert(Name::new("Block"))
						.with_children(|parent| {
							// let name = chain_info
							// .chain_name
							// .replace(" ", "-")
							// .replace("-Testnet", "");
							let texture_handle = asset_server

							
							// .load("https://jpeg.org/images/jpegpleno-home.jpg");
							.load(&format!("http://0.0.0.0:8080/branding/{}.jpeg", chain_str));
							// .load(&format!("https://github.com/gilescope/dotsamaworld/blob/dyn/assets/branding/{}.jpeg?raw=true", chain_str));
							//	.load(&format!("branding/{}.jpeg", chain_str));
							let aspect = 1. / 3.;

							// create a new quad mesh. this is what we will apply the
							// texture to
							let quad_width = BLOCK;
							let quad_handle = meshes.add(Mesh::from(shape::Quad::new(
								Vec2::new(quad_width, quad_width * aspect),
							)));

							// this material renders the texture normally
							let material_handle = materials.add(StandardMaterial {
								base_color_texture: Some(texture_handle.clone()),
								alpha_mode: AlphaMode::Blend,
								unlit: true, // !block_events.2.inserted_pic,
								..default()
							});

							use std::f32::consts::PI;
							// textured quad - normal
							let rot =
								Quat::from_euler(EulerRot::XYZ, -PI / 2., -PI, PI / 2.); // to_radians()
												// let mut rot = Quat::from_rotation_x(-std::f32::consts::PI
												// / 2.0);
							let transform = Transform {
								translation: Vec3::new(
									-7., // + (BLOCK_AND_SPACER * block_num as f32),
									0.1, //1.5
									0.,  //(BLOCK_AND_SPACER * chain as f32) * rflip,
								),
								rotation: rot,
								..default()
							};

							parent
								.spawn_bundle(PbrBundle {
									mesh: quad_handle.clone(),
									material: material_handle.clone(),

									transform,
									..default()
								})
								.insert(Name::new("BillboardDown"))
								.insert(ClearMe);
							// by adding Details to the banners they are cleared down
							// when we remove every entity with Details.

							// create a new quad mesh. this is what we will apply the
							// texture to
							let quad_width = BLOCK;
							let quad_handle = meshes.add(Mesh::from(shape::Quad::new(
								Vec2::new(quad_width, quad_width * aspect),
							)));

							// this material renders the texture normally
							let material_handle = materials.add(StandardMaterial {
								base_color_texture: Some(texture_handle.clone()),
								alpha_mode: AlphaMode::Blend,
								unlit: true,
								..default()
							});

							// textured quad - normal
							let rot =
								Quat::from_euler(EulerRot::XYZ, -PI / 2., 0., -PI / 2.); // to_radians()
												// let mut rot = Quat::from_rotation_x(-std::f32::consts::PI
												// / 2.0);
							let transform = Transform {
								translation: Vec3::new(
									-7., // + (BLOCK_AND_SPACER * block_num as f32),
									0.1, //1.5
									0.,  //(BLOCK_AND_SPACER  as f32) * rflip,
								),
								rotation: rot,
								..default()
							};

							parent
								.spawn_bundle(PbrBundle {
									mesh: quad_handle.clone(),
									material: material_handle.clone(),
									transform,
									..default()
								})
								.insert(Name::new("BillboardUp"))
								.insert(ClearMe);
							// .insert(Aabb::from_min_max(
							//     Vec3::new(0., 0., 0.),
							//     Vec3::new(1., 1., 1.),
							// )); // TODO: should be able to add same component onto 3
							// different entities maybe?

							//block_events.2.inserted_pic = true;
						})
						.insert_bundle(PickableBundle::default());
				}

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
					&chain_info,
					block_num,
					fun,
					&mut commands,
					&mut meshes,
					&mut materials,
					BuildDirection::Up,
					&block.blockhash,
					&links,
					&mut polyline_materials,
					&mut polylines,
					&encoded,
				);

				add_blocks(
					&chain_info,
					block_num,
					boring,
					&mut commands,
					&mut meshes,
					&mut materials,
					BuildDirection::Down,
					&block.blockhash,
					&links,
					&mut polyline_materials,
					&mut polylines,
					&encoded,
				);
				event.send(RequestRedraw);
			},
			DataUpdate::NewChain(chain_info) => {
				CHAINS.fetch_add(1, Ordering::Relaxed);
				draw_chain_rect(&chain_info, &mut commands, &mut meshes, &mut materials)
			},

		}
	}
}
		}
	}
}

// TODO allow different block building strategies. maybe dependent upon quantity of blocks in the
// space?
fn add_blocks<'a>(
	chain_info: &ChainInfo,
	block_num: f32,
	block_events: Vec<(Option<DataEntity>, Vec<DataEvent>)>,
	commands: &mut Commands,
	meshes: &mut ResMut<Assets<Mesh>>,
	materials: &mut ResMut<Assets<StandardMaterial>>,
	build_direction: BuildDirection,
	block_hash: &H256,
	links: &Query<(Entity, &MessageSource, &GlobalTransform)>,
	polyline_materials: &mut ResMut<Assets<PolylineMaterial>>,
	polylines: &mut ResMut<Assets<Polyline>>,
	encoded: &str,
) {
	let rflip = chain_info.chain_url.rflip();
	let build_dir = if let BuildDirection::Up = build_direction { 1.0 } else { -1.0 };
	// Add all the useful blocks

	let mesh = meshes.add(Mesh::from(shape::Icosphere { radius: 0.40, subdivisions: 32 }));
	let mesh_xcm = meshes.add(Mesh::from(shape::Torus {
		radius: 0.6,
		ring_radius: 0.4,
		subdivisions_segments: 20,
		subdivisions_sides: 10,
	}));
	let mesh_extrinsic = meshes.add(Mesh::from(shape::Box::new(0.8, 0.8, 0.8)));
	let mut mat_map = HashMap::new();

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

	let hex_block_hash = format!("0x{}", hex::encode(block_hash.as_bytes()));

	for (event_num, (block, events)) in block_events.iter().enumerate() {
		let z = event_num % 9;
		let x = (event_num / 9) % 9;

		rain_height[event_num % 81] += fastrand::f32() * HIGH;

		let (px, py, pz) = (base_x + x as f32, rain_height[event_num % 81], (base_z + z as f32));

		let transform = Transform::from_translation(Vec3::new(px, py * build_dir, pz * rflip));

		if let Some(block @ DataEntity::Extrinsic { .. }) = block {
			for block in std::iter::once(block).chain(block.contains().iter()) {
				EXTRINSICS.fetch_add(1, Ordering::Relaxed);
				let target_y = next_y[event_num % 81];
				next_y[event_num % 81] += DOT_HEIGHT;
				let dark = block.details().doturl.is_darkside();
				let style = style::style_event(&block);
				let material = mat_map.entry(style.clone()).or_insert_with(|| {
					materials.add(if dark {
						StandardMaterial {
							base_color: style.color.clone(),
							emissive: style.color.clone(),
							perceptual_roughness: 0.3,
							metallic: 1.0,
							..default()
						}
					} else {
						style.color.clone().into()
					})
				});
				let mesh = if content::is_message(&block) {
					mesh_xcm.clone()
				} else if matches!(block, DataEntity::Extrinsic { .. }) {
					mesh_extrinsic.clone()
				} else {
					mesh.clone()
				};

				let call_data = format!("0x{}", hex::encode(block.as_bytes()));

				let mut create_source = vec![];
				for (link, _link_type) in block.end_link() {
					//if this id already exists then this is the destination, not the source...
					for (entity, id, source_global) in links.iter() {
						if id.id == *link {
							// println!("creating rainbow!");

							let mut vertices = vec![
								source_global.translation,
								Vec3::new(px, base_y + target_y * build_dir, pz * rflip),
							];
							rainbow(&mut vertices, 50);

							let colors = vec![
								Color::PURPLE,
								Color::BLUE,
								Color::CYAN,
								Color::YELLOW,
								Color::RED,
							];
							for color in colors.into_iter() {
								// Create rainbow from entity to current extrinsic location.
								commands
									.spawn_bundle(PolylineBundle {
										polyline: polylines.add(Polyline {
											vertices: vertices.clone(),
											..Default::default()
										}),
										material: polyline_materials.add(PolylineMaterial {
											width: 10.0,
											color,
											perspective: true,
											..Default::default()
										}),
										..Default::default()
									})
									.insert(ClearMe);

								for v in vertices.iter_mut() {
									v.y += 0.5;
								}
							}

							commands.entity(entity).remove::<MessageSource>();
						}
					}
				}

				for (link, link_type) in block.start_link() {
					// println!("inserting source of rainbow!");
					create_source
						.push(MessageSource { id: link.to_string(), link_type: *link_type });
				}

				let mut bun = commands.spawn_bundle(PbrBundle {
					mesh,
					/// * event.blocknum as f32
					material: material.clone(),
					transform,
					..Default::default()
				});

				bun.insert_bundle(PickableBundle::default())
					.insert(Details {
						// hover: format_entity(block),
						// data: (block).clone(),http://192.168.1.241:3000/#/extrinsics/decode?calldata=0
						doturl: block.dot().clone(),
						flattern: block.details().flattern.clone(),
						url: format!(
							"https://polkadot.js.org/apps/?{}#/extrinsics/decode/{}",
							&encoded, &call_data
						),
						parent: None,
						success: ui::details::Success::Happy,
						pallet: block.pallet().to_string(),
						variant: block.variant().to_string(),
					})
					.insert(ClearMe)
					.insert(Rainable { dest: base_y + target_y * build_dir, build_direction })
					.insert(Name::new("Extrinsic"))
					.insert(MedFi);
				// .insert(Aabb::from_min_max(
				//     Vec3::new(0., 0., 0.),
				//     Vec3::new(1., 1., 1.),
				// ));

				for source in create_source {
					bun.insert(source);
				}
			}
		}

		let mut events = events.clone();
		events.sort_unstable_by_key(|e| e.details.pallet.to_string() + &e.details.variant);
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
		for event in events {
			EVENTS.fetch_add(1, Ordering::Relaxed);
			let details = Details {
				url: format!(
					"https://polkadot.js.org/apps/?{}#/explorer/query/{}",
					&encoded, &hex_block_hash
				),
				..event.details.clone()
			};
			let dark = details.doturl.is_darkside();
			let entity = DataEvent { details, ..event.clone() };
			let style = style::style_data_event(&entity);
			//TODO: map should be a resource.
			let material = mat_map.entry(style.clone()).or_insert_with(|| {
				materials.add(if dark {
					StandardMaterial {
						base_color: style.color.clone(),
						emissive: style.color.clone(),
						perceptual_roughness: 0.3,
						metallic: 1.0,
						..default()
					}
				} else {
					style.color.clone().into()
				})
			});

			let mesh = if content::is_event_message(&entity) {
				mesh_xcm.clone()
			} else {
				// let mesh = meshes.add(Mesh::from(shape::Box {
				//     min_x: 0.,
				//     max_x: 0.8,

				//     min_y: 0.,
				//     max_y: 0.8 * height + (height - 1.) * 0.4,

				//     min_z: 0.,
				//     max_z: 0.8,
				// }));
				// let mesh = meshes.add(Mesh::from(shape::Icosphere {
				//     radius: 0.40,
				//     subdivisions: 32,
				// }));
				mesh.clone()
			};
			rain_height[event_num % 81] += DOT_HEIGHT; // * height;
			let target_y = next_y[event_num % 81];
			next_y[event_num % 81] += DOT_HEIGHT; // * height;

			let t = Transform::from_translation(Vec3::new(
				px,
				rain_height[event_num % 81] * build_dir,
				pz * rflip,
			));

			let mut x = commands.spawn_bundle(PbrBundle {
				mesh,
				material: material.clone(),
				transform: t,
				..Default::default()
			});
			let event_bun = x
				.insert_bundle(PickableBundle::default())
				.insert(entity.details.clone())
				.insert(Rainable { dest: base_y + target_y * build_dir, build_direction })
				.insert(Name::new("BlockEvent"))
				.insert(ClearMe)
				.insert(HiFi);
			// .insert(Aabb::from_min_max(
			//     Vec3::new(0., 0., 0.),
			//     Vec3::new(1., 1., 1.),
			// ));

			for (link, link_type) in &event.start_link {
				// println!("inserting source of rainbow (an event)!");
				event_bun.insert(MessageSource { id: link.to_string(), link_type: *link_type });
			}
		}
		// }
	}
}

/// Yes this is now a verb. Who knew?
fn rainbow(vertices: &mut Vec<Vec3>, points: usize) {
	use std::f32::consts::PI;
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
		vertices.push(Vec3::new(x, y, z));
	}
}

#[derive(Component)]
pub struct Rainable {
	dest: f32,
	build_direction: BuildDirection,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

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

pub fn rain(
	time: Res<Time>,
	mut commands: Commands,
	mut drops: Query<(Entity, &mut Transform, &Rainable)>,
	mut timer: ResMut<UpdateTimer>,
) {
	//TODO: remove the Rainable component once it has landed for performance!
	let delta = 1.;
	if timer.timer.tick(time.delta()).just_finished() {
		for (entity, mut transform, rainable) in drops.iter_mut() {
			if let BuildDirection::Up = rainable.build_direction {
				if transform.translation.y > rainable.dest {
					let todo = transform.translation.y - rainable.dest;
					let delta = min!(1., delta * (todo / rainable.dest));

					transform.translation.y = max!(rainable.dest, transform.translation.y - delta);
					// Stop raining...
					if delta < f32::EPSILON {
						commands.entity(entity).remove::<Rainable>();
					}
				}
			} else {
				// Austrialian down under world. Balls coming up from the depths...
				if transform.translation.y < rainable.dest {
					transform.translation.y = min!(rainable.dest, transform.translation.y + delta);
					// Stop raining...
					if delta < f32::EPSILON {
						commands.entity(entity).remove::<Rainable>();
					}
				}
			}
		}
	}
}

pub struct UpdateTimer {
	timer: Timer,
}
use bevy_egui::EguiContext;
pub fn print_events(
	mut events: EventReader<PickingEvent>,
	mut query2: Query<(Entity, &Details, &GlobalTransform)>,
	mut urlbar: ResMut<ui::UrlBar>,
	mut inspector: ResMut<Inspector>,
	mut custom: EventWriter<DataSourceChangedEvent>,
	mut dest: ResMut<Destination>,
	mut anchor: ResMut<Anchor>,

	// Is egui using the mouse?
	mut egui_context: ResMut<EguiContext>, // TODO: this doesn't need to be mut.
) {
	let ctx = &mut egui_context.ctx_mut();
	// If we're over an egui area we should not be trying to select anything.
	if ctx.is_pointer_over_area() {
		return
	}
	if urlbar.changed() {
		urlbar.reset_changed();
		let timestamp = urlbar.timestamp();

		custom.send(DataSourceChangedEvent { source: urlbar.location.clone(), timestamp });
	}
	for event in events.iter() {
		match event {
			PickingEvent::Selection(selection) => {
				if let SelectionEvent::JustSelected(_entity) = selection {
					//  let mut inspector_window_data = inspector_windows.window_data::<Details>();
					//   let window_size =
					// &world.get_resource::<ExtractedWindowSizes>().unwrap().0[&self.window_id];

					// let selection = query.get_mut(*entity).unwrap();

					// Unspawn the previous text:
					// query3.for_each_mut(|(entity, _)| {
					//     commands.entity(entity).despawn();
					// });

					// if inspector.active == Some(details) {
					//     print!("deselected current selection");
					//     inspector.active = None;
					// } else {

					// }

					// info!("{}", details.hover.as_str());
					// decode_ex!(events, crate::polkadot::ump::events::UpwardMessagesReceived,
					// value, details);
				}
			},
			PickingEvent::Hover(e) => {
				// info!("Egads! A hover event!? {:?}", e)

				match e {
					HoverEvent::JustEntered(entity) => {
						let (_entity, details, _global_location) = query2.get_mut(*entity).unwrap();
						inspector.hovered = Some(if details.doturl.extrinsic.is_some() {
							format!("{} - {} ({})", details.doturl, details.variant, details.pallet)
						} else {
							format!("{}", details.doturl)
						});
					},
					HoverEvent::JustLeft(_) => {
						//	inspector.hovered = None;
					},
				}
			},
			PickingEvent::Clicked(entity) => {
				let now = Utc::now().timestamp_millis() as i32;
				let prev = LAST_CLICK_TIME.swap(now as i32, Ordering::Relaxed);
				let (_entity, details, global_location) = query2.get_mut(*entity).unwrap();
				if let Some(selected) = &inspector.selected {
					if selected.doturl == details.doturl && !(now - prev < 400) {
						inspector.selected = None;
						return
					}
				}
				inspector.selected = Some(details.clone());
				inspector.texture = None;

				// info!("Gee Willikers, it's a click! {:?}", e)

				// use async_std::task::block_on;
				// 				use serde_json::json;
				// 				let metad = block_on(datasource::get_desub_metadata(&url, &mut source,
				// None)).unwrap(); 				if let Ok(extrinsic) =
				// 					decoder::decode_unwrapped_extrinsic(&metad, &mut details.raw.as_slice())
				// 				{
				// 					println!("{:#?}", extrinsic);
				// 				} else {
				// 					println!("could not decode.");
				// 				}
				// 				serde_json::to_value(&value);

				if now - prev < 400 {
					println!("double click {}", now - prev);
					// if you double clicked on just a chain then you really don't want to get sent
					// to the middle of nowhere!
					if details.doturl.block_number.is_some() {
						println!("double clicked to {}", details.doturl);
						anchor.follow_chain = false; // otherwise when we get to the destination then we will start moving away
							 // from it.
						dest.location = Some(global_location.translation);
					}
				}
			},
		}
	}
}

struct Width(f32);

static LAST_CLICK_TIME: AtomicI32 = AtomicI32::new(0);
static LAST_KEYSTROKE_TIME: AtomicI32 = AtomicI32::new(0);

fn update_visibility(
	mut entity_low_midfi: Query<(
		&mut Visibility,
		&GlobalTransform,
		With<ClearMe>,
		Without<HiFi>,
		Without<MedFi>,
	)>,
	mut entity_medfi: Query<(&mut Visibility, &GlobalTransform, With<MedFi>, Without<HiFi>)>,
	mut entity_hifi: Query<(&mut Visibility, &GlobalTransform, With<HiFi>, Without<MedFi>)>,
	player_query: Query<&Transform, With<Viewport>>,
	#[cfg(feature = "adaptive-fps")] diagnostics: Res<'_, Diagnostics>,
	#[cfg(feature = "adaptive-fps")] mut visible_width: ResMut<Width>,
	#[cfg(not(feature = "adaptive-fps"))] visible_width: Res<Width>,
) {
	// TODO: have a lofi zone and switch visibility of the lofi and hifi entities

	let transform: &Transform = player_query.get_single().unwrap();
	let x = transform.translation.x;
	let y = transform.translation.y;

	// If nothing's visible because we're far away make a few things visible so you know which dir
	// to go in and can double click to get there...
	#[cfg(feature = "adaptive-fps")]
	if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
		let min = diag.history_len();
		if let Some(avg) = diag.values().map(|&i| i as u32).min() {
			println!("avg {}\t{}", avg, visible_width.0);
			let target = 30.;
			let avg = avg as f32;
			if avg < target && visible_width.0 > 100. {
				visible_width.0 -= (target - avg) / 4.;
			}
			// Because of frame rate differences it will go up much faster than it will go down!
			else if avg > target && visible_width.0 < 1000. {
				visible_width.0 += (avg - target) / 32.;
			}
		}
	}

	let width = visible_width.0;
	let (min, max) = (x - width, x + width);

	let mut vis_count = 0;
	for (mut vis, transform, _, _, _) in entity_low_midfi.iter_mut() {
		vis.is_visible = transform.translation.x > min && transform.translation.x < max;
		if vis.is_visible {
			vis_count += 1;
		}
	}
	for (mut vis, transform, _, _) in entity_hifi.iter_mut() {
		vis.is_visible = transform.translation.x > min && transform.translation.x < max;
		if y > 500. {
			vis.is_visible = false;
		}
	}
	for (mut vis, transform, _, _) in entity_medfi.iter_mut() {
		vis.is_visible = transform.translation.x > min && transform.translation.x < max;
		if y > 800. {
			vis.is_visible = false;
		}
	}

	if vis_count == 0 {
		for (mut vis, _, _, _, _) in entity_low_midfi.iter_mut().take(1000) {
			vis.is_visible = true;
		}
	}

	// println!("viewport x = {},    {}  of   {} ", x, count_vis, count);
}

pub fn right_click_system(
	mouse_button_input: Res<Input<MouseButton>>,
	touches_input: Res<Touches>,
	// hover_query: Query<
	//     (Entity, &Hover, ChangeTrackers<Hover>),
	//     (Changed<Hover>, With<PickableMesh>),
	// >,
	// selection_query: Query<
	//     (Entity, &Selection, ChangeTrackers<Selection>),
	//     (Changed<Selection>, With<PickableMesh>),
	// >,
	query_details: Query<&Details>,
	click_query: Query<(Entity, &Hover)>,
) {
	if mouse_button_input.just_pressed(MouseButton::Right) ||
		touches_input.iter_just_pressed().next().is_some()
	{
		for (entity, hover) in click_query.iter() {
			if hover.hovered() {
				// Open browser.
				let details = query_details.get(entity).unwrap();
				#[cfg(not(target_arch = "wasm32"))]
				open::that(&details.url).unwrap();
				// picking_events.send(PickingEvent::Clicked(entity));
			}
		}
	}
}

/// set up a simple 3D scene
fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	// asset_server: Res<AssetServer>,
	mut datasource_events: EventWriter<DataSourceChangedEvent>,
) {
	// add entities to the world
	// plane

	commands.spawn_bundle(PbrBundle {
		mesh: meshes.add(Mesh::from(shape::Box::new(50000., 0.1, 50000.))),
		material: materials.add(
			StandardMaterial {
				base_color: Color::rgba(0.2, 0.2, 0.2, 0.3),
				alpha_mode: AlphaMode::Blend,
				perceptual_roughness: 0.08,
				..default()
			}, //    Color::rgb(0.5, 0.5, 0.5).into()
		),
		transform: Transform { translation: Vec3::new(0., 0., -25000.), ..default() },
		..default()
	});
	commands.spawn_bundle(PbrBundle {
		mesh: meshes.add(Mesh::from(shape::Box::new(50000., 0.1, 50000.))),
		material: materials.add(
			StandardMaterial {
				base_color: Color::rgba(0.2, 0.2, 0.2, 0.3),
				alpha_mode: AlphaMode::Blend,
				perceptual_roughness: 0.08,
				unlit: true,
				..default()
			}, //    Color::rgb(0.5, 0.5, 0.5).into()
		),
		transform: Transform { translation: Vec3::new(0., 0., 25000.), ..default() },
		..default()
	});

	//somehow this can change the color
	//    mesh_highlighting(None, None, None);
	// camera
	let camera_transform =
		Transform::from_xyz(200.0, 50., 0.0).looking_at(Vec3::new(-1000., 1., 0.), Vec3::Y);
	commands.insert_resource(ui::OriginalCameraTransform(camera_transform));
	let mut entity_comands = commands.spawn_bundle(PerspectiveCameraBundle {
		transform: camera_transform,

		perspective_projection: PerspectiveProjection {
			// far: 1., // 1000 will be 100 blocks that you can s
			far: 10.,
			near: 0.000001,
			..default()
		},
		camera: Camera { far: 10., near: 0.000001, ..default() },
		..default()
	});
	#[cfg(feature = "normalmouse")]
	entity_comands.insert(FlyCam);
	entity_comands
		.insert(Viewport)
		.insert_bundle(PickingCameraBundle { ..default() });

	// #[cfg(feature = "spacemouse")]
	// entity_comands.insert(SpaceMouseRelativeControllable);

	commands.insert_resource(UpdateTimer { timer: Timer::new(Duration::from_millis(50), true) });

	// light

	commands.insert_resource(AmbientLight { color: Color::WHITE, brightness: 0.9 });

	// commands.spawn_bundle(PointLightBundle {
	//     transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
	//     ..Default::default()
	// });
	commands.spawn_bundle(PointLightBundle {
		transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
		..Default::default()
	});

	// Kick off the live mode automatically so people have something to look at
	datasource_events.send(DataSourceChangedEvent {
		source: "dotsama:/1//10504599".to_string(), // LIVE.to_string(),
		timestamp: None,
	});
}

#[derive(Default)]
pub struct Inspector {
	// #[inspectable(deletable = false)]
	// #[inspectable(collapse)]
	// start_location: UrlBar,
	// timestamp: DateTime,
	// #[inspectable(deletable = false)]
	selected: Option<Details>,

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

#[derive(Component)]
pub struct Viewport;

// #[cfg(target_arch = "wasm32")]
// pub mod html_body {
//     use web_sys::HtmlElement;

//     pub fn get() -> HtmlElement {
//         // From https://www.webassemblyman.com/rustwasm/how_to_add_mouse_events_in_rust_webassembly.html
//         let window = web_sys::window().expect("no global `window` exists");
//         let document = window.document().expect("should have a document on window");
//         let body = document.body().expect("document should have a body");
//         body
//     }
// }
