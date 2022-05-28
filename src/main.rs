#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(slice_pattern)]
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
// use bevy::winit::WinitSettings;
use bevy_ecs::prelude::Component;
#[cfg(feature = "normalmouse")]
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};

use bevy_inspector_egui::{Inspectable, InspectorPlugin};
use bevy_mod_picking::*;
//use bevy_egui::render_systems::ExtractedWindowSizes;
use bevy_polyline::{prelude::*, PolylinePlugin};
use color_eyre::owo_colors::OwoColorize;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
mod content;
mod datasource;
mod movement;
mod style;
mod ui;
use crate::ui::{Details, DotUrl};
use bevy_inspector_egui::RegisterInspectable;
#[cfg(feature = "spacemouse")]
use bevy_spacemouse::{SpaceMousePlugin, SpaceMouseRelativeControllable};
use sp_core::H256;
use std::convert::AsRef;

// #[subxt::subxt(runtime_metadata_path = "wss://kusama-rpc.polkadot.io:443")]
// pub mod polkadot {}
#[subxt::subxt(runtime_metadata_path = "polkadot_metadata.scale")]
pub mod polkadot {}

#[cfg(feature = "spacemouse")]
pub struct MovementSettings {
    pub sensitivity: f32,
    pub speed: f32,
}

#[cfg(feature = "spacemouse")]
impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.00012,
            speed: 12.,
        }
    }
}

static RELAY_BLOCKS: AtomicU32 = AtomicU32::new(0);
static RELAY_BLOCKS2: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
pub struct ChainInfo {
    pub chain_name: String,
    pub chain_ws: String,
    pub chain_id: Option<NonZeroU32>,
    pub inserted_pic: bool,
}

// Wait in hashmap till both events and extrinsics together, then released into queue:
type ABlocks = Arc<
    Mutex<(
        HashMap<String, datasource::PolkaBlock>,
        Vec<datasource::PolkaBlock>,
        ChainInfo,
    )>,
>;
use crossbeam_channel::unbounded;

mod networks;
use color_eyre::eyre::Result;
use networks::Env;

pub struct DataSourceChangedEvent {
    source: String,
}

#[async_std::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins);
    //  .insert_resource(WinitSettings::desktop_app()) - this messes up the 3d space mouse?
    app.add_event::<DataSourceChangedEvent>();
    app.insert_resource(MovementSettings {
        sensitivity: 0.00020, // default: 0.00012
        speed: 12.0,          // default: 12.0
    });

    app.insert_resource(Sovereigns { relays: vec![] });

    #[cfg(feature = "normalmouse")]
    app.add_plugin(NoCameraPlayerPlugin);
    app.insert_resource(movement::MouseCapture::default());

    #[cfg(feature = "spacemouse")]
    app.add_plugin(SpaceMousePlugin);

    app.add_plugins(DefaultPickingPlugins)
        // .add_plugin(DebugCursorPickingPlugin) // <- Adds the green debug cursor.
        .add_plugin(InspectorPlugin::<Inspector>::new())
        .register_inspectable::<Details>()
        // .add_plugin(DebugEventsPickingPlugin)
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(LogDiagnosticsPlugin::default())
        // .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(PolylinePlugin)
        // .add_system(movement::scroll)
        .add_startup_system(
            move |commands: Commands,
                  meshes: ResMut<Assets<Mesh>>,
                  materials: ResMut<Assets<StandardMaterial>>| {
                // let clone_chains_for_lanes = clone_chains_for_lanes.clone();
                setup(commands, meshes, materials);
            },
        );
    #[cfg(feature = "spacemouse")]
    app.add_startup_system(move |mut scale: ResMut<bevy_spacemouse::Scale>| {
        scale.rotate_scale = 0.00010;
        scale.translate_scale = 0.004;
    });
    app.add_system(movement::player_move_arrows)
        .add_system(rain)
        .add_system(source_data)
        .add_system(right_click_system)
        .add_startup_system(ui::details::configure_visuals)
        .insert_resource(bevy_atmosphere::AtmosphereMat::default()) // Default Earth sky
        .add_plugin(bevy_atmosphere::AtmospherePlugin {
            dynamic: true, // Set to false since we aren't changing the sky's appearance
            sky_radius: 10.0,
        })
        .add_system(
            render_new_events, // move |commands: Commands,
                               //       meshes: ResMut<Assets<Mesh>>,
                               //       materials: ResMut<Assets<StandardMaterial>>,
                               //       asset_server: Res<AssetServer>,
                               //       links: Query<(Entity, &MessageSource, &GlobalTransform)>,
                               //       polyline_materials: ResMut<Assets<PolylineMaterial>>,
                               //       polylines: ResMut<Assets<Polyline>>| {
                               //     // let clone_chains = clone_chains.clone();
                               //     render_new_events(
                               //         commands,
                               //         meshes,
                               //         materials,
                               //         clone_chains,
                               //         asset_server,
                               //         is_self_sovereign,
                               //         links,
                               //         polyline_materials,
                               //         polylines,
                               //     )
                               // },
        )
        .add_system_to_stage(CoreStage::PostUpdate, print_events);

    app.run();

    // app.insert_resource(GreetTimer(Timer::from_seconds(2.0, true)))
    // .add_startup_system(add_people)
    // .add_system(greet_people);
    Ok(())
}

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

fn source_data(
    mut datasouce_events: EventReader<DataSourceChangedEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sovereigns: ResMut<Sovereigns>,
    details: Query<Entity, With<Details>>,
) {
    for event in datasouce_events.iter() {
        println!("data source chaneg to {}", event.source);

        // Clear

        // 1. Stop existing event threads!
        // 2. remove all entities.
        for detail in details.iter() {
            commands.entity(detail).despawn();
            // detail.despawn();
        }

        if event.source == "" {
            return;
        }
        let dot_url = DotUrl::parse(&event.source).unwrap_or(DotUrl::default());
        let selected_env = &dot_url.env; //if std::env::args().next().is_some() { Env::Test } else {Env::Prod};
        println!("dot url {:?}", &dot_url);
        let as_of = dot_url.block_number;
        println!("Block number selected for relay chains: {:?}", as_of);
        // let mut as_of = Some("10000000");

        // if let Env::Local = selected_env {
        // as_of = Some("10000000"); // If local show from the first block...
        // }

        let relays = networks::get_network(&selected_env);
        // let is_self_sovereign = selected_env.is_self_sovereign();
        let relays = relays
            .into_iter()
            .map(|relay| {
                relay
                    .iter()
                    .map(|chain_name| (ABlocks::default(), chain_name.to_string()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let clone_chains = relays.clone();
        let clone_chains_for_lanes = relays.clone();

        sovereigns.relays = clone_chains;

        for (rcount, chains) in clone_chains_for_lanes.iter().enumerate() {
            let rfip = if rcount == 1 { -1. } else { 1. };

            for (chain, _chain) in chains.iter().enumerate() {
                commands.spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box::new(10000., 0.1, 10.))),
                    material: materials.add(
                        StandardMaterial {
                            base_color: Color::rgba(0., 0., 0., 0.4),
                            alpha_mode: AlphaMode::Blend,
                            perceptual_roughness: 0.08,
                            ..default()
                        }, //    Color::rgb(0.5, 0.5, 0.5).into()
                    ),
                    transform: Transform::from_translation(Vec3::new(
                        (10000. / 2.) - 5.,
                        0.,
                        ((RELAY_CHAIN_CHASM_WIDTH - 5.)
                            + (BLOCK / 2. + BLOCK_AND_SPACER * chain as f32))
                            * rfip,
                    )),
                    ..Default::default()
                });
            }
        }

        for (relay_id, relay) in relays.into_iter().enumerate() {
            let relay_url = DotUrl {
                sovereign: Some(relay_id as u32),
                ..dot_url.clone()
            };
            let mut relay2: Vec<_> = vec![];
            let mut send_map: HashMap<
                NonZeroU32,
                crossbeam_channel::Sender<(datasource::RelayBlockNumber, H256)>,
            > = Default::default();
            for (arc, chain_name) in relay {
                let url = chain_name_to_url(&chain_name);
                // let mut client = ClientBuilder::new().set_url(&url).build().await.unwrap();
                let para_id: Option<NonZeroU32> = datasource::get_parachain_id_from_url(&url);
                let (tx, rc) = unbounded();
                relay2.push((arc, chain_name, para_id, rc));
                if let Some(para_id) = para_id {
                    send_map.insert(para_id, tx);
                }
            }

            let relay = relay2;
            let mut send_map = Some(send_map); // take by only one.

            for (arc, chain_name, para_id, rc) in relay {
                let lock_clone = arc.clone();
                let url = chain_name_to_url(&chain_name);
                println!("url attaching to {}", url);

                // let chain_name_clone = chain_name.clone();
                let url_clone = url.clone();
                let maybe_sender = if para_id.is_none() {
                    send_map.take()
                } else {
                    None
                };

                // events are requested with blocks if not live.
                if as_of.is_none() {
                    let relay_url_clone = relay_url.clone();
                    std::thread::spawn(move || {
                        // let mut reconnects = 0;

                        // while reconnects < 20 {
                        println!("Connecting to {}", &url);
                        let res = async_std::task::block_on(datasource::watch_events(
                            lock_clone.clone(),
                            &url,
                            as_of,
                            DotUrl {
                                para_id,
                                ..relay_url_clone
                            },
                            // /aybe_sender
                        ));
                        // if res.is_ok() { break; }
                        // println!("Problem with {} events (retrys left {})", &url, reconnects);
                        // std::thread::sleep(std::time::Duration::from_secs(20));
                        // reconnects += 1;
                        // }
                        println!("giving up on {} for live events", url);
                    });
                }

                // let chain_name = chain_name_clone;
                let lock_clone = arc.clone();
                let relay_url_clone = relay_url.clone();
                std::thread::spawn(move || {
                    // let mut reconnects = 0;

                    // while reconnects < 20 {
                    println!("Connecting to {}", &url_clone);
                    let res = async_std::task::block_on(datasource::watch_blocks(
                        lock_clone.clone(),
                        url_clone.clone(),
                        as_of,
                        DotUrl {
                            para_id,
                            ..relay_url_clone
                        },
                        rc,
                        maybe_sender,
                    ));
                    // if res.is_ok() { break; }
                    // println!(
                    //     "Problem with {} blocks (retries left {})",
                    //     &url_clone, reconnects
                    // );
                    // std::thread::sleep(std::time::Duration::from_secs(20));
                    // reconnects += 1;
                    // }
                    println!("giving up on {} blocks", url_clone);
                });
            }
        }
    }
}

// use bevy_hanabi::AccelModifier;
// use bevy_hanabi::ColorOverLifetimeModifier;
// use bevy_hanabi::EffectAsset;
// use bevy_hanabi::Gradient;
// use bevy_hanabi::ParticleEffect;
// use bevy_hanabi::PositionSphereModifier;
// use bevy_hanabi::Spawner;

// fn setup_particles(mut effects: ResMut<Assets<EffectAsset>>) {
//     // Define a color gradient from red to transparent black
//     let mut gradient = Gradient::new();
//     gradient.add_key(0.0, Vec4::new(1., 0., 0., 1.));
//     gradient.add_key(1.0, Vec4::splat(0.));

//     // Create the effect asset
//     let effect = effects.add(
//         EffectAsset {
//             name: "MyEffect".to_string(),
//             // Maximum number of particles alive at a time
//             capacity: 32768,
//             // Spawn at a rate of 5 particles per second
//             spawner: Spawner::rate(5.0.into()),
//             ..Default::default()
//         }
//         // On spawn, randomly initialize the position and velocity
//         // of the particle over a sphere of radius 2 units, with a
//         // radial initial velocity of 6 units/sec away from the
//         // sphere center.
//         .init(PositionSphereModifier {
//             center: Vec3::ZERO,
//             radius: 2.,
//             dimension: ShapeDimension::Surface,
//             speed: 6.0.into(),
//         })
//         // Every frame, add a gravity-like acceleration downward
//         .update(AccelModifier {
//             accel: Vec3::new(0., -3., 0.),
//         })
//         // Render the particles with a color gradient over their
//         // lifetime.
//         .render(ColorOverLifetimeModifier { gradient }),
//     );
// }

enum BuildDirection {
    Up,
    Down,
}

// fn focus_manager(mut windows: ResMut<Windows>, //toggle_mouse_capture: Res<movement::MouseCapture>
// ) {
//     // let window = windows.get_primary_mut().unwrap();
//     // if window.is_focused() {
//     //     window.set_cursor_lock_mode(toggle_mouse_capture.0);
//     // } else {
//     //     window.set_cursor_lock_mode(false);
//     // }
// }

fn format_entity(chain_name: &str, entity: &DataEntity) -> String {
    let res = match entity {
        DataEntity::Event(DataEvent { details, .. }) => {
            format!("{:#?}", details)
        }
        DataEntity::Extrinsic {
            // id: _,
            args,
            contains,
            details,
            ..
        } => {
            let kids = if contains.is_empty() {
                String::new()
            } else {
                format!(" contains {} extrinsics", contains.len())
            };
            format!(
                "{}\n{} {} {}\n{:#?}",
                chain_name, details.pallet, details.variant, kids, args
            )
        }
    };

    // if let Some(pos) = res.find("data: Bytes(") {
    //     res.truncate(pos + "data: Bytes(".len());
    //     res.push_str("...");
    // }
    res
}

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
        /// psudo-unique id to link to some other node(s).
        /// There can be multiple destinations per block! (TODO: need better resolution)
        /// Is this true of an extrinsic - system ones plus util batch could do multiple msgs.
        start_link: Vec<String>,
        /// list of links that we have finished
        end_link: Vec<String>,
        details: Details,
    },
}

#[derive(Clone)]
pub struct DataEvent {
    // raw: RawEventDetails,
    details: Details,
    start_link: Vec<String>,
    end_link: Vec<String>,
}

/// A tag to identify an entity as being the source of a message.
#[derive(Component)]
pub struct MessageSource {
    /// Currently sending block id + hash of beneficiary address.
    pub id: String,
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

    pub fn start_link(&self) -> &Vec<String> {
        match self {
            Self::Extrinsic { start_link, .. } => &start_link,
            Self::Event(DataEvent { .. }) => &EMPTY_VEC,
        }
    }
    pub fn end_link(&self) -> &Vec<String> {
        match self {
            Self::Extrinsic { end_link, .. } => &end_link,
            Self::Event(DataEvent { .. }) => &EMPTY_VEC,
        }
    }
}

static EMPTY_VEC: Vec<String> = vec![];

const BLOCK: f32 = 10.;
const BLOCK_AND_SPACER: f32 = BLOCK + 4.;
const RELAY_CHAIN_CHASM_WIDTH: f32 = 25.;

struct Sovereigns {
    pub relays: Vec<Vec<(ABlocks, String)>>,
}

fn render_new_events(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // asset_server: Res<AssetServer>,
    relays: Res<Sovereigns>,
    asset_server: Res<AssetServer>,
    // effects: Res<Assets<EffectAsset>>,
    links: Query<(Entity, &MessageSource, &GlobalTransform)>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
) {
    let is_self_sovereign = false; //TODO!
    for (rcount, relay) in relays.relays.iter().enumerate() {
        for (chain, (lock, _chain_name)) in relay.iter().enumerate() {
            if let Ok(ref mut block_events) = lock.try_lock() {
                if let Some(block) = (*block_events).1.pop() {
                    let block_num = if is_self_sovereign {
                        block.blocknum as u32
                    } else {
                        if rcount == 0 {
                            if chain == 0 {
                                //relay
                                RELAY_BLOCKS.store(
                                    RELAY_BLOCKS.load(Ordering::Relaxed) + 1,
                                    Ordering::Relaxed,
                                );
                            }
                            RELAY_BLOCKS.load(Ordering::Relaxed)
                        } else {
                            if chain == 0 {
                                //relay
                                RELAY_BLOCKS2.store(
                                    RELAY_BLOCKS2.load(Ordering::Relaxed) + 1,
                                    Ordering::Relaxed,
                                );
                            }
                            RELAY_BLOCKS2.load(Ordering::Relaxed)
                        }
                    };

                    let rflip = if rcount == 1 { -1.0 } else { 1.0 };

                    // Add the new block as a large rectangle on the ground:
                    commands.spawn_bundle(PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Box::new(10., 0.1, 10.))),
                        material: materials.add(StandardMaterial {
                            base_color: Color::rgba(0., 0., 0., 0.7),
                            alpha_mode: AlphaMode::Blend,
                            perceptual_roughness: 0.08,
                            ..default()
                        }),
                        transform: Transform::from_translation(Vec3::new(
                            0. + (BLOCK_AND_SPACER * block_num as f32),
                            0.,
                            (RELAY_CHAIN_CHASM_WIDTH + BLOCK_AND_SPACER * chain as f32) * rflip,
                        )),
                        ..Default::default()
                    });

                    // if !block_events.2.inserted_pic
                    {
                        let name = (*block_events)
                            .2
                            .chain_name
                            .replace(" ", "-")
                            .replace("-Testnet", "");
                        let texture_handle = asset_server.load(&format!("branding/{}.jpeg", name));
                        let aspect = 1. / 3.;

                        // create a new quad mesh. this is what we will apply the texture to
                        let quad_width = BLOCK;
                        let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
                            quad_width,
                            quad_width * aspect,
                        ))));

                        // this material renders the texture normally
                        let material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(texture_handle.clone()),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true, // !block_events.2.inserted_pic,
                            ..default()
                        });

                        use std::f32::consts::PI;
                        // textured quad - normal
                        let rot = Quat::from_euler(EulerRot::XYZ, -PI / 2., -PI, PI / 2.); // to_radians()
                                                                                           // let mut rot = Quat::from_rotation_x(-std::f32::consts::PI / 2.0);
                        let transform = Transform {
                            translation: Vec3::new(
                                -7. + (BLOCK_AND_SPACER * block_num as f32),
                                0.1, //1.5
                                (RELAY_CHAIN_CHASM_WIDTH + BLOCK_AND_SPACER * chain as f32) * rflip,
                            ),
                            rotation: rot,
                            ..default()
                        };

                        commands
                            .spawn_bundle(PbrBundle {
                                mesh: quad_handle.clone(),
                                material: material_handle.clone(),
                                transform,
                                ..default()
                            })
                            .insert(Name::new("BillboardDown"));

                        // create a new quad mesh. this is what we will apply the texture to
                        let quad_width = BLOCK;
                        let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
                            quad_width,
                            quad_width * aspect,
                        ))));

                        // this material renders the texture normally
                        let material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(texture_handle.clone()),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true, // !block_events.2.inserted_pic,
                            ..default()
                        });

                        // textured quad - normal
                        let rot = Quat::from_euler(EulerRot::XYZ, -PI / 2., 0., -PI / 2.); // to_radians()
                                                                                           // let mut rot = Quat::from_rotation_x(-std::f32::consts::PI / 2.0);
                        let transform = Transform {
                            translation: Vec3::new(
                                -7. + (BLOCK_AND_SPACER * block_num as f32),
                                0.1, //1.5
                                (RELAY_CHAIN_CHASM_WIDTH + BLOCK_AND_SPACER * chain as f32) * rflip,
                            ),
                            rotation: rot,
                            ..default()
                        };

                        commands
                            .spawn_bundle(PbrBundle {
                                mesh: quad_handle.clone(),
                                material: material_handle.clone(),
                                transform,
                                ..default()
                            })
                            .insert(Name::new(format!(
                                "BillboardUp {}",
                                block_events.2.chain_name
                            )));

                        block_events.2.inserted_pic = true;
                    }

                    let ext_with_events =
                        datasource::associate_events(block.extrinsics, block.events);

                    let (boring, fun): (Vec<_>, Vec<_>) =
                        ext_with_events.into_iter().partition(|(e, _)| {
                            if let Some(ext) = e {
                                content::is_utility_extrinsic(ext)
                            } else {
                                true
                            }
                        });

                    add_blocks(
                        &block_events.2,
                        block_num,
                        chain,
                        fun,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        BuildDirection::Up,
                        rflip,
                        &block.blockhash,
                        &links,
                        &mut polyline_materials,
                        &mut polylines,
                    );

                    add_blocks(
                        &block_events.2,
                        block_num,
                        chain,
                        boring,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        BuildDirection::Down,
                        rflip,
                        &block.blockhash,
                        &links,
                        &mut polyline_materials,
                        &mut polylines,
                    );
                }
            }
        }
    }

    //                 "Balances" => {
    //                     match event.raw_event.variant.as_str() {
    //                         "Deposit" => {
    //                             use crate::polkadot::balances::events::Deposit;
    //                             use codec::Decode;
    //                             use  bevy::prelude::shape::CapsuleUvProfile;
    //                             let deposit = Deposit::decode(&mut event.raw_event.data.to_vec().as_slice()).unwrap();
    //                             println!("{:?}", deposit);
    //                             //use num_integer::roots::Roots;
}

// TODO allow different block building strateges. maybe dependent upon quanity of blocks in the space?
fn add_blocks<'a>(
    chain_info: &ChainInfo,
    block_num: u32,
    chain: usize,
    block_events: Vec<(Option<DataEntity>, Vec<DataEvent>)>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    build_direction: BuildDirection,
    rflip: f32,
    block_hash: &H256,
    links: &Query<(Entity, &MessageSource, &GlobalTransform)>,
    polyline_materials: &mut ResMut<Assets<PolylineMaterial>>,
    polylines: &mut ResMut<Assets<Polyline>>,
) {
    let build_direction = if let BuildDirection::Up = build_direction {
        1.0
    } else {
        -1.0
    };
    // Add all the useful blocks

    let mesh = meshes.add(Mesh::from(shape::Icosphere {
        radius: 0.40,
        subdivisions: 32,
    }));
    let mesh_xcm = meshes.add(Mesh::from(shape::Torus {
        radius: 0.6,
        ring_radius: 0.4,
        subdivisions_segments: 20,
        subdivisions_sides: 10,
    }));
    let mesh_extrinsic = meshes.add(Mesh::from(shape::Box::new(0.8, 0.8, 0.8)));
    let mut mat_map = HashMap::new();

    let (base_x, base_y, base_z) = (
        0. + (BLOCK_AND_SPACER * block_num as f32) - 4.,
        0.5,
        RELAY_CHAIN_CHASM_WIDTH + BLOCK_AND_SPACER * chain as f32 - 4.,
    );

    const DOT_HEIGHT: f32 = 1.;
    const HIGH: f32 = 100.;
    let mut rain_height: [f32; 81] = [HIGH; 81];
    let mut next_y: [f32; 81] = [base_y; 81]; // Always positive.

    let encoded: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("rpc", &chain_info.chain_ws)
        .finish();

    let hex_block_hash = format!("0x{}", hex::encode(block_hash.as_bytes()));

    for (event_num, (block, events)) in block_events.iter().enumerate() {
        let z = event_num % 9;
        let x = (event_num / 9) % 9;

        rain_height[event_num % 81] += fastrand::f32() * HIGH;

        let (px, py, pz) = (
            base_x + x as f32,
            rain_height[event_num % 81],
            (base_z + z as f32),
        );

        let transform =
            Transform::from_translation(Vec3::new(px, py * build_direction, pz * rflip));

        if let Some(block @ DataEntity::Extrinsic { .. }) = block {
            for block in std::iter::once(block).chain(block.contains().iter()) {
                let target_y = next_y[event_num % 81];
                next_y[event_num % 81] += DOT_HEIGHT;
                let style = style::style_event(&block);
                let material = mat_map
                    .entry(style.clone())
                    .or_insert_with(|| materials.add(style.color.clone().into()));
                let mesh = if content::is_message(&block) {
                    mesh_xcm.clone()
                } else if matches!(block, DataEntity::Extrinsic { .. }) {
                    mesh_extrinsic.clone()
                } else {
                    mesh.clone()
                };

                let call_data = format!("0x{}", hex::encode(block.as_bytes()));

                let mut found = false;

                let mut create_source = vec![];
                for link in block.end_link() {
                    //if this id already exists then this is the destination, not the source...
                    for (entity, id, source_global) in links.iter() {
                        if id.id == *link {
                            found = true;
                            println!("creating rainbow!");

                            let mut vertices = vec![
                                source_global.translation,
                                Vec3::new(px, target_y * build_direction, pz),
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
                                commands.spawn_bundle(PolylineBundle {
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
                                });

                                for v in vertices.iter_mut() {
                                    v.y += 0.5;
                                }
                            }

                            commands.entity(entity).remove::<MessageSource>();
                        }
                    }
                }
                for link in block.start_link() {
                    println!("inserting source of rainbow!");
                    create_source.push(MessageSource {
                        id: link.to_string(),
                    });
                }

                let mut bun = commands.spawn_bundle(PbrBundle {
                    mesh,
                    ///* event.blocknum as f32
                    material: material.clone(),
                    transform,
                    ..Default::default()
                });

                bun.insert_bundle(PickableBundle::default())
                    .insert(Details {
                        hover: format_entity(&chain_info.chain_name, block),
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
                    .insert(Rainable {
                        dest: target_y * build_direction,
                    })
                    .insert(Name::new("BlockEvent"));

                for source in create_source {
                    bun.insert(source);
                }
            }
        } else {
            // Remove the spacer as we did not add a block.
            // next_y[event_num % 81] -= DOT_HEIGHT;
        }

        for event in events {
            let details = Details {
                // hover: format!("{:#?}", event.raw),
                // flattern: String::new(),
                url: format!(
                    "https://polkadot.js.org/apps/?{}#/explorer/query/{}",
                    &encoded, &hex_block_hash
                ),
                // pallet: event.raw.pallet.to_string(),
                // variant: event.raw.variant.to_string(),
                ..event.details.clone()
            };
            let entity = DataEvent {
                details,
                ..event.clone()
            };
            let style = style::style_data_event(&entity);
            let material = mat_map
                .entry(style.clone())
                .or_insert_with(|| materials.add(style.color.clone().into()));

            let mesh = if content::is_event_message(&entity) {
                mesh_xcm.clone()
            } else {
                mesh.clone()
            };
            rain_height[event_num % 81] += DOT_HEIGHT;
            let target_y = next_y[event_num % 81];
            next_y[event_num % 81] += DOT_HEIGHT;

            let t = Transform::from_translation(Vec3::new(
                px,
                rain_height[event_num % 81] * build_direction,
                pz * rflip,
            ));

            // t.translation.y += ;
            let mut x = commands.spawn_bundle(PbrBundle {
                mesh,
                material: material.clone(),
                transform: t,
                ..Default::default()
            });
            let event_bun = x
                .insert_bundle(PickableBundle::default())
                .insert(entity.details.clone())
                .insert(Rainable {
                    dest: target_y * build_direction,
                })
                .insert(Name::new("BlockEvent"));

            for link in &event.start_link {
                println!("inserting source of rainbow (an event)!");
                event_bun.insert(MessageSource {
                    id: link.to_string(),
                });
            }
        }
    }
}

/// Yes this is now a verb. Who knew?
fn rainbow(vertices: &mut Vec<Vec3>, points: usize) {
    use std::f32::consts::PI;
    let start = vertices[0];
    let end = vertices[1];
    let diff = end - start;
    println!("start {:#?}", start);
    println!("end {:#?}", end);
    println!("diff {:#?}", diff);
    // x, z are linear interpolations, it is only y that goes up!

    let center = (start + end) / 2.;
    println!("center {:#?}", center);
    let r = end - center;
    println!("r {:#?}", r);
    let radius = (r.x * r.x + r.y * r.y + r.z * r.z).sqrt(); // could be aproximate
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
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

// // A unit struct to help identify the color-changing Text component
// #[derive(Component)]
// pub struct ColorText;

// #[derive(]
// struct Data {
//     should_render: bool,
//     text: String,
//     #[inspectable(min = 42.0, max = 100.0)]
//     size: f32,
// }
// macro_rules! decode_ex {
//     ($value:ident, $details:ident, $event:ty) => {
//         if $details.raw.pallet == <$event>::PALLET {
//             if $details.raw.variant == <$event>::EVENT {
//                 // The macro will expand into the contents of this block.
//                 if let Ok(decoded) = <$eventt::decode(&mut $details.raw.data.to_vec().as_slice()) {
//                     $value.push_str(&format!("{:#?}", decoded));
//                 } else {
//                     $value.push_str("(missing metadata to decode)");
//                 }
//             }
//         }
//     };
// }

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
    // world: &mut World,
    mut commands: Commands,
    // mut events: EventReader<PickingEvent>,
    // query: Query<&mut Selection>,
    // mut query2: Query<&mut Details>,
    mut drops: Query<(Entity, &mut Transform, &Rainable)>,
    // asset_server: Res<AssetServer>,
    mut timer: ResMut<UpdateTimer>,
) {
    //TODO: remove the Rainable component once it has landed for performance!
    let delta = 1.;
    if timer.timer.tick(time.delta()).just_finished() {
        for (entity, mut transform, rainable) in drops.iter_mut() {
            if rainable.dest > 0. {
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

pub fn print_events(
    mut events: EventReader<PickingEvent>,
    mut query2: Query<(Entity, &Details)>,
    mut inspector: ResMut<Inspector>,
    mut custom: EventWriter<DataSourceChangedEvent>,
    //  mut inspector_windows: Res<InspectorWindows>,
) {
    if inspector.start_location.changed {
        inspector.start_location.changed = false;

        custom.send(DataSourceChangedEvent {
            source: inspector.start_location.location.clone(),
        });
    }
    for event in events.iter() {
        match event {
            PickingEvent::Selection(selection) => {
                if let SelectionEvent::JustSelected(entity) = selection {
                    //  let mut inspector_window_data = inspector_windows.window_data::<Details>();
                    //   let window_size = &world.get_resource::<ExtractedWindowSizes>().unwrap().0[&self.window_id];

                    // let selection = query.get_mut(*entity).unwrap();

                    // Unspawn the previous text:
                    // query3.for_each_mut(|(entity, _)| {
                    //     commands.entity(entity).despawn();
                    // });

                    let (_entity, details) = query2.get_mut(*entity).unwrap();

                    // if inspector.active == Some(details) {
                    //     print!("deselected current selection");
                    //     inspector.active = None;
                    // } else {
                    inspector.selected = Some(details.clone());
                    // }

                    // info!("{}", details.hover.as_str());
                    // decode_ex!(events, crate::polkadot::ump::events::UpwardMessagesReceived, value, details);
                }
            }
            PickingEvent::Hover(_e) => {
                // info!("Egads! A hover event!? {:?}", e)
            }
            PickingEvent::Clicked(_e) => {
                // info!("Gee Willikers, it's a click! {:?}", e)
            }
        }
    }
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
    if mouse_button_input.just_pressed(MouseButton::Right)
        || touches_input.iter_just_pressed().next().is_some()
    {
        for (entity, hover) in click_query.iter() {
            if hover.hovered() {
                // Open browser.
                let details = query_details.get(entity).unwrap();
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
        ..Default::default()
    });

    //somehow this can change the color
    //    mesh_highlighting(None, None, None);
    // camera

    let mut entity_comands = commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        perspective_projection: PerspectiveProjection {
            // far: 1., // 1000 will be 100 blocks that you can s
            far: 0.0001,
            near: 0.000001,
            ..default()
        },
        camera: Camera {
            far: 0.0001,
            near: 0.000001,

            ..default()
        },
        ..default()
    });
    #[cfg(feature = "normalmouse")]
    entity_comands.insert(FlyCam);
    entity_comands
        .insert(Viewport)
        .insert_bundle(PickingCameraBundle { ..default() });

    #[cfg(feature = "spacemouse")]
    entity_comands.insert(SpaceMouseRelativeControllable);

    use std::time::Duration;
    commands.insert_resource(UpdateTimer {
        timer: Timer::new(Duration::from_millis(50), true),
    });

    // light

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.9,
    });

    // commands.spawn_bundle(PointLightBundle {
    //     transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
    //     ..Default::default()
    // });
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}

#[derive(Inspectable, Default)]
pub struct Inspector {
    // #[inspectable(deletable = false)]
    start_location: UrlBar,
    #[inspectable(deletable = false)]
    selected: Option<Details>,
}

impl Default for UrlBar {
    fn default() -> Self {
        Self {
            location: "dotsama:/1//10000000".to_string(),
            changed: false,
        }
    }
}

struct UrlBar {
    changed: bool,
    location: String,
}
use bevy_inspector_egui::options::StringAttributes;
use bevy_inspector_egui::Context;
use egui::Grid;
impl Inspectable for UrlBar {
    type Attributes = ();

    fn ui(
        &mut self,
        ui: &mut bevy_egui::egui::Ui,
        _options: Self::Attributes,
        context: &mut Context,
    ) -> bool {
        let mut changed = false;
        ui.vertical_centered(|ui| {
            Grid::new(context.id()).min_col_width(400.).show(ui, |ui| {
                // ui.label("Pallet");
                changed |= self
                    .location
                    .ui(ui, StringAttributes { multiline: false }, context);

                ui.end_row();

                if ui.button("Time travel").clicked() {
                    self.changed = true;
                    println!("clicked {}", &self.location);
                };
                ui.end_row();
                if ui.button("Live").clicked() {
                    self.changed = true;
                    self.location = "///".into();
                    println!("clicked {}", &self.location);
                };
                ui.end_row();
                if ui.button("Clear").clicked() {
                    self.changed = true;
                    self.location = "".into();
                    println!("clicked {}", &self.location);
                };
                ui.end_row();
                // ui.label("Method");
                // changed |= self
                //     .variant
                //     .ui(ui, StringAttributes { multiline: false }, context);
                // ui.end_row();
                // changed |= self
                //     .parent.unwrap_or_default()
                //     .ui(ui, NumberAttributes::default(), context);
                // ui.end_row();
                // changed |= self
                //     .hover
                //     .ui(ui, StringAttributes { multiline: true }, context);
                // ui.end_row();
                // changed |= self
                //     .flattern
                //     .ui(ui, StringAttributes { multiline: true }, context);
                // ui.end_row();
                // ui.label("Rotation");
                // changed |= self.rotation.ui(ui, Default::default(), context);
                // self.rotation = self.rotation.normalize();
                // ui.end_row();

                // ui.label("Scale");
                // let scale_attributes = NumberAttributes {
                //     min: Some(Vec3::splat(0.0)),
                //     ..Default::default()
                // };
                // changed |= self.scale.ui(ui, scale_attributes, context);
                // ui.end_row();
            });
        });
        changed
    }
}

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
