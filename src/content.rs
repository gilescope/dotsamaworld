use subxt::RawEventDetails;

/// Is this extrinsic part of the overheads of running this blockchain?
/// For the relay chain including parachain blocks is useful work.
pub fn is_utiliy_extrinsic(event: &RawEventDetails) -> bool {
    let pallet: &str = &event.pallet;
    let variant: &str = &event.variant;
    match (pallet, variant) {
        ("ImOnline", _) | ("EVM", "Log") => true,

        _ => false,
    }
}
