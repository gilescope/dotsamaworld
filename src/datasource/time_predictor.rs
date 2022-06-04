use std::convert::TryInto;

type TIME = u64;

/// Note: Genisis blocks (0) do not generally have timestamps.
pub fn get_block_number_near_timestamp(
    search_timestamp: TIME,
    start_block: u64,
    time_for_blocknum: &impl Fn(u64) -> Option<TIME>,
    average_blocktime_in_ms: Option<u64>,
) -> Option<u64> {
    get_block_number_near_timestamp_helper(
        search_timestamp as i64,
        start_block as i64,
        time_for_blocknum,
        average_blocktime_in_ms.map(|a| a as i64),
    ).map(|a| a as u64)
}

fn get_block_number_near_timestamp_helper(
    search_timestamp: i64,
    start_block: i64,
    time_for_blocknum: &impl Fn(u64) -> Option<TIME>,
    average_blocktime_in_ms: Option<i64>,
) -> Option<i64> {
    let average_blocktime_in_ms = average_blocktime_in_ms.unwrap_or(12_000);

    let start_time = time_for_blocknum(start_block.try_into().unwrap_or(1))? as i64;

    let time_distance = start_time - search_timestamp;
    let block_distance = time_distance / average_blocktime_in_ms;

    let guess = start_block - block_distance;

    let guess_time = time_for_blocknum(guess.try_into().unwrap_or(1))? as i64;

    let actual_blocktime = (start_time - guess_time) / (start_block - guess);
    let calibrated_block_distance = time_distance / actual_blocktime;
    let calibrated_guess = start_block - calibrated_block_distance;
    let calibrated_guess = calibrated_guess.try_into().unwrap_or(1);
    let calibrated_guess_time = time_for_blocknum( calibrated_guess )? as i64;
    if (calibrated_guess_time.abs_diff(search_timestamp) as i64) < actual_blocktime * 2 {
        return Some(calibrated_guess as i64);
    }
    get_block_number_near_timestamp_helper(
        search_timestamp,
        calibrated_guess as i64,
        time_for_blocknum,
        Some(actual_blocktime),
    )
}

#[cfg(test)]
mod tests {
    use super::get_block_number_near_timestamp;
    use super::TIME;

    #[test]
    fn real_polkadot_example() {
        fn time_for_blocknum(blocknum: u64) -> Option<TIME> {
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
                _ => panic!("oh dear {}", blocknum)
            })
        }
        // Track backwards in time:
        assert_eq!(
            Some(10000000),
            get_block_number_near_timestamp(1_650_715_386_009, 10500000, &time_for_blocknum, None)
        );

        // // Track forwards in time:
        assert_eq!(
            Some(10500000),
            get_block_number_near_timestamp(1_653_739_872_004, 10000000, &time_for_blocknum, None)
        );

        // Track backwards to genesis:
        assert_eq!(
            Some(1),
            get_block_number_near_timestamp(1_590_507_378_000, 10500000, &time_for_blocknum, None)
        );

        // // Track forwards te restaunt at the end of the universe:
        assert_eq!(
            None,
            get_block_number_near_timestamp(1_653_739_872_004_000_000, 10000000, &time_for_blocknum, None)
        );
    }
}