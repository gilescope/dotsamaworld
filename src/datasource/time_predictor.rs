use crate::datasource::{find_timestamp, get_block_hash, Source};
use async_recursion::async_recursion;
use std::convert::TryInto;

type TIME = i64;

/// Note: Genisis blocks (0) do not generally have timestamps.
pub async fn get_block_number_near_timestamp(
	search_timestamp: TIME,
	start_block: u32,
	source: &mut impl Source,
	//&mut impl FnMut(u32) -> Option<TIME>,
	average_blocktime_in_ms: Option<u64>,
	metad_current: &frame_metadata::RuntimeMetadataPrefixed,
) -> Option<u32> {
	debug_assert!(search_timestamp > 9_654_602_493, "you were meant to multiply that by 1000");
	get_block_number_near_timestamp_helper(
		search_timestamp as i64,
		start_block as i64,
		source,
		average_blocktime_in_ms.map(|a| a as i64),
		metad_current,
	)
	.await
	.map(|a| a as u32)
}

#[async_recursion(?Send)]
async fn get_block_number_near_timestamp_helper<S: Source>(
	search_timestamp: i64,
	start_block: i64,
	source: &mut S,
	average_blocktime_in_ms: Option<i64>,
	metad_current: &frame_metadata::RuntimeMetadataPrefixed,
) -> Option<i64> {
	let average_blocktime_in_ms = average_blocktime_in_ms.unwrap_or(12_000);

	let start_time = {
		let start_blocknum = start_block.try_into().unwrap_or(1);
		let mut block_hash: Option<primitive_types::H256> =
			get_block_hash(source, start_blocknum).await;
		for _ in 0..10 {
			if block_hash.is_some() {
				break
			}
			block_hash = get_block_hash(source, start_blocknum).await;
		}

		find_timestamp(
			// chain_info.chain_url.clone(),
			block_hash.unwrap(),
			source,
			metad_current,
		)
		.await
	}
	.unwrap() as i64;

	let time_distance = start_time - search_timestamp;
	let block_distance = time_distance / average_blocktime_in_ms;

	let guess = start_block - block_distance;

	let guess_time = {
		let start_blocknum = guess.try_into().unwrap_or(1);
		let mut block_hash: Option<primitive_types::H256> =
			get_block_hash(source, start_blocknum).await;
		for _ in 0..10 {
			if block_hash.is_some() {
				break
			}
			block_hash = get_block_hash(source, start_blocknum).await;
		}

		find_timestamp(
			// chain_info.chain_url.clone(),
			block_hash.unwrap(),
			source,
			metad_current,
		)
		.await
	}
	.unwrap() as i64;

	let actual_blocktime = (start_time - guess_time) / (start_block - guess);
	if actual_blocktime == 0 {
		return None
	} // Suspicious.
	let calibrated_block_distance = time_distance / actual_blocktime;
	let calibrated_guess = start_block - calibrated_block_distance;
	let calibrated_guess = calibrated_guess.try_into().unwrap_or(1);

	let calibrated_guess_time = {
		let start_blocknum = calibrated_guess;
		let mut block_hash: Option<primitive_types::H256> =
			get_block_hash(source, start_blocknum).await;
		for _ in 0..10 {
			if block_hash.is_some() {
				break
			}
			block_hash = get_block_hash(source, start_blocknum).await;
		}

		find_timestamp(
			// chain_info.chain_url.clone(),
			block_hash.unwrap(),
			source,
			metad_current,
		)
		.await
	}
	.unwrap() as i64;

	if (calibrated_guess_time.abs_diff(search_timestamp) as i64) < actual_blocktime * 2 {
		return Some(calibrated_guess as i64)
	}
	get_block_number_near_timestamp_helper(
		search_timestamp,
		calibrated_guess as i64,
		source,
		Some(actual_blocktime),
		metad_current,
	)
	.await
}

#[cfg(test)]
mod tests {
	use crate::datasource::RawDataSource;

	use super::{super::get_metadata, get_block_number_near_timestamp, TIME};

	#[test]
	fn real_polkadot_example_test() {
		async_std::task::block_on(real_polkadot_example());
	}

	async fn real_polkadot_example() {
		let mut source = RawDataSource::new("wss://rpc.polkadot.io:443");
		let _ = color_eyre::install();
		let metad_current = get_metadata(&mut source, None).await.unwrap();

		fn time_for_blocknum(blocknum: u32) -> Option<TIME> {
			Some(match blocknum {
				10000000 => 1_650_715_386_009,
				10000023 => 1_650_715_524_004,
				10001897 => 1_650_726_768_002,
				10247960 => 1_652_209_404_007,
				10500000 => 1_653_739_872_004,

				// Needed for reverse:
				10252040 => 1_652_234_052_006,
				10501989 => 1_653_751_962_006,
				10499983 => 1_653_739_770_009,

				// Back to genesis:
				5230626 => 1_622_037_510_000,
				1 => 1_590_507_378_000,

				// Anything above here is future
				12000000.. => return None,
				_ => panic!("oh dear {}", blocknum),
			})
		}
		// Track backwards in time:
		assert_eq!(
			Some(10000000),
			get_block_number_near_timestamp(
				1_650_715_386_009,
				10500000,
				&mut source,
				None,
				&metad_current
			)
			.await
		);

		// // Track forwards in time:
		assert_eq!(
			Some(10500000),
			get_block_number_near_timestamp(
				1_653_739_872_004,
				10000000,
				&mut source,
				None,
				&metad_current
			)
			.await
		);

		// Track backwards to genesis:
		assert_eq!(
			Some(1),
			get_block_number_near_timestamp(
				1_590_507_378_000,
				10500000,
				&mut source,
				None,
				&metad_current
			)
			.await
		);

		// Track forwards to the restaurant at the end of the universe:
		assert_eq!(
			None,
			get_block_number_near_timestamp(
				1_653_739_872_004_000_000,
				10000000,
				&mut source,
				None,
				&metad_current
			)
			.await
		);
	}
}
