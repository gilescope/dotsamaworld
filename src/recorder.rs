use crate::{Anchor, Destination, Viewport};
use bevy::prelude::{GlobalTransform, Query, *};

#[derive(Default)]
pub struct Script {
	pub moments: Vec<(f64, Vec3, Quat)>,
}

impl Script {
	fn parse(contents: &str) -> Result<Script, ()> {
		let mut result = Script::default();
		for line in contents.lines() {
			let parts: Vec<_> = line.split(',').collect();
			result.moments.push((
				parts[0].parse().unwrap(),
				Vec3::new(
					parts[1].parse().unwrap(),
					parts[2].parse().unwrap(),
					parts[3].parse().unwrap(),
				),
				Quat::from_xyzw(
					parts[4].parse().unwrap(),
					parts[5].parse().unwrap(),
					parts[6].parse().unwrap(),
					parts[7].parse().unwrap(),
				),
			));
		}
		result.moments = result.moments.into_iter().rev().collect();
		Ok(result)
	}
}

/// Same as [`PlayerPlugin`] but does not spawn a camera
pub struct RecorderPlugin;
impl Plugin for RecorderPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(Script::default())
			.add_startup_system(start_recording)
			.add_startup_system(start_playing)
			.add_system(camera_recorder)
			.add_system(player);
	}
}

pub fn start_recording() {
	// Clean recording each time.
	let _ = std::fs::remove_file("record.csv");
}

pub fn camera_recorder(time: Res<Time>, query_t: Query<&GlobalTransform, With<Viewport>>) {
	let query = query_t.single();
	use std::io::Write;
	//TODO stick in resource and buffer.
	let mut file = std::fs::OpenOptions::new()
		.write(true)
		.create(true)
		.append(true)
		.open("record.csv")
		.unwrap();

	let _ = write!(
		file,
		"{},{},{},{},{},{},{},{}\r\n",
		time.seconds_since_startup(),
		query.translation.x,
		query.translation.y,
		query.translation.z,
		query.rotation.x,
		query.rotation.y,
		query.rotation.z,
		query.rotation.w
	);
}

pub fn start_playing(mut script: ResMut<Script>, mut anchor: ResMut<Anchor>) {
	if let Ok(contents) = std::fs::read_to_string("play.csv") {
		if let Ok(new_script) = Script::parse(&contents) {
			script.moments = new_script.moments;
			println!("Parsed script ok, playing {} moments", script.moments.len());
			anchor.follow_chain = false;
		}
	}
}

pub fn player(time: Res<Time>, mut script: ResMut<Script>, mut dest: ResMut<Destination>) {
	if let Some(top) = script.moments.last() {
		if time.seconds_since_startup() > top.0 {
			dest.location = Some(top.1);
			dest.look_at = Some(top.2);
			script.moments.pop();
			// println!("playing event");
		}
	}
}
