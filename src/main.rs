use bevy::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy_ecs::prelude::Component;

use bevy_flycam::PlayerPlugin;
use bevy_flycam::FlyCam;
use bevy::render::camera::CameraProjection;

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
    app.add_plugin(PlayerPlugin)
    .add_system(scroll);
    app.add_startup_system(setup.system());
    app.add_system(hello_world);
    app.run();


    // app.insert_resource(GreetTimer(Timer::from_seconds(2.0, true)))
    // .add_startup_system(add_people)
    // .add_system(greet_people);
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
    // camera
    // commands.spawn_bundle(PerspectiveCameraBundle {
    //     transform: Transform::from_translation(Vec3::new(-2.0, 2.5, 5.0))
    //         .looking_ at(Vec3::default(), Vec3::Y),
    //     ..Default::default()
    // });

    //spawn_camera(commands);
}
