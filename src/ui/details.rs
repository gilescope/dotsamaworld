//! This mod contains all the egui code for 2d that powers the details screen.
use super::DotUrl;
use bevy::{ecs as bevy_ecs, prelude::*};
use bevy_ecs::prelude::Component;
use bevy_egui::EguiSettings;
use serde::{Deserialize, Serialize};
// use bevy_inspector_egui::{
// 	options::{NumberAttributes, StringAttributes},
// 	Context, Inspectable,
// };

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Success {
	#[default]
	Happy,
	Worried,
	Sad,
}

#[derive(Component, Default, Clone, Debug, Serialize, Deserialize)]
pub struct Details//<'scale> - would require details being stored somewhere with ids.
{
	pub pallet: String,
	pub doturl: DotUrl,
	pub parent: Option<u32>,
	pub variant: String,
	pub success: Success,
	// pub hover: String,
	pub flattern: String,
	// #[inspectable(label = "Url:")]
	pub url: String,
	// pub chain_name: String,
	pub raw: Vec<u8>,
	// pub value: Option<scale_value::Value>
}

// use egui::Grid;
// impl Inspectable for Details {
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
// 				changed |=
// 					self.doturl.to_string().ui(ui, StringAttributes { multiline: false }, context);
// 				ui.end_row();
// 				changed |= self.pallet.ui(ui, StringAttributes { multiline: false }, context);
// 				ui.end_row();
// 				// ui.label("Method");
// 				changed |= self.variant.ui(ui, StringAttributes { multiline: false }, context);
// 				ui.end_row();
// 				changed |=
// 					self.parent.unwrap_or_default().ui(ui, NumberAttributes::default(), context);
// 				ui.end_row();
// 				changed |= self.hover.ui(ui, StringAttributes { multiline: true }, context);
// 				ui.end_row();
// 				changed |= self.flattern.ui(ui, StringAttributes { multiline: true }, context);
// 				ui.end_row();
// 				// ui.label("Rotation");
// 				// changed |= self.rotation.ui(ui, Default::default(), context);
// 				// self.rotation = self.rotation.normalize();
// 				// ui.end_row();

// 				// ui.label("Scale");
// 				// let scale_attributes = NumberAttributes {
// 				//     min: Some(Vec3::splat(0.0)),
// 				//     ..Default::default()
// 				// };
// 				// changed |= self.scale.ui(ui, scale_attributes, context);
// 				// ui.end_row();
// 			});
// 		});
// 		changed
// 	}
// }

pub fn configure_visuals(
	// egui_ctx: ResMut<EguiContext>,
	mut egui_settings: ResMut<EguiSettings>, //  ,windows: Res<Windows>
) {
	// egui_ctx.ctx_mut().set_visuals(egui::Visuals {
	//     window_rounding: 0.0.into(),
	//     // scale:1.2,
	//     ..default()
	// });
	// egui_ctx.ctx_mut().set_style(egui::Style {
	//     spacing: Spacing {
	//         text_edit_width: 1000000.,
	//         ..default()
	//     },
	//     ..default()
	// });
	// egui_ctx.ctx_mut().set_debug_on_hover(true);
	// egui_ctx.ctx_mut().set_widgets(
	//     egui::Widgets {
	//         ..default()
	//     }
	// );
	//  .desired_width(f32::INFINITY)
	// if let Some(window) = windows.get_primary().is_some()
	{
		egui_settings.scale_factor = 1.5;
	}
}
