pub mod details;
pub mod doturl;
pub mod toggle;
use crate::{Anchor, Inspector, Viewport};
use bevy::prelude::*;
use bevy_egui::EguiContext;
use bevy_inspector_egui::{options::StringAttributes, Inspectable};
use chrono::{DateTime, NaiveDateTime, Utc};
pub use details::Details;
pub use doturl::DotUrl;
use egui_datepicker::DatePicker;
use std::ops::DerefMut;
#[derive(Default)]
pub struct OccupiedScreenSpace {
	left: f32,
	top: f32,
	right: f32,
	bottom: f32,
}

pub struct OriginalCameraTransform(pub Transform);

pub fn ui_bars_system(
	mut egui_context: ResMut<EguiContext>,
	mut occupied_screen_space: ResMut<OccupiedScreenSpace>,
	viewpoint_query: Query<&GlobalTransform, With<Viewport>>,
	mut spec: ResMut<UrlBar>,
	mut anchor: ResMut<Anchor>,
	inspector: Res<Inspector>,
) {
	// occupied_screen_space.left = egui::SidePanel::left("left_panel")
	//     .resizable(true)
	//     .show(egui_context.ctx_mut(), |ui| {
	//         ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
	//     })
	//     .response
	//     .rect
	//     .width();
	// occupied_screen_space.right = egui::SidePanel::right("right_panel")
	//     .resizable(true)
	//     .show(egui_context.ctx_mut(), |ui| {
	//         ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
	//     })
	//     .response
	//     .rect
	//     .width();
	occupied_screen_space.top = egui::TopBottomPanel::top("top_panel")
		.resizable(false)
		.show(egui_context.ctx_mut(), |ui| {
			// ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
			ui.horizontal(|ui| {
				let timestamp =
					super::x_to_timestamp(viewpoint_query.get_single().unwrap().translation.x);
				let mut naive = NaiveDateTime::from_timestamp(timestamp as i64, 0);
				ui.add(
					DatePicker::<std::ops::Range<NaiveDateTime>>::new(
						"noweekendhighlight",
						&mut naive,
					)
					.highlight_weekend(false),
				);
				ui.text_edit_singleline(&mut spec.location);
				ui.with_layout(egui::Layout::right_to_left(), |ui| {
					ui.add(toggle::toggle(&mut anchor.deref_mut().follow_chain));
					ui.heading("Follow:");
					// spec.location.deref_mut().ui(ui, StringAttributes { multiline: false },
					// egui_context);
				});
			});
		})
		.response
		.rect
		.height();
	occupied_screen_space.bottom = egui::TopBottomPanel::bottom("bottom_panel")
		.resizable(false)
		.show(egui_context.ctx_mut(), |ui| {
			ui.horizontal(|ui| {
				if let Some(selected) = &inspector.hovered {
					ui.heading(selected);
				}
				ui.with_layout(egui::Layout::right_to_left(), |ui| {
					let timestamp =
						super::x_to_timestamp(viewpoint_query.get_single().unwrap().translation.x);
					let naive = NaiveDateTime::from_timestamp(timestamp as i64, 0);
					let datetime: DateTime<chrono::Utc> = DateTime::from_utc(naive, Utc);
					let datetime: DateTime<chrono::Local> = datetime.into();

					let newdate = datetime.format("%Y-%m-%d %H:%M:%S");
					ui.heading(format!("{}", newdate));
				});
			});
		})
		.response
		.rect
		.height();
}

// TODO: Something like this would probably stop us rendering
// behind the footer and header.
// pub fn update_camera_transform_system(
//     occupied_screen_space: Res<OccupiedScreenSpace>,
//     original_camera_transform: Res<OriginalCameraTransform>,
//     windows: Res<Windows>,
//     // cam_query: Query<(&Viewport)>,
//     mut camera_query: Query<(&PerspectiveProjection, &mut Transform)>,
// ) {
//     // let cam = cam_query.get_single().unwrap();
//     let (camera_projection, mut transform) = camera_query.get_single_mut().unwrap();

//     let distance_to_target = (/*CAMERA_TARGET -*/
// original_camera_transform.0.translation).length();     let frustum_height = 2.0 *
// distance_to_target * (camera_projection.fov * 0.5).tan();     let frustum_width = frustum_height
// * camera_projection.aspect_ratio;

//     let window = windows.get_primary().unwrap();

//     let left_taken = occupied_screen_space.left / window.width();
//     let right_taken = occupied_screen_space.right / window.width();
//     let top_taken = occupied_screen_space.top / window.height();
//     let bottom_taken = occupied_screen_space.bottom / window.height();
//     transform.translation = original_camera_transform.0.translation
//         + transform.rotation.mul_vec3(Vec3::new(
//             (right_taken - left_taken) * frustum_width * 0.5,
//             (top_taken - bottom_taken) * frustum_height * 0.5,
//             0.0,
//         ));
// }

// impl Default for UrlBar {
// 	fn default() -> Self {
// 		Self {
// 			//dotsama:/1//10504605 doesn't stop.
// 			//dotsama:/1//10504599 stops after 12 blocks
// 			location: "dotsama:/1//10504599".to_string(),
// 			changed: false,
// 		}
// 	}
// }

pub struct UrlBar {
	pub changed: bool,
	pub location: String,
	pub timestamp: Option<u64>,
}
// use bevy_inspector_egui::{options::StringAttributes, Context};
// use egui::Grid;
// impl Inspectable for UrlBar {
// 	type Attributes = ();

// 	fn ui(
// 		&mut self,
// 		ui: &mut bevy_egui::egui::Ui,
// 		_options: Self::Attributes,
// 		context: &mut Context,
// 	) -> bool {
// 		let mut changed = false;
// 		ui.vertical_centered(|ui| {
// 			Grid::new(context.id()).min_col_width(400.).show(ui, |ui| {
// 				// ui.label("Pallet");
// 				changed |= self.location.ui(ui, StringAttributes { multiline: false }, context);

// 				ui.end_row();

// 				if ui.button("Time travel").clicked() {
// 					self.changed = true;
// 					println!("clicked {}", &self.location);
// 				};
// 				ui.end_row();
// 				if ui.button("Live").clicked() {
// 					self.changed = true;
// 					self.location = LIVE.into();
// 					println!("clicked {}", &self.location);
// 				};
// 				ui.end_row();
// 				if ui.button("Clear").clicked() {
// 					self.changed = true;
// 					self.location = "".into();
// 					println!("clicked {}", &self.location);
// 				};
// 				ui.end_row();
// 			});
// 		});
// 		changed
// 	}
// }
