#![feature(drain_filter)]
#![feature(hash_drain_filter)]
#![feature(slice_pattern)]
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy_ecs::prelude::Component;
use bevy_flycam::FlyCam;
use bevy_flycam::MovementSettings;
use bevy_flycam::NoCameraPlayerPlugin;
use bevy_inspector_egui::{Inspectable, InspectorPlugin};
use bevy_mod_picking::*;
use bevy_polyline::{prelude::*, PolylinePlugin};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use subxt::RawEventDetails;
mod content;
mod datasource;
mod movement;
mod style;
use crate::details::Details;
use bevy_inspector_egui::RegisterInspectable;
use sp_core::H256;
use std::convert::AsRef;

// #[subxt::subxt(runtime_metadata_path = "wss://kusama-rpc.polkadot.io:443")]
// pub mod polkadot {}
#[subxt::subxt(runtime_metadata_path = "polkadot_metadata.scale")]
pub mod polkadot {}

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

mod networks;
use networks::Env;

mod details;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let selected_env = Env::Prod; //if std::env::args().next().is_some() { Env::Test } else {Env::Prod};

    let relays = networks::get_network(&selected_env);
    let is_self_sovereign = selected_env.is_self_sovereign();
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
    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .insert_resource(MovementSettings {
            sensitivity: 0.00020, // default: 0.00012
            speed: 12.0,          // default: 12.0
        })
        .insert_resource(movement::MouseCapture::default())
        .add_plugin(NoCameraPlayerPlugin)
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(DebugCursorPickingPlugin) // <- Adds the green debug cursor.
        .add_plugin(InspectorPlugin::<Inspector>::new())
        .register_inspectable::<Details>()
        // .add_plugin(DebugEventsPickingPlugin)
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(LogDiagnosticsPlugin::default())
        // .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(PolylinePlugin)
        .add_system(movement::scroll)
        .add_startup_system(
            move |commands: Commands,
                  meshes: ResMut<Assets<Mesh>>,
                  materials: ResMut<Assets<StandardMaterial>>| {
                let clone_chains_for_lanes = clone_chains_for_lanes.clone();
                setup(commands, meshes, materials, clone_chains_for_lanes);
            },
        )
        .add_system(movement::player_move_arrows)
        .add_system(rain)
        .add_system(right_click_system)
        .add_startup_system(details::configure_visuals)
        .insert_resource(bevy_atmosphere::AtmosphereMat::default()) // Default Earth sky
        .add_plugin(bevy_atmosphere::AtmospherePlugin {
            dynamic: false, // Set to false since we aren't changing the sky's appearance
            sky_radius: 10.0,
        })
        .add_system(
            move |commands: Commands,
                  meshes: ResMut<Assets<Mesh>>,
                  materials: ResMut<Assets<StandardMaterial>>,
                  asset_server: Res<AssetServer>,
                  links: Query<(Entity, &MessageSource, &GlobalTransform)>,
                  polyline_materials: ResMut<Assets<PolylineMaterial>>,
                  polylines: ResMut<Assets<Polyline>>| {
                let clone_chains = clone_chains.clone();
                render_new_events(
                    commands,
                    meshes,
                    materials,
                    clone_chains,
                    asset_server,
                    is_self_sovereign,
                    links,
                    polyline_materials,
                    polylines,
                )
            },
        )
        .add_system_to_stage(CoreStage::PostUpdate, print_events);

    for (relay_id, relay) in relays.into_iter().enumerate() {
        for (arc, mut chain_name) in relay {
            let lock_clone = arc.clone();
            if !chain_name.starts_with("ws:") && !chain_name.starts_with("wss:") {
                chain_name = format!("wss://{}", chain_name);
            }

            let url = if chain_name[5..].contains(':') {
                format!("{chain_name}")
            } else {
                format!("{chain_name}:443")
            };
            println!("url attaching to {}", url);

            // let chain_name_clone = chain_name.clone();
            let url_clone = url.clone();
            std::thread::spawn(move || {
                let mut reconnects = 0;

                while reconnects < 20 {
                    println!("Connecting to {}", &url);
                    let _ = async_std::task::block_on(datasource::watch_events(
                        lock_clone.clone(),
                        &url,
                    ));
                    println!("Problem with {} events (retrys left {})", &url, reconnects);
                    std::thread::sleep(std::time::Duration::from_secs(20));
                    reconnects += 1;
                }
                println!("giving up on {} events", url);
            });

            // let chain_name = chain_name_clone;
            let lock_clone = arc.clone();
            std::thread::spawn(move || {
                let mut reconnects = 0;

                while reconnects < 20 {
                    println!("Connecting to {}", &url_clone);
                    let _ = async_std::task::block_on(datasource::watch_blocks(
                        lock_clone.clone(),
                        url_clone.clone(),
                        relay_id.to_string(),
                    ));
                    println!(
                        "Problem with {} blocks (retries left {})",
                        &url_clone, reconnects
                    );
                    std::thread::sleep(std::time::Duration::from_secs(20));
                    reconnects += 1;
                }
                println!("giving up on {} blocks", url_clone);
            });
        }
    }
    app.run();

    // app.insert_resource(GreetTimer(Timer::from_seconds(2.0, true)))
    // .add_startup_system(add_people)
    // .add_system(greet_people);
    Ok(())
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
        DataEntity::Event { raw, .. } => {
            format!("{:#?}", raw)
        }
        DataEntity::Extrinsic {
            id: _,
            pallet,
            variant,
            args,
            contains,
            ..
        } => {
            let kids = if contains.is_empty() {
                String::new()
            } else {
                format!(" contains {} extrinsics", contains.len())
            };
            format!(
                "{}\n{} {} {}\n{:#?}",
                chain_name, pallet, variant, kids, args
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
    Event {
        raw: RawEventDetails,
        details: Details,
    },
    Extrinsic {
        id: (u32, u32),
        pallet: String,
        variant: String,
        args: Vec<String>,
        contains: Vec<DataEntity>,
        raw: Vec<u8>,
        /// psudo-unique id to link to some other node.
        link: Option<String>,
        details: Details,
    },
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
            Self::Event { details, .. } => details,
            Self::Extrinsic { details, .. } => details,
        }
    }
    pub fn pallet(&self) -> &str {
        match self {
            Self::Event { raw, .. } => raw.pallet.as_ref(),
            Self::Extrinsic { pallet, .. } => pallet,
        }
    }
    pub fn variant(&self) -> &str {
        match self {
            Self::Event { raw, .. } => raw.variant.as_ref(),
            Self::Extrinsic { variant, .. } => variant,
        }
    }

    pub fn contains(&self) -> &[DataEntity] {
        match self {
            Self::Event { .. } => EMPTY_SLICE.as_slice(),
            Self::Extrinsic { contains, .. } => contains.as_slice(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Event { .. } => EMPTY_BYTE_SLICE.as_slice(),
            Self::Extrinsic { raw, .. } => raw.as_slice(),
        }
    }

    pub fn link(&self) -> Option<&String> {
        match self {
            Self::Extrinsic { link, .. } => link.as_ref(),
            Self::Event { .. } => None,
        }
    }
}

const BLOCK: f32 = 10.;
const BLOCK_AND_SPACER: f32 = BLOCK + 4.;

fn render_new_events(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // asset_server: Res<AssetServer>,
    relays: Vec<Vec<(ABlocks, String)>>,
    asset_server: Res<AssetServer>,
    // effects: Res<Assets<EffectAsset>>,
    is_self_sovereign: bool,
    links: Query<(Entity, &MessageSource, &GlobalTransform)>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
) {
    for (rcount, relay) in relays.iter().enumerate() {
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
                            (5.5 + BLOCK_AND_SPACER * chain as f32) * rflip,
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
                            unlit: !block_events.2.inserted_pic,
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
                                (5.5 + BLOCK_AND_SPACER * chain as f32) * rflip,
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
                            unlit: !block_events.2.inserted_pic,
                            ..default()
                        });

                        // textured quad - normal
                        let rot = Quat::from_euler(EulerRot::XYZ, -PI / 2., 0., -PI / 2.); // to_radians()
                                                                                           // let mut rot = Quat::from_rotation_x(-std::f32::consts::PI / 2.0);
                        let transform = Transform {
                            translation: Vec3::new(
                                -7. + (BLOCK_AND_SPACER * block_num as f32),
                                0.1, //1.5
                                (5.5 + BLOCK_AND_SPACER * chain as f32) * rflip,
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
    block_events: Vec<(Option<DataEntity>, Vec<RawEventDetails>)>,
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
        5.5 + BLOCK_AND_SPACER * chain as f32 - 4.,
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

                let create_source = if let Some(link) = block.link() {
                    //if this id already exists then this is the destination, not the source...
                    for (entity, id, source_global) in links.iter() {
                        if id.id == *link {
                            found = true;
                            println!("creating rainbow!");

                            let mut vertices = vec![
                                Vec3::new(
                                    source_global.translation.x,
                                    source_global.translation.y,
                                    source_global.translation.z,
                                ),
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
                    if found {
                        None
                    } else {
                        println!("inserting source of rainbow!");
                        Some(MessageSource {
                            id: link.to_string(),
                        })
                    }
                } else {
                    None
                };

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
                        flattern: block.details().flattern.clone(),
                        url: format!(
                            "https://polkadot.js.org/apps/?{}#/extrinsics/decode/{}",
                            &encoded, &call_data
                        ),
                    })
                    .insert(Rainable {
                        dest: target_y * build_direction,
                    })
                    .insert(Name::new("BlockEvent"));

                if let Some(source) = create_source {
                    bun.insert(source);
                }
            }
        } else {
            // Remove the spacer as we did not add a block.
            // next_y[event_num % 81] -= DOT_HEIGHT;
        }

        for event in events {
            let details = Details {
                hover: format!("{:#?}", event),
                flattern: String::new(),
                url: format!(
                    "https://polkadot.js.org/apps/?{}#/explorer/query/{}",
                    &encoded, &hex_block_hash
                ),
            };
            let entity = DataEntity::Event {
                raw: (*event).clone(),
                details,
            };
            let style = style::style_event(&entity);
            let material = mat_map
                .entry(style.clone())
                .or_insert_with(|| materials.add(style.color.clone().into()));

            let mesh = if content::is_message(&entity) {
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
            commands
                .spawn_bundle(PbrBundle {
                    mesh,
                    material: material.clone(),
                    transform: t,
                    ..Default::default()
                })
                .insert_bundle(PickableBundle::default())
                .insert(entity.details().clone())
                .insert(Rainable {
                    dest: target_y * build_direction,
                })
                .insert(Name::new("BlockEvent"));
        }
    }
}

/// Yes this is now a verb. Who knew?
fn rainbow(vertices: &mut Vec<Vec3>, points: usize) {
    use std::f32::consts::PI;
    let start = vertices[0];
    let end = vertices[1];
    let diff = end - start;
    // x, z are linear interpolations, it is only y that goes up!

    let center = (start + end) / 2.;

    let r = end - center;
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
    mut query2: Query<Entity>,
    mut inspector: ResMut<Inspector>,
) {
    for event in events.iter() {
        match event {
            PickingEvent::Selection(selection) => {
                if let SelectionEvent::JustSelected(entity) = selection {
                    // let selection = query.get_mut(*entity).unwrap();

                    // Unspawn the previous text:
                    // query3.for_each_mut(|(entity, _)| {
                    //     commands.entity(entity).despawn();
                    // });

                    let entity = query2.get_mut(*entity).unwrap();

                    if inspector.active == Some(entity) {
                        print!("deselected current selection");
                        inspector.active = None;
                    } else {
                        inspector.active = Some(entity);
                    }

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
    relays: Vec<Vec<(ABlocks, String)>>,
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

    for (rcount, chains) in relays.iter().enumerate() {
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
                    (0.5 + (BLOCK / 2. + BLOCK_AND_SPACER * chain as f32)) * rfip,
                )),
                ..Default::default()
            });
        }
    }

    //somehow this can change the color
    //    mesh_highlighting(None, None, None);
    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
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
        })
        .insert_bundle(PickingCameraBundle { ..default() })
        .insert(FlyCam);

    use std::time::Duration;
    commands.insert_resource(UpdateTimer {
        timer: Timer::new(Duration::from_millis(50), true),
    });

    // light

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.7,
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
    #[inspectable(deletable = false)]
    active: Option<Entity>,
}

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
