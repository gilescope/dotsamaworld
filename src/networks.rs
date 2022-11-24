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


/// Return the network(s) to visulise
pub fn get_network(selected_env: &Env) -> Vec<Vec<(Option<u32>, Vec<&'static str>)>> {
	match selected_env {
		Env::Test => {
			vec![
				vec![
					(None, vec!["westend-rpc.polkadot.io"]),
					(Some(1000), vec!["westmint-rpc.polkadot.io"]),
					(Some(1001), vec!["westend-collectives-rpc.polkadot.io"]),
					// (Some(2000), "fullnode-collator.charcoal.centrifuge.io"),
					// (Some(2000), "teerw1.integritee.network"),
					// (Some(2000), "westend.kylin-node.co.uk"),
					// (Some(2000), "rpc.westend.standard.tech"),
					// (Some(2000), "westend.kilt.io:9977"),
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
					(Some(1000), vec!["statemine-rpc.polkadot.io"]),
					// (Some(1001), "kusama.api.encointer.org"),
					// //
					// kusama Auction Batch 1
					(Some(2000), vec!["karura-rpc-0.aca-api.network"]), // 1st
					(Some(2023), vec!["wss.api.moonriver.moonbeam.network"]), // 2nd.
					(Some(2007), vec!["rpc.shiden.astar.network"]),     // 3rd
					(Some(2004), vec!["khala-api.phala.network/ws"]),   // 4th
					(Some(2001), vec!["hk.p.bifrost-rpc.liebi.com/ws"]), // 5th
					// //
					// kusama Auction Batch 2
					(Some(2086), vec!["kilt-rpc.dwellir.com"]),          // 6th
					(Some(2084), vec!["ws.calamari.systems", "calamari-rpc.dwellir.com"]),      // 7th
					(Some(2090), vec!["basilisk-rpc.dwellir.com"]),      // 8th
					(Some(2088), vec!["fullnode.altair.centrifuge.io"]), //9th
					(Some(2085), vec!["heiko-rpc.parallel.fi"]),         // 10th
					(Some(2092), vec!["wss://api-kusama.interlay.io:443/parachain","wss://kintsugi.api.onfinality.io:443/public-ws",  "kintsugi-rpc.dwellir.com"]),      // 11th
					// //
					// kusama Auction Batch 3
					(Some(2087), vec!["picasso-rpc.composable.finance"]), // 12th
					(Some(2097), vec!["pioneer.api.onfinality.io/public-ws", "pioneer-1-rpc.bit.country"]),      // 13th
					(Some(2095), vec!["us-ws-quartz.unique.network"]),    // 14th
					// //15th genshiro

					// kusama Auction Batch 4
					(Some(2100), vec!["para.f3joule.space", "para.subsocial.network"]),    // 16th
					(Some(2101), vec!["zeitgeist-rpc.dwellir.com"]), // 17th
					//Sakura 18th
					(Some(2012), vec!["rpc-shadow.crust.network"]), // 19th
					(Some(2048), vec!["kusama.rpc.robonomics.network"]), // 20th
					// //
					// kusama Auction Batch 5
					(Some(2015), vec!["kusama.api.integritee.network"]), // 21st
					(Some(2105), vec!["crab-parachain-rpc.darwinia.network"]), // 22nd
					(Some(2106), vec!["rpc.litmus-parachain.litentry.io"]), // 23rd
					//"ws.parachain-collator-1.c1.sora2.soramitsu.co.jp", // 24th
					(Some(2107), vec!["rpc.kico.dico.io", "rpc.api.kico.dico.io"]), // 25th
					// //
					// kusama Auction Batch 6
					(Some(2110), vec!["prod-kusama-collator-01.mangatafinance.cloud"]), // 26th
					// // 27th renewal moonriver
					// // 28th renewal kilt 
					// // 29th renewal karura
					(Some(2114), vec!["rpc.turing.oak.tech"]), // 30th Oak Turing network
															
					// kusama Auction Batch 7
					(Some(2102), vec!["wss://pichiu.api.onfinality.io:443/public-ws"]), // * 31st not online yet
					// * 32nd renewal khala
					(Some(2115), vec!["kusama.dorafactory.org"]), // * Dora Factory 33rd
					// * 34nd renewal bifrost
					// * 35nd renewal shiden

					// kusama Auction Batch 8
					(Some(2118), vec!["wss.mainnet.listen.io"]),//* Listen 36th 
					(Some(2119), vec!["wss://bajun.api.onfinality.io:443/public-ws"]),//* 37th Bajun 
					(Some(2113), vec!["kabocha.jelliedowl.net"]), // 38th Kabocha
					// (Some(2116), vec![]),// 39th Tanganika Network
					(Some(2121), vec!["imbue-kusama.imbue.network"]),// 40th Imbue network
					//41: Calimari renewal

					// kusama auction batch 9
					(Some(2124), vec!["rpc-amplitude.pendulumchain.tech"]),// 42: Amplitude
					(Some(2125), vec!["tinker.invarch.network"]),// 43: Tinkernet
					// 44: renewal kinsugi
					// 45: renewal heiko finance
					// 46: renewal Altair
					// 47: renewal Basilisk

					// Kusama Auction Batch 10
					(Some(2123), vec!["intern.gmordie.com"]),// 48: GM Parachain
					// 49: parathread 2130
					// 50: renewal subsocial
					(Some(2129), vec!["snow-rpc.icenetwork.io"]),// 51: Snow
					// 52: renewal bit.country
				],
				vec![
					// TODO: how can we dynamically discover
					// nodes we can hit? - can we track back to the
					// collator ip?
					(None, vec!["rpc.polkadot.io"]),
					(Some(1000), vec!["statemint-rpc.polkadot.io"]), //1st parachain.
					(Some(1001), vec!["polkadot-collectives-rpc.polkadot.io"]),
					//
					// polkadot Auction Batch 1
					(Some(2000), vec!["acala-rpc-1.aca-api.network", "acala.polkawallet.io"]),     // 1st auction winner
					(Some(2004), vec!["wss.api.moonbeam.network"]), // 2nd
					(Some(2006), vec!["rpc.astar.network"]),    // 3rd
					(Some(2012), vec!["rpc.parallel.fi"]),          // 4th
					// //(Some(2002), vec!["rpc-para.clover.finance"),  // 5th - closed.
					// //
					// polkadot Auction Batch 2
					(Some(2021), vec!["rpc.efinity.io"]),                           // 6th
					(Some(2019), vec!["rpc.composable.finance"]),                   // 7th
					(Some(2031), vec!["fullnode.parachain.centrifuge.io"]),         // 8th
					(Some(2034), vec!["rpc.hydradx.cloud", "rpc-01.hydradx.io"]),                        // 9th
					// (Some(2032), vec![ "interlay.api.onfinality.io:443/public-ws"]), // 10th 
					(Some(2026), vec!["wss://nodle-parachain.api.onfinality.io:443/public-ws","eden-rpc.dwellir.com"]),                     // noodle 11th
					// //
					// polkadot Auction Batch 3
					(Some(2011), vec!["node.pol.equilibrium.io"]),        // 12th
					(Some(2035), vec!["wss://api.phala.network:443/ws"]), //13th
					(Some(2037), vec!["ws.unique.network"]),              // 14th
					(Some(2013), vec!["rpc.litentry-parachain.litentry.io"]), // 15th
					                                //    * "mainnet.polkadex.trade", // 16th (not on line yet)
					(Some(2043), vec!["wss://parachain-rpc.origin-trail.network:443"]), // * 17th origin trail 
					(Some(2030), vec!["wss://hk.p.bifrost-rpc.liebi.com:443/ws"]),                //    * 18th Bifrost polkadot

					// polkadot Auction Batch 4
					// (Some(2027), vec![]), // 19th Coinversation
					(Some(2007), vec!["k-ui.kapex.network"]), // 20th Totem Kapex
					(Some(2046), vec!["parachain-rpc.darwinia.network"]), // 21st Darwinia
					// 22nd Parathread 2055?
					(Some(2039), vec!["polkadot.api.integritee.network"]),// 23rd Integritee polkadot
					(Some(2086), vec!["kilt-rpc.dwellir.com"]),// 24th kilt
					(Some(2052), vec!["polkadot.kylin-node.co.uk"]),// 25th Kylin network

					// polkadot Auction Batch 5
					// (Some(2056), vec![""]), // 26th Aventus network
					// (Some(), vec![""]), // 27th Watr
					// (Some(2090), vec![]), // 28th Oak Network
					(Some(2048), vec!["mainnet.bitgreen.org"]), // 29th BitGreen
					// (Some(2008), vec![""]), // 30th CrustNetwork
					(Some(2051), vec!["rpc-parachain.ajuna.network"]), // 31st Ajuna network
					// 32nd parathread 2092

					// polkadot Auction Batch 6

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
		// 			"wss://mainnet.chainx.org:443/ws",
		// 			"rpc.coinversation.io",
		// 			"wss://node0.competitors.club:443/wss",
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
		// 			"wss://rpc.kulupu.corepaper.org:443/ws",
		// 			"ws.kusari.network",
		// 			"wss://mathchain-asia.maiziqianbao.net:443/ws",
		// 			"wss://rpc.neatcoin.org:443/ws",
		// 			"wss://mainnet.nftmart.io:443/rpc/ws",
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
		// 			"wss://moonbeam-alpha.api.onfinality.io:443/public-ws",
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
					(Some(1000), vec!["ws://127.0.0.1:9910"]),
					(Some(2000), vec!["ws://127.0.0.1:9920"]),
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
