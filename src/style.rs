use bevy::render::color::Color;
use subxt::RawEventDetails;

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

pub fn style_event(event: &RawEventDetails) -> ExStyle {
    if event.pallet.as_str() == "System" && event.variant.as_str() == "ExtrinsicFailed" {
        return ExStyle {
            color: Color::rgb(1., 0., 0.),
        };
    }

    let color = palette::Lchuv::new(
        80.,
        80. + (calculate_hash(&event.variant) as f32 % 100.),
        (calculate_hash(&event.pallet) as f32) % 360.,
    );
    let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);

    // println!("rgb {} {} {}", rgb.red, rgb.green, rgb.blue);

    ExStyle {
        color: Color::rgb(rgb.red, rgb.green, rgb.blue),
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
}
