use super::{as_rgba_u32, as_rgbemoji_u32, emoji_index, DataEntity};
use crate::{ui::details::Success, DataEvent};
// use bevy::render::color::Color;
use crate::log;
use palette::FromColor;
use std::{
	collections::hash_map::DefaultHasher,
	hash::{Hash, Hasher},
};

#[derive(Clone)]
pub struct ExStyle {
	pub color: u32,
}

impl Hash for ExStyle {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(self.color).hash(state);
	}
}

impl Eq for ExStyle {}
impl PartialEq for ExStyle {
	fn eq(&self, other: &Self) -> bool {
		self.color == other.color
	}
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
	let mut s = DefaultHasher::new();
	t.hash(&mut s);
	s.finish()
}

// coloring block timestamp actually
pub fn color_block_number(block_number: i64, darkside: bool) -> u32 {
	let color = palette::Lchuv::new(
		if darkside { 40. } else { 80. },
		80. + (block_number % 100) as f32,
		(block_number % 360) as f32,
	);
	let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);
	as_rgba_u32(rgb.red, rgb.green, rgb.blue, 0.7)
}

pub fn style_event(entry: &DataEntity) -> ExStyle {
	let darkside = entry.details().doturl.is_darkside();
	let msg = crate::content::is_message(entry);
	match entry {
		DataEntity::Event(data_event @ DataEvent { .. }) => style_data_event(data_event),
		// match event.pallet.as_str() {
		//     "Staking" => ExStyle {
		//         color: Color::hex("00ffff").unwrap(),
		//     },
		//     "Deposit" => ExStyle {
		//         color: Color::hex("e6007a").unwrap(),
		//     },
		//     "Withdraw" => ExStyle {
		//         color: Color::hex("e6007a").unwrap(),
		//     },
		//     _ => ExStyle {
		//         color: Color::hex("000000").unwrap(),
		//     },
		// }
		DataEntity::Extrinsic { details, .. } => {
			let color = palette::Lchuv::new(
				if darkside { 40. } else { 80. },
				80. + (calculate_hash(&details.variant) as f32 % 100.),
				(calculate_hash(&details.pallet) as f32) % 360.,
			);
			let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);
			let emoji = match (details.pallet.as_str(), details.variant.as_str()) {
				("AcalaOracle", "feed_values") => emoji_index("crystal_ball"),
				("AuthorInherent", "kick_off_authorship_validation") => emoji_index("detective"),
				("AMM", "remove_liquidity") => emoji_index("chart_decreasing"),
				("AMMRoute", "swap_exact_tokens_for_tokens") => emoji_index("currency_exchange"),
				("AMMRoute", "swap_tokens_for_exact_tokens") => emoji_index("currency_exchange"),
				("Balances", "transfer") => emoji_index("shuffle_tracks"),
				("Balances", "transfer_keep_alive") => emoji_index("shuffle_tracks"),
				("Balances", "transfer_all") => emoji_index("shuffle_tracks"),
				("ConvictionVoting", "vote") => emoji_index("crystal_ball"),
				("Datalog", "record") => emoji_index("black_nib"),
				("DappsStaking", "bond_and_stake") => emoji_index("locked"),
				("DappsStaking", "set_reward_destination") => emoji_index("wrench"),
				("DappsStaking", "claim_staker") => emoji_index("crystal_ball"),
				("Democracy", "vote") => emoji_index("check_box_with_check"),
				("Dex", "swap_with_exact_supply") => emoji_index("currency_exchange"),
				("Ethereum", "transact") => emoji_index("gear"),
				("EVM", "withdraw") => emoji_index("money_bag"),
				("FuelTanks", "dispatch") => emoji_index("crystal_ball"),
				("FuelTanks", "add_account") => emoji_index("identification_card"),
				("Farming", "withdraw") => emoji_index("farmer"),
				("Farming", "withdraw_claim") => emoji_index("farmer"),
				("Farming", "deposit") => emoji_index("farmer"),
				("ImOnline", "heartbeat") => emoji_index("beating_heart"),
				("Incentives", "claim_rewards") => emoji_index("face_savoring_food"),
				("Lighthouse", "set") => emoji_index("crystal_ball"),
				("LiquidStaking", "set_staking_ledger") => emoji_index("black_nib"),
				("Loans", "mint") => emoji_index("heavy_dollar"),
				("Loans", "claim_reward") => emoji_index("face_savoring_food"),
				("Loans", "redeem") => emoji_index("money_bag"),
				("MultiTokens", "batch_transfer") => emoji_index("shuffle_tracks"),
				("Multisig", "as_multi") => emoji_index("busts"),
				("NominationPools", "bond_extra") => emoji_index("locked"),
				("NominationPools", "claim_payout") => emoji_index("unlocked"),
				("Nft", "transfer") => emoji_index("woman_artist"),
				("NominationPools", "join") => emoji_index("crystal_ball"),
				("Oracle", "set_price_unsigned") => emoji_index("crystal_ball"),
				("Oracle", "feed_values") => emoji_index("crystal_ball"),
				("PolkadotXcm", "limited_reserve_transfer_assets") => emoji_index("shuffle_tracks"),
				("ParaInherent", "enter") => emoji_index("gear"),
				("ParachainStaking", "delegator_bond_more") => emoji_index("locked"),
				("ParachainSystem", "set_validation_data") => emoji_index("gear"),
				("PhalaMq", "sync_offchain_message") => emoji_index("incoming_envelope"),
				("PhalaRegistry", "register_worker") => emoji_index("gear"),
				("Proxy", "proxy") => emoji_index("ghost"),
				("Randomness", "set_babe_randomness_results") => emoji_index("game_die"),
				("RWS", "call") => emoji_index("gear"),
				("Staking", "nominate") => emoji_index("crystal_ball"),
				("Staking", "bond") => emoji_index("locked"),
				("Staking", "bond_extra") => emoji_index("crystal_ball"),
				("Staking", "withdraw_unbonded") => emoji_index("money_bag"),
				("Session", "set_keys") => emoji_index("old_key"),
				("Staking", "unbond") => emoji_index("unlocked"),
				("Staking", "set_payee") => emoji_index("bust"),
				("Staking", "payout_stakers") => emoji_index("gem_stone"),
				("Staking", "chill") => emoji_index("snowflake"),
				("System", "remark") => emoji_index("left_speach_bubble"),
				("Timestamp", "set") => emoji_index("nine_oclock"),
				("TransactionPayment", "TransactionFeePaid") => emoji_index("palm_up_hand"),
				("Utility", "batch_all") => emoji_index("crystal_ball"),
				("Utility", "batch") => emoji_index("crystal_ball"),
				("Utility", "force_batch") => emoji_index("crystal_ball"),
				("Unique", "set_token_properties") => emoji_index("black_nib"),
				("Unique", "create_item") => emoji_index("woman_artist"),
				("Vesting", "claim") => emoji_index("carrot"),
				("Vesting", "vest") => emoji_index("carrot"),
				("XcmPallet", "limited_teleport_assets") => emoji_index("shuffle_tracks"),
				("XcmPallet", "reserve_transfer_assets") => emoji_index("shuffle_tracks"),
				("XTokens", "transfer") => emoji_index("shuffle_tracks"),
				("ZenlinkProtocol", "swap_exact_assets_for_assets") =>
					emoji_index("currency_exchange"),
				("Xyk", "sell_asset") => emoji_index("currency_exchange"),
				_ => {
					log!(
						"missing extrinsic {}, {}",
						details.pallet.as_str(),
						details.variant.as_str()
					);
					255
				},
			};
			ExStyle { color: as_rgbemoji_u32(rgb.red, rgb.green, rgb.blue, emoji) }
		},
	}
}

pub fn style_data_event(entry: &DataEvent) -> ExStyle {
	let darkside = entry.details.doturl.is_darkside();
	let raw = &entry.details;

	let alpha = match (raw.pallet.as_str(), raw.variant.as_str()) {
		("AcalaOracle", "NewFeedData") => emoji_index("crystal_ball"),
		("Assets", "ApprovedTransfer") => emoji_index("currency_exchange"),
		("AMMRoute", "Traded") => emoji_index("currency_exchange"),
		("AMM", "Traded") => emoji_index("currency_exchange"),
		("Assets", "Transferred") => emoji_index("currency_exchange"),
		("AMM", "LiquidityAdded") => emoji_index("chart_increasing"),
		("Assets", "Issued") => emoji_index("hatching_chick"),
		("Assets", "ApprovalCancelled") => emoji_index("warning"),
		("Attestation", "AttestationCreated") => emoji_index("black_nib"),
		("Assets", "Burned") => emoji_index("fire"),
		("AMM", "LiquidityRemoved") => emoji_index("chart_decreasing"),
		("Balances", "Unreserved") => emoji_index("unlocked"),
		("Balances", "BalanceSet") => emoji_index("bank"),
		("Balances", "Deposit") => emoji_index("bank"),
		("Balances", "Transfer") => emoji_index("shuffle_tracks"),
		("Balances", "Endowed") => emoji_index("bank"),
		("Balances", "Withdraw") => emoji_index("money_bag"),
		("Balances", "DustLost") => emoji_index("broom"),
		("Balances", "Reserved") => emoji_index("locked"),
		("Common", "TokenPropertySet") => emoji_index("wrench"),
		("CollatorRewards", "CollatorRewarded") => emoji_index("carrot"),
		("Currencies", "Withdrawn") => emoji_index("money_bag"),
		("Currencies", "Transferred") => emoji_index("shuffle_tracks"),
		("Currencies", "Deposited") => emoji_index("dollar"),
		("CrowdloanRewards", "RewardsPaid") => emoji_index("carrot"),
		("Crowdloan", "HandleBidResult") => emoji_index("crystal_ball"),
		("DmpQueue", "ExecutedDownward") => emoji_index("incoming_envelope"),
		("Dex", "Swap") => emoji_index("currency_exchange"),
		("Democracy", "Voted") => emoji_index("check_box_with_check"),
		("DappsStaking", "RewardDestination") => emoji_index("carrot"),
		("Datalog", "NewRecord") => emoji_index("black_nib"),
		("DicoOracle", "NewLockedPrice") => emoji_index("crystal_ball"),
		("DappsStaking", "Reward") => emoji_index("carrot"),
		("DappsStaking", "BondAndStake") => emoji_index("locked"),
		("Ethereum", "Executed") => emoji_index("brain"),
		("EVM", "Executed") => emoji_index("gear"),
		("EVM", "Log") => emoji_index("log"),
		("Farming", "Withdrawn") => emoji_index("farmer"),
		("Farming", "WithdrawClaimed") => emoji_index("farmer"),
		("Farming", "RewardPaid") => emoji_index("carrot"),
		("Farming", "Claimed") => emoji_index("farmer"),
		("Farming", "AssetsDeposited") => emoji_index("farmer"),
		("Farming", "Deposited") => emoji_index("farmer"),
		("FlexibleFee", "FlexibleFeeExchanged") => emoji_index("currency_exchange"),
		("FuelTanks", "AccountAdded") => emoji_index("baby_symbol"),
		("FuelTanks", "CallDispatched") => emoji_index("alembic"),
		("Homa", "Minted") => emoji_index("baby_symbol"),
		("Homa", "RequestedRedeem") => emoji_index("pause"),
		("Homa", "RedeemedByFastMatch") => emoji_index("money_bag"),
		("Homa", "RedeemRequestCancelled") => emoji_index("warning"),
		("Hrmp", "OpenChannelRequested") => emoji_index("loudspeaker"),
		("Incentives", "WithdrawDexShare") => emoji_index("chart_decreasing"),
		("Incentives", "ClaimRewards") => emoji_index("carrot"),
		("ImOnline", "HeartbeatReceived") => emoji_index("beating_heart"),
		("Lighthouse", "BlockReward") => emoji_index("carrot"),
		("Loans", "DistributedSupplierReward") => emoji_index("carrot"),
		("Loans", "DistributedBorrowerReward") => emoji_index("carrot"),
		("LiquidStaking", "StakingLedgerUpdated") => emoji_index("black_nib"),
		("Loans", "Deposited") => emoji_index("heavy_dollar"),
		("Loans", "PositionUpdated") => emoji_index("robot"),
		("Loans", "Redeemed") => emoji_index("money_bag"),
		("Loans", "RepaidBorrow") => emoji_index("star"),
		("Loans", "Borrowed") => emoji_index("money_bag"),
		("Loans", "RewardPaid") => emoji_index("carrot"),
		("Mining", "MiningResourceMintedTo") => emoji_index("pick"),
		("MultiTokens", "TokenCreated") => emoji_index("baby_symbol"),
		("MultiTokens", "CollectionAccountCreated") => emoji_index("baby_symbol"),
		("MultiTokens", "TokenAccountCreated") => emoji_index("baby_symbol"),
		("MultiTokens", "Minted") => emoji_index("pick"),
		("MultiTokens", "TokenAccountDestroyed") => emoji_index("fire"),
		("MultiTokens", "Transferred") => emoji_index("shuffle_tracks"),
		("Multisig", "MultisigApproval") => emoji_index("busts"),
		("Multisig", "NewMultisig") => emoji_index("busts"),
		("Multisig", "MultisigExecuted") => emoji_index("gear"),
		("MoonbeamOrbiters", "OrbiterRewarded") => emoji_index("carrot"),
		("NominationPools", "PaidOut") => emoji_index("money_bag"),
		("NominationPools", "Bonded") => emoji_index("locked"),
		("NominationPools", "Unbonded") => emoji_index("unlocked"),
		("Nft", "TransferedNft") => emoji_index("artistic_palette"),
		("Oracle", "NewPrice") => emoji_index("crystal_ball"),
		("Oracle", "NewFeedData") => emoji_index("crystal_ball"),
		("PolkadotXcm", "AssetsTrapped") => emoji_index("warning"),
		("PolkadotXcm", "Sent") => emoji_index("envelope_with_arrow"),
		("ParachainStaking", "Delegation") => emoji_index("busts"),
		("ParachainSystem", "DownwardMessagesReceived") => emoji_index("incoming_envelope"),
		("ParachainSystem", "DownwardMessagesProcessed") => emoji_index("incoming_envelope"),
		("ParachainStaking", "DelegationRevocationScheduled") => emoji_index("alarm_clock"),
		("ParachainStaking", "ReservedForParachainBond") => emoji_index("locked"),
		("ParachainStaking", "CollatorChosen") => emoji_index("game_die"),
		("ParachainStaking", "NewRound") => emoji_index("baby_symbol"),
		("ParachainStaking", "AutoCompoundSet") => emoji_index("wrench"),
		("ParachainStaking", "DelegatorLeftCandidate") => emoji_index("ghost"),
		("ParachainStaking", "DelegationRevoked") => emoji_index("warning"),
		("ParachainStaking", "DelegationKicked") => emoji_index("skull"),
		("ParachainStaking", "DelegatorLeft") => emoji_index("ghost"),
		("PolkadotXcm", "Attempted") => emoji_index("alembic"),
		("ParachainStaking", "DelegationIncreased") => emoji_index("chart_increasing"),
		("ParaInclusion", "CandidateIncluded") => emoji_index("anchor"),
		("ParaInclusion", "CandidateBacked") => emoji_index("cowboy_hat_face"),
		("ParachainStaking", "Compounded") => emoji_index("heavy_dollar"),
		("ParachainStaking", "Rewarded") => emoji_index("carrot"),
		("Proxy", "ProxyExecuted") => emoji_index("ghost"),
		("Proxy", "Announced") => emoji_index("ghost"),
		("RWS", "NewCall") => emoji_index("alien_monster"),
		("RelayChainInfo", "CurrentBlockNumbers") => emoji_index("black_nib"),
		("Sudo", "Sudid") => emoji_index("cowboy_hat_face"),
		("System", "ExtrinsicSuccess") => emoji_index("thumbs_up"),
		("System", "ExtrinsicFailed") => emoji_index("warning"),
		("System", "NewAccount") => emoji_index("hatching_chick"),
		("Staking", "Unbonded") => emoji_index("unlocked"),
		("StableAsset", "BalanceUpdated") => emoji_index("robot"),
		("StableAsset", "FeeCollected") => emoji_index("palm_up_hand"),
		("StableAsset", "TokenSwapped") => emoji_index("currency_exchange"),
		("Session", "NewSession") => emoji_index("baby_symbol"),
		("Staking", "Chilled") => emoji_index("snowflake"),
		("Staking", "ValidatorPrefsSet") => emoji_index("black_nib"),
		("Scheduler", "Canceled") => emoji_index("warning"),
		("Scheduler", "Scheduled") => emoji_index("calendar"),
		("Scheduler", "Dispatched") => emoji_index("gear"),
		("Staking", "PayoutStarted") => emoji_index("gem_stone"),
		("Staking", "Rewarded") => emoji_index("carrot"),
		("Staking", "Withdrawn") => emoji_index("unlocked"),
		("Staking", "Bonded") => emoji_index("locked"),
		("Slp", "TimeUnitUpdated") => emoji_index("alarm_clock"),
		("System", "KilledAccount") => emoji_index("headstone"),
		("TransactionPayment", "TransactionFeePaid") => emoji_index("palm_up_hand"),
		("Treasury", "Deposit") => emoji_index("pig"),
		("Treasury", "Spending") => emoji_index("heavy_dollar"),
		("Treasury", "Burnt") => emoji_index("fire"),
		("Treasury", "Rollover") => emoji_index("counter_clockwise_arrows"),
		("Tokens", "DustLost") => emoji_index("broom"),
		("Tokens", "Withdrawn") => emoji_index("money_bags"),
		("Tokens", "Deposited") => emoji_index("heavy_dollar"),
		("Tokens", "Endowed") => emoji_index("pill"),
		("Tokens", "Transfer") => emoji_index("shuffle_tracks"),
		("Ump", "UpwardMessagesReceived") => emoji_index("incoming_envelope"),
		("Ump", "ExecutedUpward") => emoji_index("incoming_envelope"),
		("Utility", "BatchCompleted") => emoji_index("thumbs_up"),
		("Utility", "BatchCompletedWithErrors") => emoji_index("warning"),
		("Utility", "ItemCompleted") => emoji_index("thumbs_up"),
		("Utility", "ItemFailed") => emoji_index("warning"),
		("Vesting", "VestingUpdated") => emoji_index("robot"),
		("Vesting", "Claimed") => emoji_index("gem_stone"),
		("VtokenMinting", "RedeemSuccess") => emoji_index("thumbs_up"),
		("VtokenMinting", "Minted") => emoji_index("pick"),
		("VtokenMinting", "Redeemed") => emoji_index("cake"),
		("VoterList", "Rebagged") => emoji_index("robot"),
		("VoterList", "ScoreUpdated") => emoji_index("black_nib"),
		("Vesting", "VestingCompleted") => emoji_index("face_savoring_food"),
		("XcmpQueue", "XcmpMessageSent") => emoji_index("envelope_with_arrow"),
		("XTokens", "TransferredMultiAssets") => emoji_index("crystal_ball"),
		("XcmPallet", "Attempted") => emoji_index("gear"),
		("XcmpQueue", "Success") => emoji_index("thumbs_up"),
		("Xyk", "AssetsSwapped") => emoji_index("currency_exchange"),
		("XcmpQueue", "Fail") => emoji_index("warning"),
		("ZenlinkProtocol", "LiquidityAdded") => emoji_index("chart_increasing"),
		("ZenlinkProtocol", "AssetSwap") => emoji_index("currency_exchange"),

		_ => {
			log!("missing {} {}", raw.pallet.as_str(), raw.variant.as_str());
			255
		},
	};

	// let msg = crate::content::is_event_message(entry);
	if matches!(
		(raw.pallet.as_str(), raw.variant.as_str()),
		("System", "ExtrinsicFailed") /* | ("PolkadotXcm", "Attempted") - only an error if
		                               * !completed variant. */
	) || entry.details.success == Success::Sad
	{
		return ExStyle { color: as_rgbemoji_u32(1., 0., 0., alpha) }
	}
	if entry.details.success == Success::Worried {
		return ExStyle {
			// Trump Orange
			color: as_rgbemoji_u32(1., 0.647_058_84, 0., alpha),
		}
	}

	let color = palette::Lchuv::new(
		if darkside { 40. } else { 80. },
		80. + (calculate_hash(&raw.variant) as f32 % 100.),
		(calculate_hash(&raw.pallet) as f32) % 360.,
	);
	let rgb: palette::rgb::Srgb = palette::rgb::Srgb::from_color(color);

	// println!("rgb {} {} {}", rgb.red, rgb.green, rgb.blue);

	ExStyle { color: as_rgbemoji_u32(rgb.red, rgb.green, rgb.blue, alpha) }
}
