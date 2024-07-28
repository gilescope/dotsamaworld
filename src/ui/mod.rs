// use bevy::diagnostic::Diagnostics;

pub mod details;
pub mod doturl;
use crate::{log, Anchor, ChainInfo, Env, Inspector, FREE_TXS};
use cgmath::Point3;
use chrono::{DateTime, NaiveDateTime};
pub use details::Details;
pub use doturl::DotUrl;
use std::{collections::HashMap, sync::atomic::Ordering};
// use std::num::NonZeroU32;
use egui::ComboBox;
// use egui_datepicker::DatePicker;
use std::ops::DerefMut;
#[derive(Default)]
pub struct OccupiedScreenSpace {
	pub left: f32,
	pub top: f32,
	//pub right: f32,
	pub bottom: f32,
}

pub fn ui_bars_system(
	egui_context: &mut egui::Context,
	occupied_screen_space: &mut OccupiedScreenSpace,
	viewpoint: &Point3<f32>,
	spec: &mut UrlBar,
	mut anchor: &mut Anchor,
	inspector: &mut Inspector,
	// _destination: &mut Destination,
	fps: u32,
	tps: u32,
	selected_details: Vec<(u32, Details, ChainInfo)>,
) {
	if !selected_details.is_empty() {
		occupied_screen_space.left = egui::SidePanel::left("left_panel")
			.resizable(true)
			.show(egui_context, |ui| {
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

				if let Some((_cube_index, selected, chain_info)) = selected_details.first() {
					ui.heading(&selected.variant);
					ui.heading(&selected.pallet);
					ui.separator();
					let chain_tuple = (
						selected.doturl.souverign_index(),
						selected.doturl.para_id.unwrap_or(0) as i32,
					);

					if ui.add(Link::new(format!("#{}", selected.doturl))).clicked() {
						open_url(&format!("#{}", &selected.doturl));
					}

					// ui.hyperlink_to("s", &selected.url); not working on linux at the moment so
					// use open.
					// if ui.add(Link::new("open in polkadot.js")).clicked() {
					// 	log!("click detected");
					// 	if let Err(e) = open::that(&selected.url) {
					// 		log!("Error opening link {:?}", e);
					// 	}
					// }
					if let Some(vall) = &selected.value {
						let (val, _s) = scale_value::stringify::from_str(vall);
						if let Ok(val) = val {
							let val_decoded = scale_value_to_borrowed::convert(&val, true);

							if let Some(v) = val_decoded.expect3("Ethereum", "0", "Executed") {
								if let Some(tx_hash) = v.find2("transaction_hash", "0") {
									if let scale_borrow::Value::ScaleOwned(tx) = tx_hash {
										let mut eth_tx_map = HashMap::new();
										eth_tx_map.insert(
											(1, 2006),
											"https://blockscout.com/astar//tx/0x{}",
										);
										eth_tx_map.insert((1, 2004), "https://moonscan.io/tx/0x{}");
										eth_tx_map.insert(
											(0, 2023),
											"https://moonriver.moonscan.io/tx/0x{}",
										);

										if let Some(url) = eth_tx_map.get(&chain_tuple) {
											let tx_hash = hex::encode(&tx[..]);
											if ui
												.add(Link::new(format!(
													"Transaction Hash #: 0x{}",
													tx_hash
												)))
												.clicked()
											{
												let url = url.replace("{}", &tx_hash);

												open_url(&url);
											}
										}
									}
								}
							}

							funk(ui, &val_decoded);
						} else {
							ui.heading(vall);
						}
						//             .default_open(depth < 1)
						// ui.label(&val.to_string());
						// ui.label(&scale_value_to_borrowed::convert(&val,true).to_string());
					}
					// ui.add(egui::TextEdit::multiline(&mut  selected.url.as_ref()));
					// ui.label("RAW Scale:");

					if let Some(event) = selected.doturl.event {
						ui.label(format!("Event #: {}", event));
					}
					if let Some(extrinsic) = selected.doturl.extrinsic {
						if selected.raw.is_empty() {
							ui.label(format!("Extrinsic #: {}", extrinsic));
						} else {
							if ui
								.add(Link::new(format!("Decode Extrinsic #: {}", extrinsic)))
								.clicked()
							{
								let encoded: String =
									form_urlencoded::Serializer::new(String::new())
										.append_pair("rpc", &chain_info.chain_ws[0])
										.finish();

								// let is_relay = chain_info.chain_url.is_relay();
								let url = format!(
									"https://polkadot.js.org/apps/?{}#/extrinsics/decode/0x{}",
									&encoded,
									&hex::encode(&selected.raw)
								);

								open_url(&url);
							}
						}
					}
					if let Some(block_number) = selected.doturl.block_number {
						if ui.add(Link::new(format!("See Block #: {}", block_number))).clicked() {
							log!("click block detected");
							let encoded: String = form_urlencoded::Serializer::new(String::new())
								.append_pair("rpc", &chain_info.chain_ws[0])
								.finish();

							// let is_relay = chain_info.chain_url.is_relay();
							let url = format!(
								"https://polkadot.js.org/apps/?{}#/explorer/query/{}",
								&encoded, block_number
							);

							open_url(&url);
						}

						let mut block_explore_map = HashMap::new();
						block_explore_map.insert((1, 2004), "https://moonscan.io/block/{}");
						block_explore_map
							.insert((0, 2023), "https://moonriver.moonscan.io/block/{}");
						block_explore_map
							.insert((0, 1000), "https://statemine.statescan.io/block/{}");
						block_explore_map
							.insert((1, 1000), "https://statemint.statescan.io/block/{}");

						if let Some(para_id) = selected.doturl.para_id {
							if let Some(url) =
								block_explore_map.get(&(selected.doturl.souverign_index(), para_id))
							{
								if ui
									.add(Link::new(format!(
										"Local block explore #: {}",
										block_number
									)))
									.clicked()
								{
									log!("click block detected");

									let url = url.replace("{}", &block_number.to_string());

									open_url(&url);
								}
							}
						}
					}
					if let Some(para_id) = selected.doturl.para_id {
						ui.label(format!("Para Id: {}", para_id));
					}
					if let Some(sovereign) = selected.doturl.sovereign {
						if sovereign == -1 {
							if ui.add(Link::new("Kusama Relay Chain")).clicked() {
								open_url("https://kusama.network/");
							}
						} else if sovereign == 1 {
							if ui.add(Link::new("Polkadot Relay Chain")).clicked() {
								open_url("https://polkadot.network/");
							}
						} else {
							ui.label(format!("Relay Id: {}", sovereign));
						}
					}

				// if ui.button("📋").clicked() {
				// 	let s = hex::encode(&selected.raw);
				// 	log!("{}", &s);
				// 	ui.output().copied_text = s; //TODO not working...
				// };
				//TODO: request raw from webworker!!!
				// ui.add(egui::TextEdit::multiline(&mut hex::encode(&selected.raw)));

				// ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
				// 	if let Some(hand) = inspector.texture.as_ref() {
				// 		let texture: &egui::TextureHandle = hand;

				// 		let l = 200.; // occupied_screen_space.left - 10.;
				// 		ui.add(egui::Image::new(texture, egui::Vec2::new(l, l / 3.)));
				// 	}
				// });

					// if ui.button("📋").clicked() {
					// 	let url = "ws://127.0.0.1:9944"; //selected.doturl.url;
					// 	log!("button clicked {}", url);
					// 	async_std::task::block_on(async {
					// 		let pipe = polkapipe::PolkaPipe{
					// 			rpc: polkapipe::ws_web::Backend::new(&[url]).await.unwrap(),
					// 		};
					// 		log!("pipe created");
					// 		// Hello world 0 lifetime, 0 nonce, signed by alice
					// 		let v = hex::decode("d5018400d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d019264ccc4a8654543325ee6b3414f11c9563f04cc3aa3afb726fce3f25ff5db7bef7f229133d35b683cfbf66f94e2278512aa0eb6d8b9e3737e708262c34b0a8d0008000000072c48656c6c6f20776f726c64").unwrap();
					// 		log!("payload created");
					// 		let r = pipe.submit(v.as_slice()).await;
					// 		log!("result {:?}", r);
					// 	});
					// };
				} else {
					occupied_screen_space.left = 0.;
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

	occupied_screen_space.top = egui::TopBottomPanel::top("top_panel")
		.resizable(false)
		.show(egui_context, |ui| {
			// ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
			ui.horizontal(|ui| {
				ui.heading("Env:");
				let _combo = ComboBox::from_label("")
					.selected_text(format!("{}", spec.env))
					.show_ui(ui, |ui| {
						ui.selectable_value(&mut spec.env, Env::Local, "local");
						ui.selectable_value(&mut spec.env, Env::Prod, "dotsama");
						// ui.selectable_value(&mut spec.env, Env::SelfSovereign, "independents");
						// ui.selectable_value(&mut spec.env, Env::Test, "test");
					});

				// ui.add(
				// 	DatePicker::<std::ops::Range<NaiveDateTime>>::new(
				// 		"noweekendhighlight",
				// 		&mut spec.timestamp,
				// 	)
				// 	.highlight_weekend(false),
				// );

				//TODO: location = alpha blend to 10% everything but XXXX
				// let response = ui.text_edit_singleline(&mut spec.find);
				let found = 0;
				// if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
				// 	if spec.find.len() <= 4 {
				// 		// if let Ok(para_id) = spec.find.parse() {
				// 		// for (loc, details) in entities.iter() {
				// 		// 	if details.doturl.para_id == Some(para_id) &&
				// 		// 		details.doturl.block_number.is_some()
				// 		// 	{
				// 		// 		destination.location = Some(loc.translation());
				// 		// 		inspector.selected = Some(details.clone());
				// 		// 		found += 1;
				// 		// 	}
				// 		// }
				// 		// }
				// 	}
				// 	// for (loc, details) in entities.iter() {
				// 	// 	if spec.find.len() <= details.pallet.len() &&
				// 	// 		spec.find.as_bytes().eq_ignore_ascii_case(
				// 	// 			&details.pallet.as_bytes()[..spec.find.len()],
				// 	// 		)
				// 	// 	// if details.pallet.contains(&spec.find) ||
				// 	// 	// details.variant.contains(&spec.find)
				// 	// 	{
				// 	// 		destination.location = Some(loc.translation());
				// 	// 		inspector.selected = Some(details.clone());
				// 	// 		found += 1;
				// 	// 	}
				// 	// }
				//
				// 	println!("find {}", spec.find);
				// }
				if !spec.find.is_empty() {
					ui.heading(format!("found: {}", found));
				}
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					ui.checkbox(&mut anchor.deref_mut().follow_chain, "Follow");
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
		.show(egui_context, |ui| {
			ui.horizontal(|ui| {
				if let Some(selected) = &inspector.hovered {
					ui.heading(selected);
				}
				ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
					let x = viewpoint.x;
					let y = viewpoint.y;
					let z = viewpoint.z;

					let timestamp = super::x_to_timestamp(viewpoint.x);
					let datetime = DateTime::from_timestamp(timestamp, 0).unwrap();

					let newdate = datetime.format("%Y-%m-%d %H:%M:%S");

					let free = FREE_TXS.load(Ordering::Relaxed);
					ui.heading(format!(
						"TPS: {:03.0} EST FREE TPS: {:03.0} FPS: {:03.0} x={:03.0} y={:03.0} z={:03.0} {} ",
						tps, free, fps, x, y, z, newdate
					));
				});
			});
		})
		.response
		.rect
		.height();
}
use egui::Ui;

fn open_url(url: &str) {
	#[cfg(target_family = "wasm")]
	{
		let window = web_sys::window().unwrap();
		let agent = window.navigator().user_agent().unwrap();
		log!("agent {}", agent);
		if agent.contains("Safari") {
			if let Err(e) = window.location().assign(url) {
				log!("Error opening link {:?}", e);
			}
		} else {
			if let Err(e) = web_sys::window().unwrap().open_with_url(url) {
				log!("Error opening link {:?}", e);
			}
		}
	}
}

fn funk<'r>(ui: &'r mut Ui, val: &scale_borrow::Value) {
	match &val {
		scale_borrow::Value::Object(ref pairs) => {
			if pairs.len() == 1 {
				let mut header = String::new();
				let (mut k, v) = &pairs[0];
				let mut v: &scale_borrow::Value = v;

				while matches!(&v, scale_borrow::Value::Object(_)) {
					if let scale_borrow::Value::Object(nested_pairs) = &v {
						if nested_pairs.len() != 1 {
							break;
						}
						header.push_str(k);
						header.push('.');
						let (nk, nv) = &nested_pairs[0];
						k = nk;
						v = nv;
					}
				}
				header.push_str(k);
				// use egui::CollapsingHeader;
				ui.collapsing(header, |ui| {
					funk(ui, v);
				});
			} else {
				for (mut k, v) in pairs.iter() {
					if let scale_borrow::Value::Object(_nested_pairs) = &v {
						let mut header = String::new();
						let mut v: &scale_borrow::Value = v;

						while matches!(&v, scale_borrow::Value::Object(_)) {
							if let scale_borrow::Value::Object(nested_pairs) = &v {
								if nested_pairs.len() != 1 { break; }
								header.push_str(k);
								header.push('.');
								let (nk, nv) = &nested_pairs[0];
								k = nk;
								v = nv;
							}
						}
						header.push_str(k);
						// use egui::CollapsingHeader;
						ui.collapsing(header, |ui| {
							funk(ui, v);
						});
					} else {
						ui.collapsing(k, |ui| {
							funk(ui, v);
						});
					}
				}
			}
		},
		scale_borrow::Value::ScaleOwned(bytes) => {
			ui.label(format!("0x{}", hex::encode(bytes.as_slice())));
		},
		_ => {
			ui.label(val.to_string());
		},
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

	// pub fn timestamp(&self) -> Option<i64> {
	// 	//self.timestamp.map(|timestamp| {
	// 	let datetime: DateTime<chrono::Utc> = DateTime::from_utc(self.timestamp, Utc);
	// 	Some(datetime.timestamp() * 1000)
	// 	//})
	// }

	// pub fn changed(&self) -> bool {
	// 	self.was_location != self.location ||
	// 		self.was_timestamp != self.timestamp ||
	// 		self.was_env != self.env
	// }

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
