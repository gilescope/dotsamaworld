use bevy::diagnostic::Diagnostics;

pub mod details;
pub mod doturl;
pub mod toggle;
//  use egui::ImageData;
use crate::{Anchor, Env, Inspector, Viewport, log};
use bevy::prelude::*;
use bevy_egui::EguiContext;
// use bevy_inspector_egui::{options::StringAttributes, Inspectable};
use crate::Destination;
use chrono::{DateTime, NaiveDateTime, Utc};
pub use details::Details;
pub use doturl::DotUrl;
use egui::ComboBox;
// use egui::ComboBox;
use egui_datepicker::DatePicker;
use std::ops::DerefMut;
#[derive(Default)]
pub struct OccupiedScreenSpace {
	left: f32,
	top: f32,
	// right: f32,
	bottom: f32,
}
macro_rules! log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

pub struct OriginalCameraTransform(pub Transform);

pub fn ui_bars_system(
	mut egui_context: ResMut<EguiContext>,
	mut occupied_screen_space: ResMut<OccupiedScreenSpace>,
	viewpoint_query: Query<&GlobalTransform, With<Viewport>>,
	mut spec: ResMut<UrlBar>,
	mut anchor: ResMut<Anchor>,
	mut inspector: ResMut<Inspector>,
	entities: Query<(&GlobalTransform, &Details)>,
	mut destination: ResMut<Destination>,
	diagnostics: Res<Diagnostics>,
) {
	if inspector.selected.is_some() {
		occupied_screen_space.left = egui::SidePanel::left("left_panel")
			.resizable(true)
			.show(egui_context.ctx_mut(), |ui| {
				// ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());

				// ui.horizontal(|ui| {
				// 	// ui.heading("Selected:");
				// });
				// ui.separator();
				

				// if inspector.selected.is_some() {
				// let name = inspector.selected.as_ref().map(|d| d.doturl.chain_str()).unwrap();

				// #[cfg(target_arch = "wasm32")]
				// let maybe_bytes: Option<Vec<u8>> = None;
				// let maybe_bytes = {
				// 	let uri = &format!("https://cloudflare-ipfs.com/ipfs/Qmb1GG87ufHEvXkarzYoLn9NYRGntgZSfvJSBvdrbhbSNe/{}.jpeg", chain_str);
				// 	use wasm_bindgen::JsCast;
				// 	use wasm_bindgen_futures::JsFuture;
				// 	let window = web_sys::window().unwrap();
				// 	let resp_value = JsFuture::from(window.fetch_with_str(uri)).await.unwrap();
				// 	let resp: web_sys::Response = resp_value.dyn_into().unwrap();
				// 	let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
				// 	Some(js_sys::Uint8Array::new(&data).to_vec())
				// };

				// #[cfg(not(target_arch = "wasm32"))]
				// let maybe_bytes = std::fs::read(&format!("assets/branding/{}.jpeg", name)).ok();

				// if let Some(bytes) = maybe_bytes {
				// 	let img = egui_extras::image::load_image_bytes(bytes.as_slice()).unwrap();
				// 	let _texture: &egui::TextureHandle =
				// 		inspector.texture.get_or_insert_with(|| {
				// 			// Load the texture only once.
				// 			ui.ctx().load_texture(name, egui::ImageData::Color(img))
				// 		});
				// }
				// }
use egui::Link;
				if let Some(selected) = &inspector.selected {					
					ui.heading(&selected.variant);
					ui.heading(&selected.pallet);
					ui.separator();
					// ui.hyperlink_to("s", &selected.url); not working on linux at the moment so use open.				
					if ui.add(Link::new("open in polkadot.js")).clicked() {
						open::that(&selected.url).unwrap();
					}
					if let Some(val) = &selected.value {
						// ui.add(|ui| Tree(val.clone()));
						// ui.collapsing(
						// 	"value", 	|
							funk(ui, 
								&scale_value_to_borrowed::convert(val,true));
//             .default_open(depth < 1)
						ui.label(&val.to_string());
						ui.label(&scale_value_to_borrowed::convert(val,true).to_string());
					}
					// ui.add(egui::TextEdit::multiline(&mut  selected.url.as_ref()));
					ui.label("RAW Scale:");
					
					if ui.button("📋").clicked() {
						let s = hex::encode(&selected.raw);
						log!("{}", &s);
						ui.output().copied_text = s;//TODO not working...
					};
					ui.add(egui::TextEdit::multiline(&mut hex::encode(&selected.raw)));

					ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
						if let Some(hand) = inspector.texture.as_ref() {
							let texture: &egui::TextureHandle = hand;

							let l = 200.; // occupied_screen_space.left - 10.;
							ui.add(egui::Image::new(texture, egui::Vec2::new(l, l / 3.)));
						}
					});
				}
			})
			.response
			.rect
			.width();
	}
	// occupied_screen_space.right = egui::SidePanel::right("right_panel")
	//     .resizable(true)
	//     .show(egui_context.ctx_mut(), |ui| {
	//         ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
	//     })
	//     .response
	//     .rect
	//     .width();

	let mut fps = 0.;
	for diag in diagnostics.iter() {
		if diag.name == "fps" {
			fps = diag.value().unwrap_or_default();
			break
		}
	}

	occupied_screen_space.top = egui::TopBottomPanel::top("top_panel")
		.resizable(false)
		.show(egui_context.ctx_mut(), |ui| {
			// ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
			ui.horizontal(|ui| {
				let _combo = ComboBox::from_label("Env")
					.selected_text(format!("{}", spec.env))
					.show_ui(ui, |ui| {
						ui.selectable_value(&mut spec.env, Env::Prod, "dotsama");
						// ui.selectable_value(&mut spec.env, Env::SelfSovereign, "independents");
						// ui.selectable_value(&mut spec.env, Env::Test, "test");
						ui.selectable_value(&mut spec.env, Env::Local, "local");
					});

				ui.add(
					DatePicker::<std::ops::Range<NaiveDateTime>>::new(
						"noweekendhighlight",
						&mut spec.timestamp,
					)
					.highlight_weekend(false),
				);

				//TODO: location = alpha blend to 10% everything but XXXX
				let response = ui.text_edit_singleline(&mut spec.find);
				let mut found = 0;
				if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
					if spec.find.len() <= 4 {
						if let Ok(para_id) = spec.find.parse() {
							for (loc, details) in entities.iter() {
								if details.doturl.para_id == Some(para_id) &&
									details.doturl.block_number.is_some()
								{
									destination.location = Some(loc.translation());
									inspector.selected = Some(details.clone());
									found += 1;
								}
							}
						}
					}
					for (loc, details) in entities.iter() {
						if spec.find.len() <= details.pallet.len() && spec.find.as_bytes().eq_ignore_ascii_case(&details.pallet.as_bytes()[..spec.find.len()])
						// if details.pallet.contains(&spec.find) || details.variant.contains(&spec.find)
						{
							destination.location = Some(loc.translation());
							inspector.selected = Some(details.clone());
							found += 1;
						}
					}

					println!("find {}", spec.find);
				}
				if !spec.find.is_empty() {
					ui.heading(format!("found: {}", found));
				}
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
					let x = viewpoint_query.get_single().unwrap().translation().x;
					let y = viewpoint_query.get_single().unwrap().translation().x;
					let z = viewpoint_query.get_single().unwrap().translation().z;

					let timestamp = super::x_to_timestamp(
						viewpoint_query.get_single().unwrap().translation().x,
					);
					let naive = NaiveDateTime::from_timestamp(timestamp as i64, 0);
					let datetime: DateTime<chrono::Utc> = DateTime::from_utc(naive, Utc);
					let datetime: DateTime<chrono::Local> = datetime.into();

					let newdate = datetime.format("%Y-%m-%d %H:%M:%S");
					ui.heading(format!("x={:03.0} y={:03.0} z={:03.0} {:03.0} fps. {}", x,y,z,fps, newdate));
				});
			});
		})
		.response
		.rect
		.height();
}
use egui::Ui;

fn funk<'r>(ui: &'r mut Ui, val: &scale_borrow::Value) -> () {
	match &val {
		scale_borrow::Value::Object(ref pairs) => {
			if pairs.len() == 1 {
				let mut header = String::new();
				let (mut k, v) = &pairs[0];
				let mut v : &scale_borrow::Value = &v;
					
				while let scale_borrow::Value::Object(nested_pairs) = &v && nested_pairs.len() == 1 {
					header.push_str(k);
					header.push('.');
					let (nk, nv) = &nested_pairs[0];
					k = nk;
					v = &nv;
				}
				header.push_str(k);
				// use egui::CollapsingHeader;
				ui.collapsing(header, |ui|{
					funk(ui, &v);
				});
			} else {
				for (mut k, v) in pairs.iter() {
					if let scale_borrow::Value::Object(nested_pairs) = &v {
						let mut header = String::new();
						let mut v : &scale_borrow::Value = &v;
							
						while let scale_borrow::Value::Object(nested_pairs) = &v && nested_pairs.len() == 1 {
							header.push_str(k);
							header.push('.');
							let (nk, nv) = &nested_pairs[0];
							k = nk;
							v = &nv;
						}
						header.push_str(k);
						// use egui::CollapsingHeader;
						ui.collapsing(header, |ui|{
							funk(ui, &v);
						});
					} else {
						ui.collapsing(k, |ui|{
							funk(ui, v);
						});
					}
				}
			}
		}
		scale_borrow::Value::ScaleOwned(bytes) => {
			ui.label(format!("0x{}", hex::encode(bytes.as_slice())));
		}
		_ => {
			ui.label(val.to_string());
		}
	}
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
	// Maybe this is a find?
	pub location: String,
	was_location: String,

	pub find: String,

	pub timestamp: NaiveDateTime,
	was_timestamp: NaiveDateTime,
	pub env: Env,
	was_env: Env,
}

impl UrlBar {
	pub fn new(location: String, timestamp: NaiveDateTime, env: Env) -> Self {
		let loc_clone = location.clone();
		Self {
			location,
			was_location: loc_clone,
			find: String::new(),
			timestamp,
			was_timestamp: timestamp,
			env: env.clone(),
			was_env: env,
		}
	}

	pub fn timestamp(&self) -> Option<i64> {
		//self.timestamp.map(|timestamp| {
		let datetime: DateTime<chrono::Utc> = DateTime::from_utc(self.timestamp, Utc);
		Some(datetime.timestamp() * 1000)
		//})
	}

	pub fn changed(&self) -> bool {
		self.was_location != self.location ||
			self.was_timestamp != self.timestamp ||
			self.was_env != self.env
	}

	pub fn reset_changed(&mut self) {
		self.was_location = self.location.clone();
		self.was_timestamp = self.timestamp;
		self.was_env = self.env.clone();
	}
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
