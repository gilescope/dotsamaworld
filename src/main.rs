// use std::time::Duration;

use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy_ecs::prelude::Component;
use bevy_flycam::FlyCam;
use bevy_flycam::MovementSettings;
use bevy_mod_picking::*;
use bevy_text_mesh::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
// pub use wasm_bindgen_rayon::init_thread_pool;
//mod coded;
// use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
// use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy_flycam::NoCameraPlayerPlugin;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use subxt::RawEventDetails;

mod movement;
mod style;

mod content;

use futures::StreamExt;

use subxt::{ClientBuilder, DefaultConfig, DefaultExtra};

#[subxt::subxt(runtime_metadata_path = "polkadot_metadata.scale")]
pub mod polkadot {}

struct PolkaBlock {
    blocknum: usize,
    blockhash: String,
    events: Vec<RawEventDetails>,
}

async fn block_chain(tx: ABlocks, url: String) -> Result<(), Box<dyn std::error::Error>> {
    let api = ClientBuilder::new()
        .set_url(&url)
        //    .set_url("ws://127.0.0.1:9944")
        //        .set_url("wss://kusama-rpc.polkadot.io:443")
        //wss://kusama-rpc.polkadot.io:443
        .build()
        .await?
        .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();

    let mut event_sub = api.events().subscribe().await?;
    let mut blocknum = 1;
    while let Some(events) = event_sub.next().await {
        let events = events?;
        let block_hash = events.block_hash();
        blocknum += 1;

        tx.lock().unwrap().push(PolkaBlock {
            blocknum,
            blockhash: events.block_hash().to_string(),
            events: events.iter_raw().map(|c| c.unwrap()).collect::<Vec<_>>(),
        });

        // for event in events.iter_raw() {
        //     let event: RawEventDetails = event?;
        //     // match event.pallet.as_str() {
        //     //     "ImOnline" | "ParaInclusion" | "PhragmenElection" => {
        //     //         continue;
        //     //     }
        //     //     _ => {}
        //     // }

        //     // if event.pallet == "System" {
        //     //     if event.variant == "ExtrinsicSuccess" {
        //     //         continue;
        //     //     }
        //     // }

        //     let is_balance_transfer = event
        //         .as_event::<polkadot::balances::events::Transfer>()?
        //         .is_some();

        //     let is_online = event
        //         .as_event::<polkadot::im_online::events::AllGood>()?
        //         .is_some();

        //     let is_new_session = event
        //         .as_event::<polkadot::session::events::NewSession>()?
        //         .is_some();

        //     if !is_online && !is_new_session {
        //         tx.lock().unwrap().push(BlockEvent {
        //             blocknum,
        //             raw_event: event.clone(),
        //         });
        //         println!("    {:?}\n", event.pallet);
        //         println!("    {:#?}\n", event);

        //         // stdout()
        //         // .execute(SetForegroundColor(Color::Green)).unwrap()
        //         // .execute(SetBackgroundColor(Color::Black)).unwrap()
        //         // .execute(Print(format!("    {:?}\r\n", event))).unwrap()
        //         // .execute(ResetColor).unwrap();
        //     }
        // }
    }
    Ok(())
}

static RELAY_BLOCKS: AtomicU32 = AtomicU32::new(0);
static RELAY_BLOCKS2: AtomicU32 = AtomicU32::new(0);

type ABlocks = Arc<Mutex<Vec<PolkaBlock>>>;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let lock = ABlocks::default();
    // let lock_statemint = ABlocks::default();
    // let lock_clone = lock.clone();
    // let lock_statemint_clone = lock_statemint.clone();
    let relays = vec![
        vec![
            "rpc.polkadot.io",
            "statemint-rpc.polkadot.io",
            "acala.polkawallet.io",
            "wss.odyssey.aresprotocol.io",
            "astar-rpc.dwellir.com",
            "fullnode.parachain.centrifuge.io",
            "clover.api.onfinality.io/public-ws",
            "rpc.efinity.io",
            "rpc-01.hydradx.io",
            "interlay.api.onfinality.io/public-ws",
            "k-ui.kapex.network",
            "wss.api.moonbeam.network",
            "eden-rpc.dwellir.com",
            "rpc.parallel.fi",
            "api.phala.network/ws",
            "polkadex.api.onfinality.io/public-ws",
            "ws.unique.network",
        ],
        vec![
            //    "rococo-rpc.polkadot.io",
            //     "rococo-canvas-rpc.polkadot.io",
            // "rococo.api.encointer.org",
            // "rpc-01.basilisk-rococo.hydradx.io",
            // "fullnode.catalyst.cntrfg.com",
            // "anjie.rococo.dolphin.engineering",
            // "rpc.rococo.efinity.io",
            // "rococo.api.integritee.network",
            // "rpc.rococo-parachain-sg.litentry.io",
            // "moonsama-testnet-rpc.moonsama.com",
            // "node-6913072722034561024.lh.onfinality.io/ws?apikey=84d77e2e-3793-4785-8908-5096cffea77a", //noodle
            // "pangolin-parachain-rpc.darwinia.network",
            // "rococo.kilt.io",
            // "rco-para.subsocial.network",

            // "westend-rpc.dwellir.com",
            // "westmint-rpc.polkadot.io",
            // "fullnode-collator.charcoal.centrifuge.io",
            // "teerw1.integritee.network",
            // "westend.kylin-node.co.uk",
            // "rpc.westend.standard.tech",
            // "westend.kilt.io:9977"

            // "ws://127.0.0.1:9944",
            // "ws://127.0.0.1:9966",
            // "ws://127.0.0.1:9920",
            "kusama-rpc.polkadot.io",
            "statemine-rpc.dwellir.com",
            "wss.api.moonriver.moonbeam.network",
            "karura-rpc.dwellir.com",
            "bifrost-rpc.dwellir.com",
            "khala-rpc.dwellir.com",
            "shiden-rpc.dwellir.com",
            "rpc-shadow.crust.network",
            "kusama.api.integritee.network",
            "kusama.rpc.robonomics.network",
            "calamari-rpc.dwellir.com",
            "heiko-rpc.parallel.fi",
            "kilt-rpc.dwellir.com",
            "picasso-rpc.composable.finance",
            "basilisk-rpc.dwellir.com",
            "kintsugi-rpc.dwellir.com",
            "us-ws-quartz.unique.network",
            "para.subsocial.network",
            "zeitgeist-rpc.dwellir.com",
            "crab-parachain-rpc.darwinia.network",
            "rpc.litmus-parachain.litentry.io",
            "rpc.api.kico.dico.io",
        ], //wss://altair.api.onfinality.io/public-ws wss://pioneer.api.onfinality.io/public-ws wss://turing.api.onfinality.io/public-ws
    ]
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
        .add_plugin(NoCameraPlayerPlugin)
        //.add_plugin(TextMeshPlugin)
        .add_plugins(DefaultPickingPlugins)
        // .add_plugin(DebugCursorPickingPlugin) // <- Adds the green debug cursor.
        // .add_plugin(DebugEventsPickingPlugin)
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(LogDiagnosticsPlugin::default())
        .add_system(movement::scroll)
        .add_startup_system(
            move |commands: Commands,
                  meshes: ResMut<Assets<Mesh>>,
                  materials: ResMut<Assets<StandardMaterial>>| {
                let clone_chains_for_lanes = clone_chains_for_lanes.clone();
                setup(commands, meshes, materials, clone_chains_for_lanes);
            },
        )
        // .add_startup_system(spawn_tasks)
        .add_system(movement::player_move_arrows)
        .add_system(
            move |commands: Commands,
                  meshes: ResMut<Assets<Mesh>>,
                  materials: ResMut<Assets<StandardMaterial>>,
                  asset_server: Res<AssetServer>| {
                let clone_chains = clone_chains.clone();
                render_new_events(commands, meshes, materials, asset_server, clone_chains)
            },
        )
        .add_system_to_stage(CoreStage::PostUpdate, print_events);

    for relay in relays {
        for (arc, mut chain_name) in relay {
            let lock_clone = arc.clone();
            std::thread::spawn(move || {
                //wss://kusama-rpc.polkadot.io:443
                //ws://127.0.0.1:9966
                if !chain_name.starts_with("ws:") && !chain_name.starts_with("wss:") {
                    chain_name = format!("wss://{}", chain_name);
                }

                let url = if chain_name[5..].contains(':') {
                    format!("{chain_name}")
                } else {
                    format!("{chain_name}:443")
                };
                println!("url attaching to {}", url);
                async_std::task::block_on(block_chain(lock_clone, url)).unwrap();
            });
        }
    }
    app.run();

    // app.insert_resource(GreetTimer(Timer::from_seconds(2.0, true)))
    // .add_startup_system(add_people)
    // .add_system(greet_people);
    Ok(())
}

// fn text(text: String, t: Transform, font: Handle<TextMeshFont>) -> TextMeshBundle {
//     TextMeshBundle {
//         // text_mesh: TextMesh::new_with_color(
//         // format!("Block {}", block.blockhash), font.clone(), Color::rgb(0., 0., 1.)),
//         text_mesh: TextMesh {
//             text,
//             style: TextMeshStyle {
//                 font: font.clone(),
//                 font_size: SizeUnit::NonStandard(36.),
//                 color: Color::hex("e6007a").unwrap(), //Color::rgb(1.0, 1.0, 0.0),
//                 font_style: FontStyle::UPPERCASE, // only UPPERCASE & LOWERCASE implemented currently
//                 mesh_quality: Quality::Low,
//                 ..Default::default()
//             },
//             alignment: TextMeshAlignment {
//                 // vertical: VerticalAlign::Top, // FUNCTIONALITY NOT IMPLEMENTED YET - NO EFFECT
//                 // horizontal: HorizontalAlign::Left, // FUNCTIONALITY NOT IMPLEMENTED YET - NO EFFECT
//                 ..Default::default()
//             },
//             size: TextMeshSize {
//                 width: SizeUnit::NonStandard(700.),      // partially implemented
//                 height: SizeUnit::NonStandard(50.),      // partially implemented
//                 depth: Some(SizeUnit::NonStandard(1.0)), // must be > 0 currently, 2d mesh not supported yet
//                 wrapping: true,                          // partially implemented
//                 overflow: false,                         // NOT IMPLEMENTED YET
//                 ..Default::default()
//             },
//             ..Default::default()
//         },

//         transform: t,

//         // size: TextMeshSize {
//         //     width: SizeUnit::NonStandard(135.),
//         //     ..Default::default()
//         // },
//         ..Default::default()
//     }
// }

enum BuildDirection {
    Up,
    Down,
}

fn render_new_events(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    relays: Vec<Vec<(ABlocks, String)>>,
) {
    for (rcount, relay) in relays.iter().enumerate() {
        for (chain, (lock, chain_name)) in relay.iter().enumerate() {
            if let Ok(ref mut block_events) = lock.try_lock() {
                if let Some(block) = block_events.pop() {
                    let block_num = if rcount == 0 {
                        if chain == 0 {
                            //relay
                            RELAY_BLOCKS
                                .store(RELAY_BLOCKS.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
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
                    };

                    // let font: Handle<TextMeshFont> =
                    //     asset_server.load("fonts/Audiowide-Mono-Latest.ttf");
                    let mut t = Transform::from_xyz(0., 0., 0.);
                    t.rotate(Quat::from_rotation_x(-90.));
                    t = t.with_translation(Vec3::new(-4., 0., 4.));

                    let mut t2 = Transform::from_xyz(0., 0., 0.);
                    t2.rotate(Quat::from_rotation_x(-90.));
                    t2 = t2.with_translation(Vec3::new(-4., 0., 2.));

                    let rflip = if rcount == 1 { -1.0 } else { 1.0 };

                    commands
                        .spawn_bundle(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Box::new(10., 0.1, 10.))),
                            material: materials.add(StandardMaterial {
                                base_color: Color::rgba(0., 0., 0., 0.7),
                                alpha_mode: AlphaMode::Blend,
                                perceptual_roughness: 0.08,
                                ..default()
                            }),
                            transform: Transform::from_translation(Vec3::new(
                                0. + (11. * block_num as f32),
                                0.,
                                (5.5 + 11. * chain as f32) * rflip,
                            )),
                            ..Default::default()
                        })
                        // .with_children(|parent| {
                        //     parent.spawn_bundle(text(
                        //         format!("Block {}", block.blockhash),
                        //         t,
                        //         font.clone(),
                        //     ));
                        //     parent.spawn_bundle(text(format!("{}", chain_name), t2, font));
                        // })
                        ;

                    // use bevy::text::Text2dBounds;
                    // //let font = asset_server.load("fonts/FiraSans-Bold.ttf");
                    // let font = asset_server.load("fonts/Audiowide-Mono-Latest.ttf");
                    // let text_style = TextStyle {
                    //     font,
                    //     font_size: 100.0,
                    //     color: Color::RED,
                    // };
                    // let text_alignment = TextAlignment {
                    //     vertical: bevy::prelude::VerticalAlign::Center,
                    //     horizontal: bevy::prelude::HorizontalAlign::Center,
                    // };
                    // // let box_size = Size::new(300.0, 200.0);
                    // // let box_position = Vec2::new(0.0, -250.0);
                    // // let text_alignment_topleft = TextAlignment {
                    // //     vertical: bevy::prelude::VerticalAlign::Top,
                    // //     horizontal: bevy::prelude::HorizontalAlign::Left,
                    // // };
                    // let mut cam = OrthographicCameraBundle::new_2d();
                    // cam.transform.rotate(Quat::from_xyzw(0.0, 0.2, 0.2, 0.0));

                    // commands.spawn_bundle(cam);
                    // commands.spawn_bundle(Text2dBundle {
                    //     text: Text::with_section(
                    //         "this text wraps in the box",
                    //         text_style,
                    //         text_alignment,
                    //     ),
                    //     // text_2d_bounds: Text2dBounds {
                    //     //     // Wrap text in the rectangle
                    //     //     size: box_size,
                    //     // },
                    //     // We align text to the top-left, so this transform is the top-left corner of our text. The
                    //     // box is centered at box_position, so it is necessary to move by half of the box size to
                    //     // keep the text in the box.
                    //     // transform: Transform::from_xyz(
                    //     //     box_position.x - box_size.width / 2.0,
                    //     //     box_position.y + box_size.height / 2.0,
                    //     //     1.0,
                    //     // ),
                    //     // visibility:Visibility::Visible,
                    //     ..default()
                    // });

                    //How to do UI text:

                    // .insert(ColorText);
                    // commands
                    // .spawn(TextBundle{
                    //     text: Text{value: "Score:".to_string(),
                    //     font: assets.load("FiraSans-Bold.ttf"),
                    //     style:TextStyle{
                    //         font_size:30.0,
                    //         color:Color::WHITE,
                    //         ..Default::default()},..Default::default()},
                    //     transform: Transform::from_translation(Vec3::new(-380.0,-380.0,2.0)),
                    //     ..Default::default()
                    // })
                    // .with(TextTag);

                    add_blocks(
                        block_num,
                        chain,
                        block
                            .events
                            .iter()
                            .filter(|&e| !content::is_utiliy_extrinsic(e)),
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        BuildDirection::Up,
                        rflip,
                    );

                    add_blocks(
                        block_num,
                        chain,
                        block
                            .events
                            .iter()
                            .filter(|&e| content::is_utiliy_extrinsic(e)),
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        BuildDirection::Down,
                        rflip,
                    );
                }
            }
        }
    }
    //     if let Ok(ref mut block_events) = lock_clone.try_lock() {
    //         if let Some(event) = block_events.pop() {
    //             match event.raw_event.pallet.as_str() {
    //                 "XcmpQueue" => {
    //                     commands.spawn_bundle(PbrBundle {
    //                         mesh: meshes.add(Mesh::from(shape::Icosphere {
    //                             radius: 0.45,
    //                             subdivisions: 32,
    //                         })),
    //                         ///* event.blocknum as f32
    //                         material: materials.add(Color::hex("FFFF00").unwrap().into()),
    //                         transform: Transform::from_translation(Vec3::new(
    //                             0.2 + (1.1 * scale(event.blocknum)),
    //                             0.2,
    //                             0.2,
    //                         )),
    //                         ..Default::default()
    //                     });
    //                     if event.raw_event.variant == "fail" {
    //                         // TODO: Xcmp pallet is not on the relay chain.
    //                         // use crate::polkadot::balances::events::Deposit;
    //                         // let deposit = Deposit::decode(&mut event.raw_event.data.to_vec().as_slice()).unwrap();
    //                         // println!("{:?}", deposit);
    //                     }
    //                 }
    //                 "Staking" => {
    //                     commands.spawn_bundle(PbrBundle {
    //                         mesh: meshes.add(Mesh::from(shape::Icosphere {
    //                             radius: 0.45,
    //                             subdivisions: 32,
    //                         })),
    //                         ///* event.blocknum as f32
    //                         material: materials.add(Color::hex("00ffff").unwrap().into()),
    //                         transform: Transform::from_translation(Vec3::new(
    //                             0.2 + (1.1 * scale(event.blocknum)),
    //                             0.2,
    //                             0.2,
    //                         )),
    //                         ..Default::default()
    //                     });
    //                 }
    //                 "Balances" => {
    //                     match event.raw_event.variant.as_str() {
    //                         "Deposit" => {
    //                             use crate::polkadot::balances::events::Deposit;
    //                             use codec::Decode;
    //                             use  bevy::prelude::shape::CapsuleUvProfile;
    //                             let deposit = Deposit::decode(&mut event.raw_event.data.to_vec().as_slice()).unwrap();
    //                             println!("{:?}", deposit);
    //                             //use num_integer::roots::Roots;

    //                             commands.spawn_bundle(PbrBundle {
    //                                 mesh: meshes.add(Mesh::from(shape::Capsule {
    //                                     radius: 0.45,
    //                                     depth: 0.4 * scale(deposit.amount as usize),
    //                                     // latitudes: 2,
    //                                     // longitudes: 1,
    //                                     // rings: 2,
    //                                     // uv_profile:CapsuleUvProfile::Aspect
    //                                     ..Default::default()
    // //                                                subdivisions: 32,
    //                                 })),
    //                                 ///* event.blocknum as f32
    //                                 material: materials
    //                                     .add(Color::hex("e6007a").unwrap().into()),
    //                                 transform: Transform::from_translation(Vec3::new(
    //                                     0.2 + (1.1 * scale(event.blocknum)),
    //                                     0.2,
    //                                     0.2,
    //                                 )),
    //                                 ..Default::default()
    //                             });
    //                         }
    //                         "Withdraw" => {
    //                             commands.spawn_bundle(PbrBundle {
    //                                 mesh: meshes.add(Mesh::from(shape::Icosphere {
    //                                     radius: 0.45,
    //                                     subdivisions: 32,
    //                                 })),
    //                                 ///* event.blocknum as f32
    //                                 material: materials
    //                                     .add(Color::hex("000000").unwrap().into()),
    //                                 transform: Transform::from_translation(Vec3::new(
    //                                     0.2 + (1.1 * scale(event.blocknum)),
    //                                     0.2,
    //                                     0.2,
    //                                 )),
    //                                 ..Default::default()
    //                             });
    //                         }
    //                         _ => {
    //                             commands.spawn_bundle(PbrBundle {
    //                                 mesh: meshes.add(Mesh::from(shape::Icosphere {
    //                                     radius: 0.45,
    //                                     subdivisions: 32,
    //                                 })),
    //                                 ///* event.blocknum as f32
    //                                 material: materials
    //                                     .add(Color::hex("ff0000").unwrap().into()),
    //                                 transform: Transform::from_translation(Vec3::new(
    //                                     0.2 + (1.1 * scale(event.blocknum)),
    //                                     0.2,
    //                                     0.2,
    //                                 )),
    //                                 ..Default::default()
    //                             });
    //                         }
    //                     }
    //                 }
    //                 _ => {
    //                     commands.spawn_bundle(PbrBundle {
    //                         mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //                         ///* event.blocknum as f32
    //                         material: materials.add(Color::hex("e6007a").unwrap().into()),
    //                         transform: Transform::from_translation(Vec3::new(
    //                             0.2 + (1.1 * scale(event.blocknum)),
    //                             0.2,
    //                             0.2,
    //                         )),
    //                         ..Default::default()
    //                     });
    //                 }
    //             }
    //         }
    // }
}

fn add_blocks<'a>(
    block_num: u32,
    chain: usize,
    block_events: impl Iterator<Item = &'a RawEventDetails>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    build_direction: BuildDirection,
    rflip: f32,
) {
    let build_direction = if let BuildDirection::Up = build_direction {
        1.0
    } else {
        -1.0
    };
    // Add all the useful blocks

    let mesh = meshes.add(Mesh::from(shape::Cube { size: 0.8 }));
    let mut mat_map = HashMap::new();

    let (base_x, base_z) = (
        0. + (11. * block_num as f32) - 4.,
        5.5 + 11. * chain as f32 - 4.,
    );
    for (event_num, event) in block_events.enumerate() {
        let x = event_num % 9;
        let z = (event_num / 9) % 9;
        let y = event_num / 9 / 9;
        match event.pallet.as_str() {
            _ => {
                let style = style::style_event(event);

                let material = mat_map
                    .entry(style.clone())
                    .or_insert_with(|| materials.add(style.color.clone().into()));

                commands
                    .spawn_bundle(PbrBundle {
                        mesh: mesh.clone(),
                        ///* event.blocknum as f32
                        material: material.clone(),
                        transform: Transform::from_translation(Vec3::new(
                            base_x + x as f32,
                            (0.5 + y as f32) * build_direction,
                            (base_z + z as f32) * rflip,
                        )),
                        ..Default::default()
                    })
                    .insert_bundle(PickableBundle::default())
                    .insert(Details {
                        hover: format!("{} - {}", event.pallet, event.variant),
                    });
            }
        }
    }
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

// A unit struct to help identify the color-changing Text component
#[derive(Component)]
pub struct ColorText;

#[derive(Component)]
pub struct Details {
    hover: String,
}

pub fn print_events(
    mut commands: Commands,
    mut events: EventReader<PickingEvent>,
    // query: Query<&mut Selection>,
    mut query2: Query<&mut Details>,
    mut query3: Query<(Entity, With<ColorText>)>,
    asset_server: Res<AssetServer>,
) {
    let t = Transform::from_xyz(1., 10., 0.);
    for event in events.iter() {
        match event {
            PickingEvent::Selection(selection) => {
                if let SelectionEvent::JustSelected(entity) = selection {
                    // let selection = query.get_mut(*entity).unwrap();
                    let details = query2.get_mut(*entity).unwrap();
                    println!("{}", details.hover);

                    query3.for_each_mut(|(entity, _)| {
                        //   entity.remove();
                        //  commands.entity(entity).despawn();
                        // entity.despawn();
                        commands.entity(entity).despawn();
                    });

                    commands
                        .spawn_bundle(TextBundle {
                            style: Style {
                                // align_self: AlignSelf::Center, // Without this the text would be on the bottom left corner
                                ..Default::default()
                            },
                            text: Text::with_section(
                                details.hover.to_string(),
                                TextStyle {
                                    font: asset_server.load("fonts/Audiowide-Mono-Latest.ttf"),
                                    font_size: 60.0,
                                    color: Color::WHITE,
                                },
                                TextAlignment {
                                    vertical: bevy::prelude::VerticalAlign::Center,
                                    horizontal: bevy::prelude::HorizontalAlign::Center,
                                },
                            ),
                            transform: t,
                            ..Default::default()
                        })
                        .insert(ColorText);
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
                    (5.5 + 11. * chain as f32) * rfip,
                )),
                ..Default::default()
            });
        }
    }

    commands.spawn_bundle(UiCameraBundle::default());

    // let mut t = Transform::from_xyz(0., 0., 0.);
    // t.rotate(Quat::from_rotation_x(-90.));

    // commands.spawn_bundle(TextBundle {
    //     style: Style {
    //         align_self: AlignSelf::Center, // Without this the text would be on the bottom left corner
    //         ..Default::default()
    //     },
    //     text: Text::with_section(
    //         "hello world!".to_string(),
    //         TextStyle {
    //             font: asset_server.load("fonts/Audiowide-Mono-Latest.ttf"),
    //             font_size: 60.0,
    //             color: Color::WHITE,
    //         },
    //         TextAlignment {
    //             vertical: bevy::prelude::VerticalAlign::Center,
    //             horizontal: bevy::prelude::HorizontalAlign::Center,
    //         },
    //     ),
    //     // transform: t,
    //     ..Default::default()
    // });

    //somehow this can change the color
    //    mesh_highlighting(None, None, None);
    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            perspective_projection: PerspectiveProjection {
                far: 100., // 1000 will be 100 blocks that you can s
                ..PerspectiveProjection::default()
            },
            ..default()
        })
        .insert_bundle(PickingCameraBundle::default())
        .insert(FlyCam);

    // commands.spawn_bundle(TextBundle {
    //     style: Style {
    //         align_self: AlignSelf::FlexEnd,
    //         position_type: PositionType::Absolute,
    //         position: Rect {
    //             bottom: Val::Px(5.0),
    //             right: Val::Px(15.0),
    //             ..default()
    //         },
    //         ..default()
    //     },
    //     // Use the `Text::with_section` constructor
    //     text: Text::with_section(
    //         // Accepts a `String` or any type that converts into a `String`, such as `&str`
    //         "hello\nbevy!",
    //         TextStyle {
    //             font: asset_server.load("/home/gilescope/fonts/Audiowide-Mono-Latest.ttf"),
    //             font_size: 100.0,
    //             color: Color::BLACK,
    //         },
    //         // Note: You can use `Default::default()` in place of the `TextAlignment`
    //         TextAlignment {
    //                             horizontal: HorizontalAlign::Center,
    //             ..default()
    //         },
    //     ),
    //     ..default()
    // });

    // cube
    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //     material: materials.add(
    //         //    Color::hex("e6007a").unwrap().into()
    //         StandardMaterial {
    //             base_color: Color::rgba(0.2, 0.3, 0.5, 0.7),
    //             // vary key PBR parameters on a grid of spheres to show the effect
    //             alpha_mode: AlphaMode::Blend,
    //             metallic: 0.2,
    //             perceptual_roughness: 0.2,
    //             ..default()
    //         },
    //     ),

    //     transform: Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
    //     ..Default::default()
    // });

    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Icosphere {
    //         radius: 0.45,
    //         subdivisions: 32,
    //     })),
    //     material: materials.add(StandardMaterial {
    //         base_color: Color::hex("e6007a").unwrap().into(),
    //         // vary key PBR parameters on a grid of spheres to show the effect
    //         metallic: 0.2,
    //         perceptual_roughness: 0.2,
    //         ..default()
    //     }),
    //     transform: Transform::from_xyz(0.3, 1.5, 0.0),
    //     ..default()
    // });

    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::UVSphere {
    //         sectors: 128,
    //         stacks: 64,
    //         ..default()
    //     })),
    //     material: materials.add(StandardMaterial {
    //         base_color: Color::hex("e6007a").unwrap(),
    //         // vary key PBR parameters on a grid of spheres to show the effect
    //         metallic: 0.2,
    //         perceptual_roughness: 0.2,
    //         ..default()
    //     }),
    //     transform: Transform::from_xyz(2.3, -2.5, 1.0),
    //     ..default()
    // });

    // light

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
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

// pub struct UiPlugin;

// impl Plugin for UiPlugin {
//     fn build(&self, app: &mut App) {
//         app
//             //.init_resource::<TrackInputState>()
//             .add_system(capture_mouse_on_click);
//     }
// }

// // #[derive(Default)]
// // struct TrackInputState<'a> {
// //     mousebtn: EventReader<'a, 'a, MouseButtonInput>,
// // }

// fn capture_mouse_on_click(
//     mouse: Res<Input<MouseButton>>,
//     //    mut state: ResMut<'a, TrackInputState>,
//     //  ev_mousebtn: Res<Events<MouseButtonInput>>,
//     //key: Res<Input<KeyCode>>,
// ) {
//     if mouse.just_pressed(MouseButton::Left) {
//         #[cfg(target_arch = "wasm32")]
//         html_body::get().request_pointer_lock();
//         // window.set_cursor_visibility(false);
//         // window.set_cursor_lock_mode(true);
//     }
//     // if key.just_pressed(KeyCode::Escape) {
//     //     //window.set_cursor_visibility(true);
//     //     //window.set_cursor_lock_mode(false);
//     // }
//     // for _ev in state.mousebtn.iter(&ev_mousebtn) {
//     //     html_body::get().request_pointer_lock();
//     //     break;
//     // }
// }

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
