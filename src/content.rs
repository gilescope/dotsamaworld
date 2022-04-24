use subxt::{Phase, RawEventDetails};

/// Is this extrinsic part of the overheads of running this blockchain?
/// For the relay chain including parachain blocks is useful work.
pub fn is_utility_extrinsic(event: &RawEventDetails) -> bool {
    if !matches!(event.phase, Phase::ApplyExtrinsic(_)) {
        return true;
    }

    let pallet: &str = &event.pallet;
    let variant: &str = &event.variant;
    match (pallet, variant) {
        ("ImOnline", _)
        | ("EVM", "Log")
        | ("Staking", _)
        | ("DappsStaking", _)
        | ("PhalaMining", _)
        | ("ParachainStaking", _) => true,

        _ => false,
    }
}
