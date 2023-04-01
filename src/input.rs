use crate::{camera::Camera, log};
use cgmath::{InnerSpace, Rad, Vector3};
use core::f32::consts::FRAC_PI_2;
use winit::{
	dpi::PhysicalPosition,
	event::{ElementState, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent},
};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct CameraController {
	pub amount_left: f32,
	amount_right: f32,
	amount_forward: f32,
	amount_backward: f32,
	pub amount_up: f32,
	amount_down: f32,
	pub rotate_horizontal: f32,
	pub rotate_vertical: f32,
	scroll: f32,
	speed: f32,
	boost: f32,
	boost_pressed: bool,
	sensitivity: f32,

	/// stack of per frame changes to do so that big changes are smoothed
	pub rotate_horizontal_stack: Vec<f32>,

	/// stack of per frame changes to do so that big changes are smoothed
	pub rotate_vertical_stack: Vec<f32>,
}

impl CameraController {
	pub fn new(speed: f32, sensitivity: f32) -> Self {
		Self {
			amount_left: 0.0,
			amount_right: 0.0,
			amount_forward: 0.0,
			amount_backward: 0.0,
			amount_up: 0.0,
			amount_down: 0.0,
			rotate_horizontal: 0.0,
			rotate_vertical: 0.0,
			scroll: 0.0,
			speed,
			boost: 4.,
			boost_pressed: false,
			sensitivity,

			rotate_horizontal_stack: vec![],
			rotate_vertical_stack: vec![],
		}
	}

	fn boost(&self) -> f32 {
		if self.boost_pressed {
			self.boost
		} else {
			1.0
		}
	}

	pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
		if let VirtualKeyCode::LShift = key {
			if state == ElementState::Pressed {
				self.boost_pressed = true;
			} else {
				self.boost_pressed = false;
			}
		}
		let amount = if state == ElementState::Pressed { 0.1 * self.boost() } else { 0.0 };
		match key {
			VirtualKeyCode::W | VirtualKeyCode::Up => {
				self.amount_forward = amount;
				true
			},
			VirtualKeyCode::S | VirtualKeyCode::Down => {
				self.amount_backward = amount;
				true
			},
			VirtualKeyCode::A | VirtualKeyCode::Left => {
				self.amount_left = amount;
				true
			},
			VirtualKeyCode::D | VirtualKeyCode::Right => {
				self.amount_right = amount;
				true
			},
			VirtualKeyCode::Space => {
				self.amount_up = amount;
				true
			},
			VirtualKeyCode::LShift => {
				self.amount_down = amount;
				true
			},
			_ => false,
		}
	}

	pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
		self.rotate_horizontal = mouse_dx as f32 / 4.;
		self.rotate_vertical = mouse_dy as f32 / 4.;
	}

	pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
		// Macs generally have their scroll inverted.
		#[cfg(target_os = "macos")]
		let invert = 1.0;
		#[cfg(not(target_os = "macos"))]
		let invert = -1.0;
		self.scroll = -match delta {
			// I'm assuming a line is about 100 pixels
			MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
			MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
		} * self.boost() *
			invert;
	}

	pub fn update_camera(&mut self, camera: &mut Camera, dt: chrono::Duration) {
		let dt = (dt.num_milliseconds() as f32 / 1000.) + dt.num_seconds() as f32;

		// Move forward/backward and left/right
		let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
		let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
		let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
		camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
		camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

		// Move in/out (aka. "zoom")
		// Note: this isn't an actual zoom. The camera's position
		// changes when zooming. I've added this to make it easier
		// to get closer to an object you want to focus on.
		let (pitch_sin, pitch_cos) = camera.pitch.0.sin_cos();
		let scrollward =
			Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
		camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
		self.scroll = 0.0;

		// Move up/down. Since we don't use roll, we can just
		// modify the y coordinate directly.
		camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;
		if camera.position.y < 3.0 {
			camera.position.y = 3.0;
		}

		if let Some(val) = self.rotate_horizontal_stack.pop() {
			self.rotate_horizontal += val;
		}
		if let Some(val) = self.rotate_vertical_stack.pop() {
			self.rotate_vertical += val;
		}
		// Rotate
		camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
		camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

		// If process_mouse isn't called every frame, these values
		// will not get set to zero, and the camera will rotate
		// when moving in a non cardinal direction.
		self.rotate_horizontal = 0.0;
		self.rotate_vertical = 0.0;

		// Keep the camera's angle from going too high/low.
		if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
			camera.pitch = -Rad(SAFE_FRAC_PI_2);
		} else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
			camera.pitch = Rad(SAFE_FRAC_PI_2);
		}
	}
}

pub(crate) fn input(
	camera_controller: &mut CameraController,
	event: &WindowEvent,
	mouse_pressed: &mut bool,
) -> bool {
	match event {
		WindowEvent::KeyboardInput {
			input: KeyboardInput { virtual_keycode: Some(key), state, .. },
			..
		} => camera_controller.process_keyboard(*key, *state),
		WindowEvent::MouseWheel { delta, .. } => {
			camera_controller.process_scroll(delta);
			true
		},
		WindowEvent::MouseInput { button: winit::event::MouseButton::Left, state, .. } => {
			*mouse_pressed = *state == ElementState::Pressed;
			true
		},
		WindowEvent::Resized(new_size) => {
			log!("Window event: new size: width {} height {}", new_size.width, new_size.height);
			true
		},
		_ => false,
	}
}
