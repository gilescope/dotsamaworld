use crate::Env;
use std::num::NonZeroU32;

#[derive(Default, Debug, Clone)]
pub struct DotUrl {
	pub env: Env,
	// 0 is the no mans land in the middle
	pub sovereign: Option<i32>,
	pub para_id: Option<NonZeroU32>,
	pub block_number: Option<u32>,
	pub extrinsic: Option<u32>,
	pub event: Option<u32>,
}

impl DotUrl {
	pub fn parse(url: &str) -> Result<Self, ()> {
		let (protocol, rest) = url.split_once(':').ok_or(())?;
		let mut result = DotUrl::default();
		result.env = match protocol {
			"indies" => Env::SelfSovereign,
			"testindies" => Env::SelfSovereignTest,
			"test" => Env::Test,
			"nfts" => Env::NFTs,
			"local" => Env::Local,
			"cgp" => Env::CGP,
			"dotsama" | _ => Env::Prod,
		};

		let mut parts = rest.split('/');

		parts.next(); // There should be nothing before the first slash as that would be something relative.
		if let Some(sovereign) = parts.next() {
			result.sovereign = sovereign.parse().ok();
			if let Some(para_id) = parts.next() {
				result.para_id = para_id.parse().ok();
				if let Some(block_number) = parts.next() {
					result.block_number = block_number.parse().ok();
					if let Some(extrinsic) = parts.next() {
						result.extrinsic = extrinsic.parse().ok();
						if let Some(event) = parts.next() {
							result.event = event.parse().ok();
						}
					}
				}
			}
		}
		Ok(result)
	}

	// Is cyberpunkusama?
	pub fn is_darkside(&self) -> bool {
		self.sovereign.unwrap_or(1) == -1
	}

	pub fn rflip(&self) -> f32 {
		if self.is_darkside() {
			1.0
		} else {
			-1.0
		}
	}

	pub fn is_relay(&self) -> bool {
		self.para_id.is_none()
	}

	/// Are we layer zero (relay chain), layer one or...
	pub fn layer(&self) -> usize {
		if self.is_relay() {
			0
		} else {
			1
		}
	}
}

impl std::fmt::Display for DotUrl {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let protocol = match self.env {
			Env::SelfSovereign => "indies",
			Env::Prod => "dotsama",
			Env::SelfSovereignTest => "testindies",
			Env::Test => "test",
			Env::NFTs => "nfts",
			Env::Local => "local",
			Env::CGP => "cgp",
		};

		f.write_fmt(format_args!(
			"{}:",
			protocol,
			// self.sovereign.map(|s| s.to_string()).unwrap_or("".to_string()),
			// self.para_id.map(|s| s.to_string()).unwrap_or("".to_string()),
			// self.block_number.map(|s| s.to_string()).unwrap_or("".to_string()),
			// self.extrinsic.map(|s| s.to_string()).unwrap_or("".to_string()),
		))?;
		if let Some(relay_id) = self.sovereign {
			f.write_fmt(format_args!("/{}", if relay_id == 1 { "polkadot" } else { "kusama" }))?;
		}
		if let Some(para_id) = self.para_id {
			f.write_fmt(format_args!("/{}", para_id))?;
		}
		if let Some(block_number) = self.block_number {
			f.write_fmt(format_args!("/{}", block_number))?;
		}
		if let Some(extrinsic) = self.extrinsic {
			f.write_fmt(format_args!("/{}", extrinsic))?;
		}
		if let Some(event) = self.event {
			f.write_fmt(format_args!("/{}", event))?;
		}
		Ok(())
	}
}
