#[cfg(feature = "spacemouse")]
use crate::MovementSettings;
// use crate::{Anchor, Viewport, LAST_KEYSTROKE_TIME, PAUSE_DATA_FETCH};
use glam::{Quat, Vec3};

#[derive(Default)]
pub struct Destination {
	pub location: Option<Vec3>,
	pub look_at: Option<Quat>,
	// how many seconds should the transition take?
	//pub time: Option<f32>
	// pub set: bool
}

// Handles keyboard input and movement
// pub fn player_move_arrows(
// 	keys: Res<Input<KeyCode>>,
// 	time: Res<Time>,
// 	windows: Res<Windows>,
// 	datasource: Res<super::Sovereigns>,
// 	mut anchor: ResMut<Anchor>,
// 	settings: Res<MovementSettings>,
// 	mut query: Query<&mut Transform, With<Viewport>>,
// 	mut toggle_mouse_capture: ResMut<MouseCapture>,
// 	mut dest: ResMut<Destination>,
// ) {
// 	if let Some(window) = windows.get_primary() {
// 		for mut transform in query.iter_mut() {
// 			let mut velocity = Vec3::ZERO;

// 			if anchor.follow_chain {
// 				// If someone has recently pressed a key to move then don't try and move...
// 				if time.seconds_since_startup() as i32 - LAST_KEYSTROKE_TIME.load(Ordering::Relaxed) >
// 					2
// 				{
// 					let x = datasource.default_track_speed;
// 					velocity = Vec3::new(x, 0., 0.);
// 				}
// 			}

// 			if keys.just_released(KeyCode::Tab) {
// 				anchor.follow_chain = !anchor.follow_chain;
// 			}
// 			if keys.just_released(KeyCode::Escape) {
// 				toggle_mouse_capture.0 = !toggle_mouse_capture.0;
// 			}
// 			if keys.just_released(KeyCode::P) {
// 				let current = PAUSE_DATA_FETCH.load(Ordering::Relaxed);
// 				let new = if current == 0 { 1 } else { 0 };
// 				PAUSE_DATA_FETCH.store(new, Ordering::Relaxed);
// 				println!("fetching new data set to {}", new);
// 			}
// 			for _key in keys.get_pressed() {
// 				if window.is_focused() {
// 					LAST_KEYSTROKE_TIME
// 						.store(time.seconds_since_startup() as i32, Ordering::Relaxed);
// 					break
// 				}
// 			}
// 			if let Some(loc) = dest.location {
// 				let dist = loc.distance_squared(transform.translation);
// 				if dist < 50. {
// 					dest.location = None;
// 					return
// 				}
// 				// velocity = (loc - transform.translation).normalize();
// 				// TODO if near stop....
// 				// let current_look = transform.rotation.normalize();
// 				// let ideal_look_direction = Quat::from_euler(
// 				//     EulerRot::XYZ,
// 				//     -velocity.x,
// 				// -velocity.y,
// 				//     -velocity.z,
// 				// );

// 				// Current location
// 				// let mut camera: dolly::prelude::CameraRig<RightHanded> = CameraRig::builder()
// 				//     .with(Position::new(transform.translation))
// 				//     .with(YawPitch::new().rotation_quat(current_look))
// 				//     .with(Smooth::new_position(1.25).predictive(true))
// 				//     .with(LookAt::new(ideal_look_direction).tracking_smoothness(1.25))
// 				//     .build();

// 				//camera.driver_mut::<YawPitch>().set_rotation_quat(ideal_look_direction);
// 				// camera.driver_mut::<Position>().position = loc;

// 				// let final_transform = camera.update(time.delta_seconds());

// 				// println!("current loc: {} {}", transform.translation, current_look);

// 				//let smooth = dolly::drivers::Smooth::new_position_rotation(2.,2.);

// 				//velocity = velocity.normalize_or_zero();
// 				// transform.translation = final_transform.position;
// 				// transform.rotation = final_transform.rotation;
// 				// println!("dolly: {} {}",  final_transform.position,  final_transform.rotation);
// 				const SMOOTHNESS_MULT: f32 = 8.0;
// 				let smoothness_param: f32 = 3.;
// 				// Calculate the exponential blending based on frame time
// 				let interp_t = 1.0 -
// 					(-SMOOTHNESS_MULT * time.delta_seconds() / smoothness_param.max(1e-5)).exp();
// 				transform.translation = transform.translation.interpolate(loc, interp_t);
// 				//transform.translation += //transform.translation.interpolate(loc, interp_t);
// 				//	 velocity * time.delta_seconds() * settings.speed * dist.sqrt() / 5.;
// 				if let Some(look_at) = dest.look_at {
// 					//transform.rotation = transform.rotation.interpolate(look_at, interp_t);
// 					transform.rotation = transform.rotation.slerp(look_at, 0.05);
// 				} // println!("our step forward: {} ", velocity * time.delta_seconds() *
// 				 // settings.speed * 3.); println!("dest: {} {}", loc,  ideal_look_direction);
// 			} else {
// 				velocity = velocity.normalize_or_zero();
// 				transform.translation += velocity * time.delta_seconds() * settings.speed
// 			}
// 		}
// 	}
// }
