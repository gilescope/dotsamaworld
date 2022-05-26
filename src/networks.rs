#[allow(dead_code)]
pub enum Env {
    Local,
    Test,
    Prod,
    SelfSovereign,
    SelfSovereignTest,
    NFTs,
}

impl Env {
    pub fn is_self_sovereign(&self) -> bool {
        matches!(self, Env::SelfSovereign | Env::SelfSovereignTest)
    }
}

/// Return the network(s) to visulise
pub fn get_network(selected_env: &Env) -> Vec<Vec<&'static str>> {
    match selected_env {
        Env::Test => {
            vec![
                vec![
                    "westend-rpc.dwellir.com",
                    "westmint-rpc.polkadot.io",
                    "fullnode-collator.charcoal.centrifuge.io",
                    "teerw1.integritee.network",
                    "westend.kylin-node.co.uk",
                    "rpc.westend.standard.tech",
                    "westend.kilt.io:9977",
                ],
                vec![
                    "rococo-rpc.polkadot.io",
                    "rococo-statemint-rpc.polkadot.io",
                    "rococo-canvas-rpc.polkadot.io",
                    "rococo.api.encointer.org",
                    "rpc-rococo.bajun.network",
                    "rococobitgreen.abhath-labs.com",
                    "rpc-01.basilisk-rococo.hydradx.io",
                    "fullnode.catalyst.cntrfg.com",
                    "anjie.rococo.dolphin.engineering",
                    "rpc.composablefinance.ninja",
                    "rpc.rococo.efinity.io",
                    "rococo.api.integritee.network",
                    "rpc.rococo-parachain-sg.litentry.io",
                    "moonsama-testnet-rpc.moonsama.com",
                    "wss://parachain-testnet.equilab.io:443/rococo/collator/node1/wss",
                    "node-6913072722034561024.lh.onfinality.io:443/ws?apikey=84d77e2e-3793-4785-8908-5096cffea77a", //noodle
                    "pangolin-parachain-rpc.darwinia.network",
                    "rococo.kilt.io",
                    "dev.net.t3rn.io",
                    "rococo.rpc.robonomics.network",
                    "rco-para.subsocial.network",
                    "wss://invarch-tinkernet.api.onfinality.io:443/public-ws",
                    "spreehafen.datahighway.com",
                    "testnet.creditcoin.network"
                    // "ws://127.0.0.1:9944",
                    // "ws://127.0.0.1:9966",
                    // "ws://127.0.0.1:9920",       
                ],
            ]
        }
        Env::Prod => {
            vec![
                 vec![
                     "rpc.polkadot.io",
                      "statemint-rpc.polkadot.io",
                    "acala.polkawallet.io",
                    "astar-rpc.dwellir.com",
                    "fullnode.parachain.centrifuge.io",
                    "rpc-para.clover.finance",
                    "rpc.efinity.io",
                    "rpc-01.hydradx.io",
                    "interlay.api.onfinality.io:443/public-ws",
                    "wss.api.moonbeam.network",
                    "eden-rpc.dwellir.com", //noodle
                    "rpc.parallel.fi",
                    "wss://api.phala.network:443/ws",
                    // "mainnet.polkadex.trade",
                    "ws.unique.network",
                    "k-ui.kapex.network",
                   
                 ],
                vec![
                    "kusama-rpc.polkadot.io",
                     "statemine-rpc.dwellir.com",
                    "wss.api.moonriver.moonbeam.network",
                    "bifrost-rpc.dwellir.com",
                    "khala-rpc.dwellir.com",
                    "shiden-rpc.dwellir.com",
                    "rpc-shadow.crust.network",
                    "kusama.api.integritee.network",
                    "kusama.rpc.robonomics.network",
                    "calamari-rpc.dwellir.com",
                    "heiko-rpc.parallel.fi",
                    "kilt-rpc.dwellir.com",
                    "picasso-rpc.composable.finance",
                    "basilisk-rpc.dwellir.com",
                    "kintsugi-rpc.dwellir.com",
                    "us-ws-quartz.unique.network",
                    "para.subsocial.network",
                    "rpc.api.kico.dico.io",
                    "zeitgeist-rpc.dwellir.com",
                    "crab-parachain-rpc.darwinia.network",
                    "rpc.litmus-parachain.litentry.io",
                    "karura-rpc.dwellir.com",
                    "fullnode.altair.centrifuge.io",
                    "pioneer-1-rpc.bit.country",
                    "rpc.turing.oak.tech",
                ],
            ]
        }

        Env::SelfSovereign => {
            vec![
                //Live
                vec![
                    //The first one on the list is treated like a relay so use whichever ticks fastest.
                    "ws.azero.dev",
                    "api.ata.network",
                    "fullnode.centrifuge.io",
                    "wss://mainnet.chainx.org:433/ws",
                    "rpc.coinversation.io",
                    "wss://node0.competitors.club:433/wss",
                    "blockchain.crownsterling.io",
                    "rpc.crust.network",
                    "rpc.darwinia.network",
                    "crab-rpc.darwinia.network",
                    "mainnet-node.dock.io",
                    "mainnet.edgewa.re",
                    "rpc.efinity.io",
                    "node.equilibrium.io",
                    "node.genshiro.io",
                    "rpc.hanonycash.com",
                    "archive.snakenet.hydradx.io",
                    "api.solo.integritee.io",
                    "wss://rpc.kulupu.corepaper.org:433/ws",
                    "ws.kusari.network",
                    "wss://mathchain-asia.maiziqianbao.net:433/ws",
                    "wss://rpc.neatcoin.org:433/ws",
                    "wss://mainnet.nftmart.io:433/rpc/ws",
                    "main3.nodleprotocol.io",
                    "rpc.plasmnet.io",
                    "mainnet.polkadex.trade",
                    "mainnet-rpc.polymesh.network",
                    "node.v1.riochain.io",
                    "kusama.rpc.robonomics.network",
                    "mainnet.sherpax.io",
                    "ws.alb.sora.org",
                    "wss.spannerprotocol.com",
                    "mainnet-rpc.stafi.io",
                    "mainnet.subgame.org",
                    "rpc.subsocial.network",
                    "ws.swapdex.network",
                    "wss://mainnet.uniarts.vip:9443",
                    "westlake.datahighway.com",
                ],
                vec![],
            ]
        }
        Env::SelfSovereignTest => {
            //Test
            vec![
                vec![
                    "ws.test.azero.dev",
                    "rpc-test.ajuna.network",
                    "fullnode.amber.centrifuge.io",
                    "arcadia1.nodleprotocol.io",
                    "gladios.aresprotocol.io",
                    "wss://contextfree.api.onfinality.io:443/public-ws",
                    "beresheet.edgewa.re",
                    "wss://asgard-rpc.liebi.com:443/ws",
                    "tewai-rpc.bit.country",
                    "api.clover.finance",
                    "rpc.coinversation.io",
                    "api.crust.network",
                    "knox-1.dock.io",
                    "trillian.dolphin.red",
                    "mogiway-01.dotmog.com",
                    "gesell.encointer.org",
                    "testnet.equilibrium.io",
                    "substrate-rpc.parity.io",
                    "test-ws.fantour.io",
                    "galital-rpc-testnet.starkleytech.com",
                    "wss://galois.maiziqianbao.net:443/ws",
                    "gamepower.io",
                    "testnet.geekcash.org",
                    "halongbay.polkafoundry.com",
                    "wss://api-testnet.interlay.io/parachain/",
                    "ws.jupiter-poa.patract.cn",
                    "wss://full-nodes.kilt.io:9944/",
                    "wss://peregrine.kilt.io/parachain-public-ws/",
                    "klugdossier.net",
                    "testnet.kylin-node.co.uk",
                    "mandala.polkawallet.io",
                    "ws.f1.testnet.manta.network",
                    "wss://moonbeam-alpha.api.onfinality.io:433/public-ws",
                    "wss://mybank.network/substrate",
                    "wss://neumann.api.onfinality.io:443/public-ws",
                    "staging-ws.nftmart.io",
                    "opal.unique.network",
                    "rpc.opportunity.standard.tech",
                    "parachain-rpc.origin-trail.network",
                    "pangolin-rpc.darwinia.network",
                    "pangoro-rpc.darwinia.network",
                    "wss://poc5.phala.network/ws",
                    "phoenix-ws.coinid.pro",
                    "blockchain.polkadex.trade",
                    "testnet-rpc.polymesh.live",
                    "wss://testnet.pontem.network:443/ws",
                    "rpc.realis.network",
                    "node.v1.staging.riochain.io",
                    "sherpax-testnet.chainx.org",
                    "rpc.shibuya.astar.network",
                    "parachain-rpc.snowbridge.network",
                    "ws.stage.sora2.soramitsu.co.jp",
                    "alpha.subdao.org",
                    "farm-rpc.subspace.network",
                    "test-rpc.subspace.network",
                    "testnet.ternoa.com",
                    "wss://testnet-node-1.laminar-chain.laminar.one:443/ws",
                    "testnet.uniarts.network",
                    "testnet2.unique.network",
                    "unitventures.io",
                    "wss://vodka.rpc.neatcoin.org:443/ws",
                    "testnet.web3games.org",
                    "test1.zcloak.network",
                    "bsr.zeitgeist.pm",
                    "alphaville.zero.io",
                    "testnet.imbue.network",
                ],
                vec![],
            ]
        }
        Env::Local => {
            vec![vec![
                "ws://127.0.0.1:9944",
                "ws://127.0.0.1:9966",
                "ws://127.0.0.1:9920",
            ]]
        }
        Env::NFTs => {
            // These are parachains known to be rocking the uniques pallet:
            vec![
                vec![
                    "rpc.polkadot.io",
                    "statemint-rpc.polkadot.io",
                    "ws.unique.network",
                    "rpc-01.hydradx.io",
                ],
                vec![
                    "kusama-rpc.polkadot.io",
                    "statemine-rpc.dwellir.com",
                    "us-ws-quartz.unique.network",
                    "basilisk-rpc.dwellir.com",
                ],
            ]
        } // TODO; networks with ink: astar, shiden, phala, peeq, aleph zero
    }
}
