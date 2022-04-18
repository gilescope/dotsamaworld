use bevy::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy_ecs::prelude::Component;

use bevy_flycam::PlayerPlugin;
use bevy_flycam::FlyCam;
use bevy::render::camera::CameraProjection;
use bevy::input::mouse::MouseWheel;
use bevy_flycam::MovementSettings;

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

fn main() {
   
 

    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins);
    app.add_plugin(HelloPlugin);
    app.insert_resource(MovementSettings {
        sensitivity: 0.00015, // default: 0.00012
        speed: 12.0,          // default: 12.0
    });
    app.add_plugin(PlayerPlugin)
    .add_system(scroll);
    app.add_startup_system(setup.system());
    app.add_system(hello_world);
    app.add_system(player_move_arrows);
    app.run();


    // app.insert_resource(GreetTimer(Timer::from_seconds(2.0, true)))
    // .add_startup_system(add_people)
    // .add_system(greet_people);
}

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
                    // KeyCode::Space => velocity += Vec3::Y,
                    // KeyCode::LShift => velocity -= Vec3::Y,
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

fn hello_world(    
    time: Res<Time>,
    // texture_atlases: Res<Assets<TextureAtlas>>,
    // mut query: Query<(
    //     &mut AnimationTimer,
    //     &mut TextureAtlasSprite,
    //     &Handle<TextureAtlas>,
    // )>
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
)
    {
    println!("hello world!");
     // cube
     commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::hex("e6007a").unwrap().into()),
        transform: Transform::from_translation(Vec3::new(0.2, 0.2, 0.1)),
        ..Default::default()
    });
}

//#[derive(Component)]
struct Block;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(
            StandardMaterial {
                base_color: Color::rgb(0.2, 0.2, 0.2),
                
                perceptual_roughness: 0.08,
                ..default()
            }
            //    Color::rgb(0.5, 0.5, 0.5).into()
        ),
        ..Default::default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(
        //    Color::hex("e6007a").unwrap().into()
        
        StandardMaterial {
            base_color:Color::rgba(0.2,0.3,0.5,0.7) ,
            // vary key PBR parameters on a grid of spheres to show the effect
            alpha_mode: AlphaMode::Blend,
            metallic: 0.2,
            perceptual_roughness: 0.2,
            ..default()
        }
        ),
       
        transform: Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ..Default::default()
    });

    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            radius: 0.45,
            subdivisions: 32,
        })),
        material: materials.add(StandardMaterial {
            base_color:Color::hex("e6007a").unwrap().into() ,
            // vary key PBR parameters on a grid of spheres to show the effect
        
            metallic: 0.2,
            perceptual_roughness: 0.2,
            ..default()
        }),
        transform: Transform::from_xyz(0.3, 1.5, 0.0),
        ..default()
    });


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

use web_sys::window;
use bevy::ecs::event::Events;
use bevy::input::mouse::MouseButtonInput;
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
        //.init_resource::<TrackInputState>()
            .add_system(capture_mouse_on_click.system());
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
