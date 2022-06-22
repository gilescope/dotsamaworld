pub mod details;
pub mod doturl;
// pub mod toggle;
use crate::Viewport;
use bevy::prelude::*;
use bevy_egui::EguiContext;
use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Utc;
pub use details::Details;
pub use doturl::DotUrl;

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
    mut viewpoint_query: Query<&GlobalTransform, With<Viewport>>,
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
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height();
    occupied_screen_space.bottom = egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            let timestamp =
                super::x_to_timestamp(viewpoint_query.get_single().unwrap().translation.x);
            let naive = NaiveDateTime::from_timestamp(timestamp as i64, 0);
            let datetime: DateTime<chrono::Utc> = DateTime::from_utc(naive, Utc);
            let datetime: DateTime<chrono::Local> = datetime.into();
            let newdate = datetime.format("%Y-%m-%d %H:%M:%S");
            ui.heading(format!("{}", newdate));
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

//     let distance_to_target = (/*CAMERA_TARGET -*/ original_camera_transform.0.translation).length();
//     let frustum_height = 2.0 * distance_to_target * (camera_projection.fov * 0.5).tan();
//     let frustum_width = frustum_height * camera_projection.aspect_ratio;

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
