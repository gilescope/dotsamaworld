use crate::DataEntity;
use parity_scale_codec::Decode;
use subxt::{Event, Phase};

/// Is this extrinsic part of the overheads of running this blockchain?
/// For the relay chain including parachain blocks is useful work.
pub fn is_utility_extrinsic(event: &DataEntity) -> bool {
    match event {
        &DataEntity::Event { ref raw, .. } => {
            if !matches!(raw.phase, Phase::ApplyExtrinsic(_)) {
                return true;
            }

            // let pallet: &str = &raw.pallet;
            // let variant: &str = &raw.variant;
            // match (pallet, variant) {
            //     ("ImOnline", _)
            //     | ("EVM", "Log")
            //     | ("Staking", _)
            //     | ("DappsStaking", _)
            //     | ("PhalaMining", _)
            //     | ("RelayChainInfo", "CurrentBlockNumbers")
            //     | ("ParaInclusion", "CandidateIncluded")
            //     | ("ParaInclusion", "CandidateBacked")
            //     | ("ParachainStaking", _) => true,

            //     (
            //         crate::polkadot::system::events::ExtrinsicSuccess::PALLET,
            //         crate::polkadot::system::events::ExtrinsicSuccess::EVENT,
            //     ) => {
            //         if let Ok(decoded) = crate::polkadot::system::events::ExtrinsicSuccess::decode(
            //             &mut raw.data.to_vec().as_slice(),
            //         ) {
            //             if matches!(
            //                 decoded.dispatch_info.class,
            //                 super::polkadot::runtime_types::frame_support::weights::DispatchClass::Mandatory // String //runtime_types::frame_support::weights::DispatchInfo
            //             ) {
            //                 return true;
            //             }
            //         }
            //         return false;
            //     }
            //     _ => false,
            // }
        }
        _ => {}
    };
    false
}

pub fn is_message(entry: &DataEntity) -> bool {
    match entry {
        &DataEntity::Event { ref raw, .. } => {
            matches!(
                raw.pallet.as_str().to_ascii_lowercase().as_str(),
                "ump" | "dmpqueue" | "polkadotxcm"
            )
        }
        _ => false,
    }
}
