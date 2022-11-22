use serde::{Deserialize, Serialize};
use std::fmt::write;

#[allow(dead_code)]
#[derive(Default, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Env {
	Local,
	Test,
	#[default]
	Prod,
	// SelfSovereign,
	// SelfSovereignTest,
	// NFTs,
	// CGP,
}

impl std::fmt::Display for Env {
	fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
		let display = match self {
			Env::Local => "local",
			Env::Test => "test",
			Env::Prod => "dotsama",
			// Env::SelfSovereign => "independents",
			// Env::SelfSovereignTest => "independent_test",
			// Env::NFTs => "nfts",
			// Env::CGP => "cgp",
		};
		write(fmt, format_args!("{}", display))?;
		Ok(())
	}
}

impl TryFrom<&str> for Env {
	type Error = ();
	fn try_from(val: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
		match val {
			"dotsama" => Ok(Env::Prod),
			"test" => Ok(Env::Test),
			"local" => Ok(Env::Local),
			_ => Err(()),
		}
	}
}

impl Env {
	pub fn is_self_sovereign(&self) -> bool {
		false
		//		matches!(self, Env::SelfSovereign | Env::SelfSovereignTest)
	}
}

macro_rules! para_id {
	($lit:literal) => {
		Some($lit)
	};
}

/// Return the network(s) to visulise
pub fn get_network(selected_env: &Env) -> Vec<Vec<(Option<u32>, Vec<&'static str>)>> {
	match selected_env {
		Env::Test => {
			vec![
				vec![
					(None, vec!["westend-rpc.polkadot.io"]),
					(para_id!(1000), vec!["westmint-rpc.polkadot.io"]),
					(para_id!(1001), vec!["westend-collectives-rpc.polkadot.io"]),
					// (para_id!(2000), "fullnode-collator.charcoal.centrifuge.io"),
					// (para_id!(2000), "teerw1.integritee.network"),
					// (para_id!(2000), "westend.kylin-node.co.uk"),
					// (para_id!(2000), "rpc.westend.standard.tech"),
					// (para_id!(2000), "westend.kilt.io:9977"),
				],
		// 		vec![
		//             "rococo-rpc.polkadot.io",
		//             "rococo-statemint-rpc.polkadot.io",
		//             "rococo-canvas-rpc.polkadot.io",
		//             "rococo.api.encointer.org",
		//             "rpc-rococo.bajun.network",
		//             "rococobitgreen.abhath-labs.com",
		//             "rpc-01.basilisk-rococo.hydradx.io",
		//             "fullnode.catalyst.cntrfg.com",
		//             "anjie.rococo.dolphin.engineering",
		//             "rpc.composablefinance.ninja",
		//             "rpc.rococo.efinity.io",
		//             "rococo.api.integritee.network",
		//             "rpc.rococo-parachain-sg.litentry.io",
		//             "moonsama-testnet-rpc.moonsama.com",
		//             "wss://parachain-testnet.equilab.io:443/rococo/collator/node1/wss",
		
		// "node-6913072722034561024.lh.onfinality.io:443/ws?
		// apikey=84d77e2e-3793-4785-8908-5096cffea77a", //noodle
		// "pangolin-parachain-rpc.darwinia.network",             "rococo.kilt.io",
		//             "dev.net.t3rn.io",
		//             "rococo.rpc.robonomics.network",
		//             "rco-para.subsocial.network",
		//             "wss://invarch-tinkernet.api.onfinality.io:443/public-ws",
		//             "spreehafen.datahighway.com",
		//             "testnet.creditcoin.network"
		//             // "ws://127.0.0.1:9944",
		//             // "ws://127.0.0.1:9966",
		//             // "ws://127.0.0.1:9920",
		//         ],
			]
		},
		Env::Prod => {
			// for history mode to work well we need to be pointing to archive nodes
			// ( those running --pruning=archive )
			// othewise you hit "State already discarded for BlockId"
			vec![
				vec![
					// Ordering should really be done on who won the auction first!
					(None, vec!["kusama-rpc.polkadot.io"]),
					(para_id!(1000), vec!["statemine-rpc.polkadot.io"]),
					// (para_id!(1001), "kusama.api.encointer.org"),
					// //
					// // Auction Batch 1
					(para_id!(2000), vec!["karura-rpc-0.aca-api.network"]), // 1st
					(para_id!(2023), vec!["wss.api.moonriver.moonbeam.network"]), // 2nd.
					(para_id!(2007), vec!["rpc.shiden.astar.network"]),     // 3rd
					(para_id!(2004), vec!["khala-api.phala.network/ws"]),   // 4th
					(para_id!(2001), vec!["hk.p.bifrost-rpc.liebi.com/ws"]), // 5th
					// //
					// // Auction Batch 2
					(para_id!(2086), vec!["kilt-rpc.dwellir.com"]),          // 6th
					(para_id!(2084), vec!["ws.calamari.systems", "calamari-rpc.dwellir.com"]),      // 7th
					(para_id!(2090), vec!["basilisk-rpc.dwellir.com"]),      // 8th
					(para_id!(2088), vec!["fullnode.altair.centrifuge.io"]), //9th
					(para_id!(2085), vec!["heiko-rpc.parallel.fi"]),         // 10th
					(para_id!(2092), vec!["kintsugi.api.onfinality.io:433/public-ws", "api-kusama.interlay.io:433/parachain", "kintsugi-rpc.dwellir.com"]),      // 11th
					// //
					// // Auction Batch 3
					(para_id!(2087), vec!["picasso-rpc.composable.finance"]), // 12th
					(para_id!(2097), vec!["pioneer.api.onfinality.io/public-ws", "pioneer-1-rpc.bit.country"]),      // 13th
					(para_id!(2095), vec!["us-ws-quartz.unique.network"]),    // 14th
					// //15th genshiro

					// // Auction Batch 4
					(para_id!(2100), vec!["para.f3joule.space", "para.subsocial.network"]),    // 16th
					(para_id!(2101), vec!["zeitgeist-rpc.dwellir.com"]), // 17th
					//Sakura 18th
					(para_id!(2012), vec!["rpc-shadow.crust.network"]), // 19th
					(para_id!(2048), vec!["kusama.rpc.robonomics.network"]), // 20th
					// //
					// // Auction Batch 5
					(para_id!(2015), vec!["kusama.api.integritee.network"]), // 21st
					(para_id!(2105), vec!["crab-parachain-rpc.darwinia.network"]), // 22nd
					(para_id!(2106), vec!["rpc.litmus-parachain.litentry.io"]), // 23rd
					//"ws.parachain-collator-1.c1.sora2.soramitsu.co.jp", // 24th
					(para_id!(2107), vec!["rpc.kico.dico.io", "rpc.api.kico.dico.io"]), // 25th
					// //
					// // Auction Batch 6
					(para_id!(2110), vec!["prod-kusama-collator-01.mangatafinance.cloud"]), // 26th
					// // 27th renewal
					// // 28th renewal     double click faster, yaw and pitch
					// // 29th renewal
					(para_id!(2114), vec!["rpc.turing.oak.tech"]), /* 30th
															  // * Auction Batch 7
															  // * "kusama.kylin-node.co.uk", 31st not online yet
															  // * 32nd renewal
															  // * Dora Factory (not yet online) 33rd
															  // * 34nd renewal
															  // * 35nd renewal */

															 /* Auction Batch 8
															  * Listen (not online yet) 36th */
				],
				vec![
					// TODO: how can we dynamically discover
					// nodes we can hit? - can we track back to the
					// collator ip?
					(None, vec!["rpc.polkadot.io"]),
					(para_id!(1000), vec!["statemint-rpc.polkadot.io"]), //1st parachain.
					(para_id!(1001), vec!["polkadot-collectives-rpc.polkadot.io"]),
					//
					// Auction Batch 1
					(para_id!(2000), vec!["acala-rpc-0.aca-api.network", "acala.polkawallet.io"]),     // 1st auction winner
					(para_id!(2004), vec!["wss.api.moonbeam.network"]), // 2nd
					(para_id!(2006), vec!["rpc.astar.network"]),    // 3rd
					(para_id!(2012), vec!["rpc.parallel.fi"]),          // 4th
					// //(Some(2002), vec!["rpc-para.clover.finance"),  // 5th - closed.
					// //
					// // Auction Batch 2
					(para_id!(2021), vec!["rpc.efinity.io"]),                           // 6th
					(para_id!(2019), vec!["rpc.composable.finance"]),                   // 7th
					(para_id!(2031), vec!["fullnode.parachain.centrifuge.io"]),         // 8th
					(para_id!(2034), vec!["rpc.hydradx.cloud", "rpc-01.hydradx.io"]),                        // 9th
					(para_id!(2032), vec![ "interlay.api.onfinality.io:443/public-ws"]), // 10th 
					(para_id!(2026), vec!["wss://nodle-parachain.api.onfinality.io:433/public-ws","eden-rpc.dwellir.com"]),                     // noodle 11th
					// //
					// // Auction Batch 3
					(para_id!(2011), vec!["node.pol.equilibrium.io"]),        // 12th
					(para_id!(2035), vec!["wss://api.phala.network:443/ws"]), //13th
					(para_id!(2037), vec!["ws.unique.network"]),              // 14th
					(para_id!(2013), vec!["rpc.litentry-parachain.litentry.io"]), /* 15th
					                                //    * "mainnet.polkadex.trade", // 16th (not on line yet)
					                                //    * 17th origin trail (not live yet)
					                                //    * "k-ui.kapex.network", */
				],
			]
		},
		// // Common good parachains
		// Env::CGP => {
		// 	vec![
		// 		vec![
		// 			"kusama-rpc.polkadot.io",
		// 			"statemine-rpc.dwellir.com",
		// 			"kusama.api.encointer.org",
		// 		],
		// 		vec![
		// 			"rpc.polkadot.io",
		// 			"statemint-rpc.polkadot.io", //1st parachain.
		// 		],
		// 	]
		// },

		// Env::SelfSovereign => {
		// 	vec![
		// 		//Live
		// 		vec![
		// 			//The first one on the list is treated like a relay so use whichever ticks
		// 			// fastest.
		// 			"ws.azero.dev",
		// 			"api.ata.network",
		// 			"fullnode.centrifuge.io",
		// 			"wss://mainnet.chainx.org:433/ws",
		// 			"rpc.coinversation.io",
		// 			"wss://node0.competitors.club:433/wss",
		// 			"blockchain.crownsterling.io",
		// 			"rpc.crust.network",
		// 			"rpc.darwinia.network",
		// 			"crab-rpc.darwinia.network",
		// 			"mainnet-node.dock.io",
		// 			"mainnet.edgewa.re",
		// 			"rpc.efinity.io",
		// 			"node.equilibrium.io",
		// 			"node.genshiro.io",
		// 			"rpc.hanonycash.com",
		// 			"archive.snakenet.hydradx.io",
		// 			"api.solo.integritee.io",
		// 			"wss://rpc.kulupu.corepaper.org:433/ws",
		// 			"ws.kusari.network",
		// 			"wss://mathchain-asia.maiziqianbao.net:433/ws",
		// 			"wss://rpc.neatcoin.org:433/ws",
		// 			"wss://mainnet.nftmart.io:433/rpc/ws",
		// 			"main3.nodleprotocol.io",
		// 			"rpc.plasmnet.io",
		// 			"mainnet.polkadex.trade",
		// 			"mainnet-rpc.polymesh.network",
		// 			"node.v1.riochain.io",
		// 			"kusama.rpc.robonomics.network",
		// 			"mainnet.sherpax.io",
		// 			"ws.alb.sora.org",
		// 			"wss.spannerprotocol.com",
		// 			"mainnet-rpc.stafi.io",
		// 			"mainnet.subgame.org",
		// 			"rpc.subsocial.network",
		// 			"ws.swapdex.network",
		// 			"wss://mainnet.uniarts.vip:9443",
		// 			"westlake.datahighway.com",
		// 		],
		// 		vec![],
		// 	]
		// },
		// Env::SelfSovereignTest => {
		// 	//Test
		// 	vec![
		// 		vec![
		// 			"ws.test.azero.dev",
		// 			"rpc-test.ajuna.network",
		// 			"fullnode.amber.centrifuge.io",
		// 			"arcadia1.nodleprotocol.io",
		// 			"gladios.aresprotocol.io",
		// 			"wss://contextfree.api.onfinality.io:443/public-ws",
		// 			"beresheet.edgewa.re",
		// 			"wss://asgard-rpc.liebi.com:443/ws",
		// 			"tewai-rpc.bit.country",
		// 			"api.clover.finance",
		// 			"rpc.coinversation.io",
		// 			"api.crust.network",
		// 			"knox-1.dock.io",
		// 			"trillian.dolphin.red",
		// 			"mogiway-01.dotmog.com",
		// 			"gesell.encointer.org",
		// 			"testnet.equilibrium.io",
		// 			"substrate-rpc.parity.io",
		// 			"test-ws.fantour.io",
		// 			"galital-rpc-testnet.starkleytech.com",
		// 			"wss://galois.maiziqianbao.net:443/ws",
		// 			"gamepower.io",
		// 			"testnet.geekcash.org",
		// 			"halongbay.polkafoundry.com",
		// 			"wss://api-testnet.interlay.io/parachain/",
		// 			"ws.jupiter-poa.patract.cn",
		// 			"wss://full-nodes.kilt.io:9944/",
		// 			"wss://peregrine.kilt.io/parachain-public-ws/",
		// 			"klugdossier.net",
		// 			"testnet.kylin-node.co.uk",
		// 			"mandala.polkawallet.io",
		// 			"ws.f1.testnet.manta.network",
		// 			"wss://moonbeam-alpha.api.onfinality.io:433/public-ws",
		// 			"wss://mybank.network/substrate",
		// 			"wss://neumann.api.onfinality.io:443/public-ws",
		// 			"staging-ws.nftmart.io",
		// 			"opal.unique.network",
		// 			"rpc.opportunity.standard.tech",
		// 			"parachain-rpc.origin-trail.network",
		// 			"pangolin-rpc.darwinia.network",
		// 			"pangoro-rpc.darwinia.network",
		// 			"wss://poc5.phala.network/ws",
		// 			"phoenix-ws.coinid.pro",
		// 			"blockchain.polkadex.trade",
		// 			"testnet-rpc.polymesh.live",
		// 			"wss://testnet.pontem.network:443/ws",
		// 			"rpc.realis.network",
		// 			"node.v1.staging.riochain.io",
		// 			"sherpax-testnet.chainx.org",
		// 			"rpc.shibuya.astar.network",
		// 			"parachain-rpc.snowbridge.network",
		// 			"ws.stage.sora2.soramitsu.co.jp",
		// 			"alpha.subdao.org",
		// 			"farm-rpc.subspace.network",
		// 			"test-rpc.subspace.network",
		// 			"testnet.ternoa.com",
		// 			"wss://testnet-node-1.laminar-chain.laminar.one:443/ws",
		// 			"testnet.uniarts.network",
		// 			"testnet2.unique.network",
		// 			"unitventures.io",
		// 			"wss://vodka.rpc.neatcoin.org:443/ws",
		// 			"testnet.web3games.org",
		// 			"test1.zcloak.network",
		// 			"bsr.zeitgeist.pm",
		// 			"alphaville.zero.io",
		// 			"testnet.imbue.network",
		// 		],
		// 		vec![],
		// 	]
		// },
		Env::Local => {
			//TODO: we should have different ports for kusama and polkadot
			// so both can exist at the same time.
			vec![
				vec![
					(None, vec!["ws://127.0.0.1:9900"]),
					(para_id!(1000), vec!["ws://127.0.0.1:9910"]),
					(para_id!(2000), vec!["ws://127.0.0.1:9920"]),
				],
				// vec!["ws://127.0.0.1:9944", "ws://127.0.0.1:9966", "ws://127.0.0.1:9920"]
			]
		},
		// Env::NFTs => {
		// 	// These are parachains known to be rocking the uniques pallet:
		// 	vec![
		// 		vec![
		// 			"rpc.polkadot.io",
		// 			"statemint-rpc.polkadot.io",
		// 			"ws.unique.network",
		// 			"rpc-01.hydradx.io",
		// 		],
		// 		vec![
		// 			"kusama-rpc.polkadot.io",
		// 			"statemine-rpc.dwellir.com",
		// 			"us-ws-quartz.unique.network",
		// 			"basilisk-rpc.dwellir.com",
		// 		],
		// 	]
		// }, // TODO; networks with ink: astar, shiden, phala, peeq, aleph zero
	}
}
