use crate::{DataEntity, DataEvent};

/// Is this extrinsic part of the overheads of running this blockchain?
/// For the relay chain including parachain blocks is useful work.
pub fn is_utility_extrinsic(event: &DataEntity) -> bool {
	match event {
		&DataEntity::Extrinsic { ref details, .. } =>
			return is_boring(details.pallet.as_str(), details.variant.as_str()),
		&DataEntity::Event(DataEvent { ref details, .. }) =>
			details.parent.is_none() || is_boring(&details.pallet, &details.variant),
	}
}

fn is_boring(pallet: &str, variant: &str) -> bool {
	match (pallet, variant) {
        ("ImOnline", _)
        | ("EVM", "Log")
        | ("Staking", _)
        | ("Authorship", "set_uncles")
        | ("CollatorStaking","set_block_producer")
        | ("DappsStaking", _)
        | ("PhalaMining", _)
        | ("RelayChainInfo", "CurrentBlockNumbers")
        | ("ParaInclusion", "CandidateIncluded")
        | ("ParaInclusion", "CandidateBacked")
        // | ("ParaInherent", "enter") - this is what the relay chains most important job is.
        | ("Timestamp", "set")
        // | ("ParachainSystem", "set_validation_data") - not boring as executes other user's messages
        | ("AuthorInherent","kick_off_authorship_validation")//zeitgeist moonbeam
        | ("Lighthouse", "set")
        | ("ParachainStaking", _) => true,
        _ => false,
    }
}

pub fn is_event_message(entry: &DataEvent) -> bool {
	match entry {
		&DataEvent { ref details, .. } => {
			matches!(
				details.pallet.as_str().to_ascii_lowercase().as_str(),
				"ump" | "dmpqueue" | "polkadotxcm" | "xcmpallet"
			)
		},
	}
}

pub fn is_message(entry: &DataEntity) -> bool {
	match entry {
		&DataEntity::Event(ref event) => is_event_message(event),
		_ => false,
	}
}
