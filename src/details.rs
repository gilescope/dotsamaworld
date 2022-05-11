//! This mod contains all the egui code for 2d that powers the details screen.
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy_ecs::prelude::Component;
// use bevy_egui::EguiContext;
use bevy_inspector_egui::{Inspectable};
// use bevy_inspector_egui_derive::*;
// use bevy_inspector_egui::{Inspectable, InspectorPlugin};
// use bevy_inspector_egui::prelude::*;
use bevy_egui::EguiSettings;
use egui::TextBuffer;
use bevy_inspector_egui::options::StringAttributes;
// use egui::style::Spacing;
#[derive(Clone, Debug, Default)]
pub struct WideStringAttributes {
    pub multiline: bool,
}
use bevy_inspector_egui::Context;
// use futures::future::WeakShared;

#[derive(Component, Default, Clone)]
pub struct Details {
    // #[inspectable(label = "Hover", multiline = true)]
    pub hover: WideString,
    pub flattern: String,
    // pub flattened: String,
    // data: DataEntity,
    // #[inspectable(label = "Url:")]
    pub url: String,
}
use egui::Grid;
impl Inspectable for Details {
    type Attributes = ();

    fn ui(
        &mut self,
        ui: &mut bevy_egui::egui::Ui,
        _options: Self::Attributes,
        context: &mut Context,
    ) -> bool {
        let mut changed = false;
        ui.vertical_centered(|ui| {
            Grid::new(context.id())
            .min_col_width(500.)
            .show(ui, |ui| {
                // ui.label("Details");
                changed |= self.hover.ui(ui, WideStringAttributes{multiline:true}, context);
                ui.end_row();
                changed |= self.flattern.ui(ui, StringAttributes{multiline:true}, context);
                ui.end_row();
                // ui.label("Rotation");
                // changed |= self.rotation.ui(ui, Default::default(), context);
                // self.rotation = self.rotation.normalize();
                // ui.end_row();

                // ui.label("Scale");
                // let scale_attributes = NumberAttributes {
                //     min: Some(Vec3::splat(0.0)),
                //     ..Default::default()
                // };
                // changed |= self.scale.ui(ui, scale_attributes, context);
                // ui.end_row();
            });
        });
        changed
    }
}


#[derive(Clone, Default)]
pub struct WideString(pub String);

impl WideString {
   pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for WideString {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl TextBuffer for WideString {
    fn is_mutable(&self) -> bool { false }
    fn insert_text(&mut self, _: &str, _: usize) -> usize { 0 }
    fn delete_char_range(&mut self, _: std::ops::Range<usize>) { }
}

impl Inspectable for WideString {
    type Attributes = WideStringAttributes;

    fn ui(&mut self, ui: &mut egui::Ui, options: Self::Attributes, _: &mut Context) -> bool {
        let widget = match options.multiline {
            false => egui::widgets::TextEdit::singleline(self),
            true => egui::widgets::TextEdit::multiline(self),
        };
        let widget = widget.desired_width(f32::INFINITY);
        // egui::widgets::TextEdit::desired_width(widget, 1000.);
        // PERF: this is changed if text if highlighted
        ui.add(widget).changed()
    }
}

pub fn configure_visuals(
    // egui_ctx: ResMut<EguiContext>,   
     mut egui_settings: ResMut<EguiSettings>
    //  ,windows: Res<Windows>
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