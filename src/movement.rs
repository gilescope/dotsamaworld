#[cfg(feature = "spacemouse")]
use crate::MovementSettings;
use crate::{Anchor, Viewport, LAST_KEYSTROKE_TIME, PAUSE_DATA_FETCH};
use bevy::{
	core::Time,
	ecs::system::{Query, Res},
	input::{keyboard::KeyCode, mouse::MouseWheel, Input},
	prelude::*,
	transform::components::Transform,
	window::Windows,
};
#[cfg(feature = "normalmouse")]
use bevy_flycam::MovementSettings;
use std::sync::atomic::Ordering;
pub struct MouseCapture(pub bool);

impl Default for MouseCapture {
	fn default() -> Self {
		Self(true)
	}
}

// From dolly but not exposed
// https://github.com/h3r2tic/dolly/blob/0276fb4b5fe2e11c70919af7dbbcd645711c8f48/src/util.rs
pub(crate) trait Interpolate {
	fn interpolate(self, other: Self, t: f32) -> Self;
}

impl Interpolate for Vec3 {
	fn interpolate(self, other: Self, t: f32) -> Self {
		Vec3::lerp(self, other, t)
	}
}

impl Interpolate for Quat {
	fn interpolate(self, other: Self, t: f32) -> Self {
		// Technically should be a `slerp` for framerate independence, but the latter
		// will rotate in the negative direction when interpolating a 180..360 degree rotation
		// to the 0..180 range. See the comment about `yaw_degrees` in `YawPitch` for more details.
		Quat::lerp(self.normalize(), other.normalize(), t).normalize()
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
	settings: Res<MovementSettings>,
	mut query: Query<&mut Transform, With<Viewport>>,
	mut toggle_mouse_capture: ResMut<MouseCapture>,
	mut dest: ResMut<Destination>,
) {
	let window = windows.get_primary().unwrap();
	for mut transform in query.iter_mut() {
		let mut velocity = Vec3::ZERO;

		if anchor.follow_chain {
			// If someone has recently pressed a key to move then don't try and move...
			if time.seconds_since_startup() as i32 - LAST_KEYSTROKE_TIME.load(Ordering::Relaxed) > 2
			{
				let x = datasource.default_track_speed;
				velocity = Vec3::new(x, 0., 0.);
			}
		}

		// Don't change the Y axis.
		// let forward = transform.forward();
		// let right = transform.right();
		// let forward = Vec3::new(forward.x, 0., forward.z);
		// let right = Vec3::new(right.x, 0., right.z);

		if keys.just_released(KeyCode::Tab) {
			anchor.follow_chain = !anchor.follow_chain;
		}
		if keys.just_released(KeyCode::Escape) {
			toggle_mouse_capture.0 = !toggle_mouse_capture.0;
		}
		if keys.just_released(KeyCode::P) {
			let current = PAUSE_DATA_FETCH.load(Ordering::Relaxed);
			let new = if current == 0 { 1 } else { 0 };
			PAUSE_DATA_FETCH.store(new, Ordering::Relaxed);
			println!("fetching new data set to {}", new);
		}
		for _key in keys.get_pressed() {
			if window.is_focused() {
				// match key {
				//     // KeyCode::Up => velocity += forward,
				//     // KeyCode::Down => velocity -= forward,
				//     // KeyCode::Left => velocity -= right,
				//     // KeyCode::Right => velocity += right,
				//     // KeyCode::Space | KeyCode::Comma => {
				//     //     if transform.local_y().y > 0. {
				//     //         settings.speed += 0.5;
				//     //     }
				//     // }
				//     // KeyCode::LShift | KeyCode::RShift | KeyCode::Period => {
				//     //     if transform.local_y().y > 0. {
				//     //         if settings.speed > 12. {
				//     //             settings.speed -= 0.5;
				//     //         }
				//     //     }
				//     // }

				// _ => (),
				// }
				LAST_KEYSTROKE_TIME.store(time.seconds_since_startup() as i32, Ordering::Relaxed);
				break
			}
		}
		if let Some(loc) = dest.location {
			let dist = loc.distance_squared(transform.translation);
			if dist < 50. {
				dest.location = None;
				return
			}
			// velocity = (loc - transform.translation).normalize();
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
			const SMOOTHNESS_MULT: f32 = 8.0;
			let smoothness_param: f32 = 3.;
			// Calculate the exponential blending based on frame time
			let interp_t =
				1.0 - (-SMOOTHNESS_MULT * time.delta_seconds() / smoothness_param.max(1e-5)).exp();
			transform.translation = transform.translation.interpolate(loc, interp_t);
			//transform.translation += //transform.translation.interpolate(loc, interp_t);
			//	 velocity * time.delta_seconds() * settings.speed * dist.sqrt() / 5.;
			if let Some(look_at) = dest.look_at {
				//transform.rotation = transform.rotation.interpolate(look_at, interp_t);
				transform.rotation = transform.rotation.slerp(look_at, 0.05);
			} // println!("our step forward: {} ", velocity * time.delta_seconds() * settings.speed *
			 // 3.); println!("dest: {} {}", loc,  ideal_look_direction);
		} else {
			velocity = velocity.normalize_or_zero();
			transform.translation += velocity * time.delta_seconds() * settings.speed
		}
	}
}

/// the mouse-scroll does not change the field-of-view of the camera
/// because if you change that too far the world goes inside out.
/// Instead scroll moves forwards or backwards.
pub fn scroll(
	time: Res<Time>,
	mut mouse_wheel_events: EventReader<MouseWheel>,
	mut query: Query<&mut Transform, With<Viewport>>,
) {
	for event in mouse_wheel_events.iter() {
		LAST_KEYSTROKE_TIME.store(time.seconds_since_startup() as i32, Ordering::Relaxed);
		for mut viewport in query.iter_mut() {
			let forward = viewport.forward();
			viewport.translation += forward * event.y;
		}
	}
}
