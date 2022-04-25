use bevy::core::Time;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseWheel;
use bevy::input::Input;
use bevy::prelude::*;
use bevy::render::camera::CameraProjection;
use bevy::transform::components::Transform;
use bevy::window::Windows;
use bevy_flycam::FlyCam;
use bevy_flycam::MovementSettings;

pub struct MouseCapture(pub bool);

impl Default for MouseCapture {
    fn default() -> Self {
        Self(true)
    }
}

/// Handles keyboard input and movement
pub fn player_move_arrows(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    windows: Res<Windows>,
    mut settings: ResMut<MovementSettings>,
    mut query: Query<&mut Transform, With<FlyCam>>,
    mut toggle_mouse_capture: ResMut<MouseCapture>,
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
                    KeyCode::Space => {
                        if transform.local_y().y > 0. {
                            settings.speed += 0.5;
                        }
                    }
                    KeyCode::LShift => {
                        if transform.local_y().y > 0. {
                            if settings.speed > 12. {
                                settings.speed -= 0.5;
                            }
                        }
                    }
                    KeyCode::Escape => {
                        toggle_mouse_capture.0 = !toggle_mouse_capture.0;
                    }
                    _ => (),
                }
            }
        }

        velocity = velocity.normalize_or_zero();

        transform.translation += velocity * time.delta_seconds() * settings.speed
    }
}

/// the mouse-scroll changes the field-of-view of the camera
pub fn scroll(
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

            // println!("FOV: {:?}", project.fov);
        }
    }
}
