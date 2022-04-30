use crate::DataEntity;
use parity_scale_codec::Decode;
use subxt::{Event, Phase};

/// Is this extrinsic part of the overheads of running this blockchain?
/// For the relay chain including parachain blocks is useful work.
pub fn is_utility_extrinsic(event: &DataEntity) -> bool {
    match event {
        &DataEntity::Extrinsic {
            ref pallet,
            ref variant,
            ..
        } => {
            let pallet: &str = pallet.as_str();
            let variant: &str = variant.as_str();
            return is_boring(pallet, variant);
        }
        &DataEntity::Event { ref raw, .. } => {
            !matches!(raw.phase, Phase::ApplyExtrinsic(_)) || is_boring(&raw.pallet, &raw.variant)
        }
        _ => false,
    }
}

fn is_boring(pallet: &str, variant: &str) -> bool {
    match (pallet, variant) {
        ("ImOnline", _)
        | ("EVM", "Log")
        | ("Staking", _)
        | ("DappsStaking", _)
        | ("PhalaMining", _)
        | ("RelayChainInfo", "CurrentBlockNumbers")
        | ("ParaInclusion", "CandidateIncluded")
        | ("ParaInclusion", "CandidateBacked")
        | ("ParaInherent", "enter")
        | ("Timestamp", "set")
        | ("ParachainSystem", "set_validation_data")
        | ("ParachainStaking", _) => true,
        _ => false,
    }
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
