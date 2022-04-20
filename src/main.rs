// use std::time::Duration;

use bevy::ecs as bevy_ecs;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::reflect::erased_serde::private::serde::de::EnumAccess;
use bevy::render::camera::CameraProjection;
use bevy_ecs::prelude::Component;
use bevy_flycam::FlyCam;
use bevy_flycam::MovementSettings;
use bevy_flycam::PlayerPlugin;
use bevy_text_mesh::prelude::*;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
// pub use wasm_bindgen_rayon::init_thread_pool;
//mod coded;
use subxt::RawEventDetails;
/// the mouse-scroll changes the field-of-view of the camera
fn scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    windows: Res<Windows>,
    mut query: Query<(&FlyCam, &mut Camera, &mut PerspectiveProjection)>,
) {
    for event in mouse_wheel_events.iter() {
        for (_camera, mut camera, mut project) in query.iter_mut() {
            project.fov = (project.fov - event.y * 0.01).abs();
            let prim = windows.get_primary().unwrap();

            //Calculate projection with new fov
            project.update(prim.width(), prim.height());

            //Update camera with the new fov
            camera.projection_matrix = project.get_projection_matrix();
            camera.depth_calculation = project.depth_calculation();

            println!("FOV: {:?}", project.fov);
        }
    }
}

// use rayon::iter::ParallelIterator;
// use rayon::iter::IntoParallelRefIterator;
// //use wasm_bindgen;

// //#[wasm_bindgen]
// pub fn sum(numbers: &[i32]) -> i32 {
//     numbers.par_iter().sum()
// }
use futures::StreamExt;
//use sp_keyring::AccountKeyring;
use subxt::{ClientBuilder, DefaultConfig, DefaultExtra, //PairSigner
};
//use smoldot::*;

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
            events: events.iter_raw().map(|c|c.unwrap()).collect::<Vec<_>>(),
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

type ABlocks = Arc<Mutex::<Vec<PolkaBlock>>>;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lock = ABlocks::default();
    let lock_statemint = ABlocks::default();
    let lock_clone = lock.clone();
    let lock_statemint_clone = lock_statemint.clone();
    let relay = "kusama-rpc.polkadot.io";
    let statemint = "statemine-rpc.dwellir.com";

    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(HelloPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00020, // default: 0.00012
            speed: 12.0,          // default: 12.0
        })
        .add_plugin(PlayerPlugin)
        .add_plugin(TextMeshPlugin)
        .add_system(scroll)
        .add_startup_system(setup)
        .add_startup_system(spawn_tasks)
        .add_system(player_move_arrows)
        .add_system(
            move |mut commands: Commands,
                  meshes: ResMut<Assets<Mesh>>,
                  materials: ResMut<Assets<StandardMaterial>>,
                  asset_server: Res<AssetServer>
                  | {
                      // Can't have too many clones!
                      let c = lock_statemint_clone.clone();
                      let cc = lock_clone.clone();
                render_new_events(commands, meshes, materials, asset_server, vec![(c,statemint.to_owned()), (cc, relay.to_owned())])
            }
        );


    let lock_clone = lock.clone();
    let lock_statemint_clone = lock_statemint.clone();
    std::thread::spawn(|| {
        //wss://kusama-rpc.polkadot.io:443
        //ws://127.0.0.1:9966
        async_std::task::block_on(block_chain(lock_clone, "wss://kusama-rpc.polkadot.io:443".to_owned()));
    });
    std::thread::spawn(|| {
        //wss://kusama-rpc.polkadot.io:443
        //ws://127.0.0.1:9966
        async_std::task::block_on(block_chain(lock_statemint_clone, "wss://statemine-rpc.dwellir.com:443".to_owned()));
    });

    app.run();

    // app.insert_resource(GreetTimer(Timer::from_seconds(2.0, true)))
    // .add_startup_system(add_people)
    // .add_system(greet_people);
    Ok(())
}

fn text(text:String, t: Transform, font: Handle<TextMeshFont> ) -> TextMeshBundle {
   
    TextMeshBundle {
        // text_mesh: TextMesh::new_with_color(
            // format!("Block {}", block.blockhash), font.clone(), Color::rgb(0., 0., 1.)),
            text_mesh: TextMesh {
                text,
                style: TextMeshStyle {
                    font:font.clone(),
                    font_size: SizeUnit::NonStandard(36.),
                    color: Color::rgb(1.0, 1.0, 0.0),
                    font_style: FontStyle::UPPERCASE, // only UPPERCASE & LOWERCASE implemented currently
                    mesh_quality: Quality::Low,
                    ..Default::default()
                },
                alignment: TextMeshAlignment {
                    // vertical: VerticalAlign::Top, // FUNCTIONALITY NOT IMPLEMENTED YET - NO EFFECT
                    // horizontal: HorizontalAlign::Left, // FUNCTIONALITY NOT IMPLEMENTED YET - NO EFFECT
                    ..Default::default()
                },
                size: TextMeshSize {
                    width: SizeUnit::NonStandard(700.),       // partially implemented
                    height: SizeUnit::NonStandard(50.),       // partially implemented
                    depth: Some(SizeUnit::NonStandard(1.0)), // must be > 0 currently, 2d mesh not supported yet
                    wrapping: true,                           // partially implemented
                    overflow: false,                          // NOT IMPLEMENTED YET
                    ..Default::default()
                },
                ..Default::default()
            },
        
            transform: t,
        
        // size: TextMeshSize {
        //     width: SizeUnit::NonStandard(135.),   
        //     ..Default::default()
        // },

        ..Default::default()
        }
}

fn render_new_events(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    locks: Vec<(ABlocks, String)>,
) {
    for (chain, (lock, chain_name)) in locks.iter().enumerate() {
        if let Ok(ref mut block_events) = lock.try_lock() {
            if let Some(block) = block_events.pop() {

                // commands.spawn_bundle(NodeBundle {
                //     style: Style {
                //         size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                //         justify_content: JustifyContent::SpaceBetween,
                //         ..default()
                //     },
                //     color: Color::NONE.into(),
                //     ..default()
                // })
                // .with_children(|parent| {
                let font: Handle<TextMeshFont> = asset_server.load("fonts/Audiowide-Mono-Latest.ttf");
                let mut t = Transform::from_xyz(0., 0., 0.);
                t.rotate(Quat::from_rotation_x(-90.));
                t = t.with_translation(Vec3::new(-4.,0.,4.));

                let mut t2 = Transform::from_xyz(0., 0., 0.);
                t2.rotate(Quat::from_rotation_x(-90.));
                t2 = t2.with_translation(Vec3::new(-4.,0.,2.));

                commands.spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
                    material: materials.add(
                        StandardMaterial {
                            base_color: Color::rgba(0.1, 0.9, 0.1, 0.6),
                            alpha_mode: AlphaMode::Blend,
                            perceptual_roughness: 0.08,
                            ..default()
                        },
                    ),
                    transform: Transform::from_translation(Vec3::new(
                        0. + (11. * block.blocknum as f32),
                        
                        0.,
                        11. * chain as f32,
                    )),
                    ..Default::default()
                }).with_children(|parent| {
                    parent.spawn_bundle(text(format!("Block {}", block.blockhash), t, font.clone()));
                    parent.spawn_bundle(text(format!("{}", chain_name), t2, font));
                });

               
                // commands.spawn_bundle();
               
//                 commands.spawn_bundle(UiCameraBundle::default());
//                     commands.spawn_bundle(TextBundle {
//                         style: Style {
//                             size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
//                             // align_self: AlignSelf::FlexEnd,
//                             // position_type: PositionType::Relative,
//                             // position: Rect {
//                             // //    bottom: Val::Px(5.0),
//                             //   //  right: Val::Px(15.0),
//                             //     ..default()
//                             // },
//                             ..default()
//                         },
//                         // Use the `Text::with_section` constructor
//                         text: Text::with_section(
//                             // Accepts a `String` or any type that converts into a `String`, such as `&str`
//                             "hello\nbevy!",
//                             TextStyle {
//                                 font: asset_server.load("/home/gilescope/fonts/Audiowide-Mono-Latest.ttf"),
//                                 font_size: 100.0,
//                                 color: Color::BLACK,
//                             },
//                             // Note: You can use `Default::default()` in place of the `TextAlignment`
//                             TextAlignment {
// //                                horizontal: HorizontalAlign::Center,
//                                 ..default()
//                             },
//                         ),
//                         ..default()
//                     });
                // });
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


            // for event in block.events {
            //     match event.pallet.as_str() {
            //         _ => {
            //             commands.spawn_bundle(PbrBundle {
            //                 mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            //                 ///* event.blocknum as f32
            //                 material: materials.add(Color::hex("e6007a").unwrap().into()),
            //                 transform: Transform::from_translation(Vec3::new(
            //                     0.2 + (1.1 * block.blocknum as f32),
            //                     0.2,
            //                     3.2,
            //                 )),
            //                 ..Default::default()
            //             });
            //         }
            //     }
            // }
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


fn scale(value: usize) -> f32 {
    value as f32 / 1000_000.
}


use bevy::tasks::AsyncComputeTaskPool;

fn spawn_tasks(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    //    std::thread::spawn(|| {
    //  std::thread::sleep(Duration::from_millis(1000));
    //  });

    #[derive(Debug, Clone, Default, Eq, PartialEq, Component)]
    pub struct BlockState {
        x: u32,
        y: u32,
        weight: u64,
    };

    //     thread_pool.spawn(async move {
    // //        std::thread::sleep(Duration::from_millis(1000));
    // //      delay_for(Duration::from_millis(1000)).await;
    //       //Result { time: 1.0 }
    //      ()
    //     }) .detach();
    //commands.spawn().insert(task);
}

//   fn handle_tasks(
//     mut commands: Commands,
//     mut transform_tasks: Query<(Entity, &mut Task<Result>)>,
//   ) {
//     for (entity, mut task) in transform_tasks.iter_mut() {
//       if let Some(res) = future::block_on(future::poll_once(&mut *task)) {
//         commands.entity(entity).remove::<Task<Result>>();
//       }
//     }
//   }

/// Handles keyboard input and movement
fn player_move_arrows(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    windows: Res<Windows>,
    settings: Res<MovementSettings>,
    mut query: Query<&mut Transform, With<FlyCam>>,
) {
    let window = windows.get_primary().unwrap();
    for mut transform in query.iter_mut() {
        let mut velocity = Vec3::ZERO;
        let local_z = transform.local_z();
        let forward = -Vec3::new(local_z.x, 0., local_z.z);
        let right = Vec3::new(local_z.z, 0., -local_z.x);

        for key in keys.get_pressed() {
            if window.cursor_locked() {
                match key {
                    KeyCode::Up => velocity += forward,
                    KeyCode::Down => velocity -= forward,
                    KeyCode::Left => velocity -= right,
                    KeyCode::Right => velocity += right,
                    _ => (),
                }
            }
        }

        velocity = velocity.normalize_or_zero();

        transform.translation += velocity * time.delta_seconds() * settings.speed
    }
}

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        // add things to your app here
    }
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);


/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 500.0 })),
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
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            //.init_resource::<TrackInputState>()
            .add_system(capture_mouse_on_click);
    }
}

// #[derive(Default)]
// struct TrackInputState<'a> {
//     mousebtn: EventReader<'a, 'a, MouseButtonInput>,
// }

fn capture_mouse_on_click(
    mouse: Res<Input<MouseButton>>,
    //    mut state: ResMut<'a, TrackInputState>,
    //  ev_mousebtn: Res<Events<MouseButtonInput>>,
    //key: Res<Input<KeyCode>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        #[cfg(target_arch="wasm32")]
        html_body::get().request_pointer_lock();
        // window.set_cursor_visibility(false);
        // window.set_cursor_lock_mode(true);
    }
    // if key.just_pressed(KeyCode::Escape) {
    //     //window.set_cursor_visibility(true);
    //     //window.set_cursor_lock_mode(false);
    // }
    // for _ev in state.mousebtn.iter(&ev_mousebtn) {
    //     html_body::get().request_pointer_lock();
    //     break;
    // }
}

#[cfg(target_arch="wasm32")]
pub mod html_body {
    use web_sys::HtmlElement;

    pub fn get() -> HtmlElement {
        // From https://www.webassemblyman.com/rustwasm/how_to_add_mouse_events_in_rust_webassembly.html
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let body = document.body().expect("document should have a body");
        body
    }
}
