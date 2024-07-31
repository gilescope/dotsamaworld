use cgmath::{perspective, prelude::*, Matrix4, Point3, Rad, Vector3};

#[derive(Debug)]
pub struct Camera {
	/// Desired position to arrive at given a little time
	pub current_position: Point3<f32>,
	pub current_yaw: Rad<f32>,
	pub current_pitch: Rad<f32>,

	pub desired_position: Point3<f32>,
	pub desired_yaw: Rad<f32>,
	pub desired_pitch: Rad<f32>,
}

/*
		 width
		_____
		|	| height
		-----
		  | direction, fnear is distance in that direction..
		  . <- position (x,y,z model space)


		Get aabb of screen space:
		  create instance width * height
		 at camera.position + direction * fnear - (width/2, height/2,0)

		at zfar what's the size of box?

		get aabb points of near view - camera.position and then divide by znear, * zfar


*/
impl Camera {
	pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
		position: V,
		yaw: Y,
		pitch: P,
	) -> Self {
		let pos = position.into();
		let yaw = yaw.into();
		let pitch = pitch.into();

		Self {
			current_position: pos.clone(), current_yaw: yaw.clone(), current_pitch: pitch.clone(),
			desired_position: pos, desired_yaw: yaw, desired_pitch: pitch,
		}
	}

	pub fn calc_matrix(&self) -> Matrix4<f32> {
		let (sin_pitch, cos_pitch) = self.current_pitch.0.sin_cos();
		let (sin_yaw, cos_yaw) = self.current_yaw.0.sin_cos();
		Matrix4::look_to_rh(
			self.current_position,
			// direction.
			Vector3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
			Vector3::unit_y(),
		)
	}
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Projection {
	pub aspect: f32,
	pub fovy: Rad<f32>,
	pub znear: f32,
	pub zfar: f32,
}

impl Projection {
	pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
		Self { aspect: width as f32 / height as f32, fovy: fovy.into(), znear, zfar }
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		self.aspect = width as f32 / height as f32;
	}

	pub fn calc_matrix(&self) -> Matrix4<f32> {
		OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
	}
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
	// We can't use cgmath with bytemuck directly so we'll have
	// to convert the Matrix4 into a 4x4 f32 array
	view_proj: [[f32; 4]; 4],
	view_position: [f32; 4],
}

impl CameraUniform {
	pub fn new() -> Self {
		Self { view_position: [0.0; 4], view_proj: cgmath::Matrix4::identity().into() }
	}

	pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
		self.view_position = camera.current_position.to_homogeneous().into();
		self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
	}
}
