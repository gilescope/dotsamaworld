use crate::Anchor;
use bevy::core::Time;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::input::keyboard::KeyCode;
use std::sync::atomic::Ordering;
// use bevy::input::mouse::MouseWheel;
use bevy::input::Input;
use bevy::prelude::*;
use egui::Key;
// use dolly::prelude::*;
use crate::LAST_KEYSTROKE_TIME;
// use bevy::render::camera::CameraProjection;
#[cfg(feature = "spacemouse")]
use crate::MovementSettings;
use crate::Viewport;
use bevy::transform::components::Transform;
use bevy::window::Windows;
#[cfg(feature = "normalmouse")]
use bevy_flycam::MovementSettings;

pub struct MouseCapture(pub bool);

impl Default for MouseCapture {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Default)]
pub struct Destination {
    pub location: Option<Vec3>,
    pub look_at: Option<Quat>,
    // how many seconds should the transition take?
    //pub time: Option<f32>
    // pub set: bool
}

/// Handles keyboard input and movement
pub fn player_move_arrows(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    windows: Res<Windows>,
    datasource: Res<super::Sovereigns>,
    mut anchor: ResMut<Anchor>,
    mut settings: ResMut<MovementSettings>,
    mut query: Query<&mut Transform, With<Viewport>>,
    mut toggle_mouse_capture: ResMut<MouseCapture>,
    mut dest: ResMut<Destination>,
) {
    let window = windows.get_primary().unwrap();
    for mut transform in query.iter_mut() {
        let mut velocity = Vec3::ZERO;

        if !anchor.dropped {
            // If someone has recently pressed a key to move then don't try and move...
            if time.seconds_since_startup() as i64 - LAST_KEYSTROKE_TIME.load(Ordering::Relaxed) > 2 {
                let x = datasource.default_track_speed;
                velocity = Vec3::new(x, 0., 0.);
            }
        }

        // Don't change the Y axis.
        let forward = transform.forward();
        let right = transform.right();
        let forward = Vec3::new(forward.x, 0., forward.z);
        let right = Vec3::new(right.x, 0., right.z);

        if keys.just_released(KeyCode::Tab) {
            anchor.dropped = !anchor.dropped;
        }
        if keys.just_released(KeyCode::Escape) {
            toggle_mouse_capture.0 = !toggle_mouse_capture.0;
        }

        for key in keys.get_pressed() {
            if window.is_focused() {
                match key {
                    KeyCode::Up => velocity += forward,
                    KeyCode::Down => velocity -= forward,
                    KeyCode::Left => velocity -= right,
                    KeyCode::Right => velocity += right,
                    KeyCode::Space | KeyCode::Comma => {
                        if transform.local_y().y > 0. {
                            settings.speed += 0.5;
                        }
                    }
                    KeyCode::LShift | KeyCode::RShift | KeyCode::Period => {
                        if transform.local_y().y > 0. {
                            if settings.speed > 12. {
                                settings.speed -= 0.5;
                            }
                        }
                    }

                    _ => (),
                }
                LAST_KEYSTROKE_TIME.store(time.seconds_since_startup() as i64, Ordering::Relaxed);
            }
        }
        if let Some(loc) = dest.location {
            let dist = loc.distance_squared(transform.translation);
            if dist < 50. {
                dest.location = None;
                return;
            }
            velocity = (loc - transform.translation).normalize();
            // TODO if near stop....
            // let current_look = transform.rotation.normalize();
            // let ideal_look_direction = Quat::from_euler(
            //     EulerRot::XYZ,
            //     -velocity.x,
            // -velocity.y,
            //     -velocity.z,
            // );

            // Current location
            // let mut camera: dolly::prelude::CameraRig<RightHanded> = CameraRig::builder()
            //     .with(Position::new(transform.translation))
            //     .with(YawPitch::new().rotation_quat(current_look))
            //     .with(Smooth::new_position(1.25).predictive(true))
            //     .with(LookAt::new(ideal_look_direction).tracking_smoothness(1.25))
            //     .build();

            //camera.driver_mut::<YawPitch>().set_rotation_quat(ideal_look_direction);
            // camera.driver_mut::<Position>().position = loc;

            // let final_transform = camera.update(time.delta_seconds());

            // println!("current loc: {} {}", transform.translation, current_look);

            //let smooth = dolly::drivers::Smooth::new_position_rotation(2.,2.);

            //velocity = velocity.normalize_or_zero();
            // transform.translation = final_transform.position;
            // transform.rotation = final_transform.rotation;
            // println!("dolly: {} {}",  final_transform.position,  final_transform.rotation);

            transform.translation +=
                velocity * time.delta_seconds() * settings.speed * dist.sqrt() / 5.;
            transform.rotation = transform.rotation.slerp(dest.look_at.unwrap(), 0.05);
            // println!("our step forward: {} ", velocity * time.delta_seconds() * settings.speed * 3.);
            // println!("dest: {} {}", loc,  ideal_look_direction);
        } else {
            velocity = velocity.normalize_or_zero();
            transform.translation += velocity * time.delta_seconds() * settings.speed
        }
    }
}

// the mouse-scroll changes the field-of-view of the camera
// pub fn scroll(
//     mut mouse_wheel_events: EventReader<MouseWheel>,
//     windows: Res<Windows>,
//     mut query: Query<(&FlyCam, &mut Camera, &mut PerspectiveProjection)>,
// ) {
//     // for event in mouse_wheel_events.iter() {
//     //     for (_camera, mut camera, mut project) in query.iter_mut() {
//     //         project.fov = (project.fov - event.y * 0.01).abs();
//     //         let prim = windows.get_primary().unwrap();

//     //         //Calculate projection with new fov
//     //         project.update(prim.width(), prim.height());

//     //         //Update camera with the new fov
//     //         camera.projection_matrix = project.get_projection_matrix();
//     //         camera.depth_calculation = project.depth_calculation();

//     //         // println!("FOV: {:?}", project.fov);
//     //     }
//     // }
// }
