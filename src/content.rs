use parity_scale_codec::Decode;
use subxt::Event;
use subxt::{Phase, RawEventDetails};

use crate::DataEntity;

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
        | ("RelayChainInfo", "CurrentBlockNumbers")
        | ("ParaInclusion", "CandidateIncluded")
        | ("ParaInclusion", "CandidateBacked")
        | ("ParachainStaking", _) => true,

        (
            crate::polkadot::system::events::ExtrinsicSuccess::PALLET,
            crate::polkadot::system::events::ExtrinsicSuccess::EVENT,
        ) => {
            if let Ok(decoded) = crate::polkadot::system::events::ExtrinsicSuccess::decode(
                &mut event.data.to_vec().as_slice(),
            ) {
                if matches!(
                    decoded.dispatch_info.class,
                    super::polkadot::runtime_types::frame_support::weights::DispatchClass::Mandatory // String //runtime_types::frame_support::weights::DispatchInfo
                ) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

pub fn is_message(entry: &DataEntity) -> bool {
    match entry {
        DataEntity::Event { raw, .. } => {
            matches!(
                raw.pallet.as_str().to_ascii_lowercase().as_str(),
                "ump" | "dmpqueue" | "polkadotxcm"
            )
        },
        _ => {
false
        } 
    }
}