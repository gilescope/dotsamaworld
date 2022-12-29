use super::{as_rgba_u32, as_rgbemoji_u32, emoji_index, DataEntity};
use crate::{ui::details::Success, DataEvent};
// use bevy::render::color::Color;
use crate::log;
use palette::FromColor;
use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

#[derive(Clone)]
pub struct ExStyle {
	pub color: u32,
}

impl Hash for ExStyle {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(self.color).hash(state);
	}
}

impl Eq for ExStyle {}
impl PartialEq for ExStyle {
	fn eq(&self, other: &Self) -> bool {
		self.color == other.color
	}
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
	let mut s = DefaultHasher::new();
	t.hash(&mut s);
	s.finish()
}

// coloring block timestamp actually
pub fn color_block_number(block_number: i64, darkside: bool) -> u32 {
	let color = palette::Lchuv::new(
		if darkside { 40. } else { 80. },
		80. + (block_number % 100) as f32,
		(block_number % 360) as f32,
	);
	let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);
	as_rgba_u32(rgb.red, rgb.green, rgb.blue, 2.0) // alpha above 1 so not a texture.
}

pub fn style_event(entry: &DataEntity) -> ExStyle {
	let darkside = entry.details().doturl.is_darkside();
	// let msg = crate::content::is_message(entry);


	match entry {
		DataEntity::Event(data_event @ DataEvent { .. }) => style_data_event(data_event),
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
		DataEntity::Extrinsic { details, .. } => {
			let color = palette::Lchuv::new(
				if darkside { 40. } else { 80. },
				80. + (calculate_hash(&details.variant) as f32 % 100.),
				(calculate_hash(&details.pallet) as f32) % 360.,
			);
			let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);

			let emoji = emojidot::extrinsic_emoji(details.pallet.as_str(), details.variant.as_str());
			if let Some(some) = emoji {
				ExStyle { color: as_rgbemoji_u32(rgb.red, rgb.green, rgb.blue, emoji_index(some)) }
			} else {
				log!(
					"missing extrinsic {}, {}",
					details.pallet.as_str(),
					details.variant.as_str()
				);
				ExStyle { color: as_rgbemoji_u32(rgb.red, rgb.green, rgb.blue, 255) }
			}
		},
	}
}

pub fn style_data_event(entry: &DataEvent) -> ExStyle {
	let darkside = entry.details.doturl.is_darkside();
	let raw = &entry.details;

	let alpha =  emojidot::event_emoji(raw.pallet.as_str(), raw.variant.as_str());


	// let msg = crate::content::is_event_message(entry);
	if matches!(
		(raw.pallet.as_str(), raw.variant.as_str()),
		("System", "ExtrinsicFailed") /* | ("PolkadotXcm", "Attempted") - only an error if
		                               * !completed variant. */
	) || entry.details.success == Success::Sad
	{
		let alpha = if let Some(alpha) = alpha {
			emoji_index(alpha)
		} else {
			255
		};
		return ExStyle { color: as_rgbemoji_u32(1., 0., 0., alpha) }
	}
	if entry.details.success == Success::Worried {
		let alpha = if let Some(alpha) = alpha {
			emoji_index(alpha)
		} else {
			255
		};
		return ExStyle {
			// Trump Orange
			color: as_rgbemoji_u32(1., 0.647_058_84, 0., alpha),
		}
	}

	let color = palette::Lchuv::new(
		if darkside { 40. } else { 80. },
		80. + (calculate_hash(&raw.variant) as f32 % 100.),
		(calculate_hash(&raw.pallet) as f32) % 360.,
	);
	let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);

	// println!("rgb {} {} {}", rgb.red, rgb.green, rgb.blue);
	let alpha = if let Some(alpha) = alpha {
		emoji_index(alpha)
	} else {
		255
	};
	ExStyle { color: as_rgbemoji_u32(rgb.red, rgb.green, rgb.blue, alpha) }
}
