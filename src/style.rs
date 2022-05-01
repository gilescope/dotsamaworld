use super::DataEntity;
use bevy::render::color::Color;

#[derive(Clone)]
pub struct ExStyle {
    pub color: Color,
}

impl Hash for ExStyle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.color.r() as u32).hash(state);
        (self.color.g() as u32).hash(state);
        (self.color.b() as u32).hash(state);
    }
}

impl Eq for ExStyle {}
impl PartialEq for ExStyle {
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color
    }
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

use palette::FromColor;

pub fn style_event(entry: &DataEntity) -> ExStyle {
    let msg = crate::content::is_message(entry);
    match entry {
        DataEntity::Event { raw, .. } => {
            if raw.pallet.as_str() == "System" && raw.variant.as_str() == "ExtrinsicFailed" {
                return ExStyle {
                    color: Color::rgb(1., 0., 0.),
                };
            }

            let color = palette::Lchuv::new(
                80.,
                80. + (calculate_hash(&raw.variant) as f32 % 100.),
                (calculate_hash(&raw.pallet) as f32) % 360.,
            );
            let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);

            // println!("rgb {} {} {}", rgb.red, rgb.green, rgb.blue);

            ExStyle {
                color: Color::rgb(rgb.red, rgb.green, rgb.blue),
            }
        }
        // match event.pallet.as_str() {
        //     "Staking" => ExStyle {
        //         color: Color::hex("00ffff").unwrap(),
        //     },
        //     "Deposit" => ExStyle {
        //         color: Color::hex("e6007a").unwrap(),
        //     },
        //     "Withdraw" => ExStyle {
        //         color: Color::hex("e6007a").unwrap(),
        //     },
        //     _ => ExStyle {
        //         color: Color::hex("000000").unwrap(),
        //     },
        // }
        DataEntity::Extrinsic {
            pallet, variant, ..
        } => {
            let color = palette::Lchuv::new(
                80.,
                80. + (calculate_hash(&variant) as f32 % 100.),
                (calculate_hash(&pallet) as f32) % 360.,
            );
            let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);

            ExStyle {
                color: Color::rgba(rgb.red, rgb.green, rgb.blue, if msg { 0.5 } else { 1. }),
            }
        }
    }
}
