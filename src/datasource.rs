use subxt::DefaultConfig;
use subxt::ClientBuilder;
use desub_current::{decoder, Metadata};
use subxt::sp_runtime::Deserialize;
use crate::ABlocks;
use subxt::DefaultExtra;use subxt::{RawEventDetails};
use super::polkadot;
use async_std::stream::StreamExt;
use subxt::rpc::Subscription;
use subxt::sp_runtime::generic::Header;
use subxt::sp_runtime::traits::BlakeTwo256;
use parity_scale_codec::Decode;
use frame_metadata::RuntimeMetadataPrefixed;
use parity_scale_codec::Encode;
use subxt::Config;
// #[derive(Clone, Debug, Default, Eq, PartialEq)]
// pub struct MyConfig;
// impl Config for MyConfig {
//     // This is different from the default `u32`.
//     //
//     // *Note* that in this example it does differ from the actual `Index` type in the
//     // polkadot runtime used, so some operations will fail. Normally when using a custom `Config`
//     // impl types MUST match exactly those used in the actual runtime.
//     type Index = u64;
//     type BlockNumber = <DefaultConfig as Config>::BlockNumber;
//     type Hash = <DefaultConfig as Config>::Hash;
//     type Hashing = <DefaultConfig as Config>::Hashing;
//     type AccountId = <DefaultConfig as Config>::AccountId;
//     type Address = <DefaultConfig as Config>::Address;
//     type Header = <DefaultConfig as Config>::Header;
//     type Signature = <DefaultConfig as Config>::Signature;
//     type Extrinsic = ExtrinsicVec;//<DefaultConfig as Config>::Extrinsic;
// }

// #[derive(PartialEq, Eq, Clone, Default, Encode, Decode, Debug, serde::Serialize, Deserialize)]
// pub struct ExtrinsicVec(pub Vec<u8>);

// impl subxt::sp_runtime::traits::Extrinsic for ExtrinsicVec {

// }

#[derive(Decode)]
pub struct ExtrinsicVec(pub Vec<u8>);


pub async fn watch_blocks(tx: ABlocks, url: String) -> Result<(), Box<dyn std::error::Error>> { 
    let metadata_bytes = std::fs::read(
        "/home/gilescope/git/bevy_webgl_template/polkadot_metadata.scale",
    )
    .unwrap();
    use core::slice::SlicePattern;
    use scale_info::form::PortableForm;
    let meta: RuntimeMetadataPrefixed =
        Decode::decode(&mut metadata_bytes.as_slice()).unwrap();
    //  match meta

    let metad = Metadata::from_bytes(&metadata_bytes).unwrap();





    let api = ClientBuilder::new()
    .set_url(url)
    .build()
    .await?
    .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();//  .to_runtime_api::<polkadot::RuntimeApi<MyConfig, DefaultExtra<MyConfig>>>();

    // For non-finalised blocks use `.subscribe_finalized_blocks()`
    let mut block_headers: Subscription<Header<u32, BlakeTwo256>> =
    api.client.rpc().subscribe_finalized_blocks().await.unwrap();

    while let Some(Ok(block_header)) = block_headers.next().await {
        let block_hash = block_header.hash();
    println!(
        "block number: {} hash:{} parent:{} state root:{} extrinsics root:{}",
        block_header.number, block_hash, block_header.parent_hash, block_header.state_root, block_header.extrinsics_root
    );
    if let Ok(Some(block_body)) = api.client.rpc().block(Some(block_hash)).await {
        let mut exts = vec![];
        for ext_bytes in block_body.block.extrinsics.iter()
        {
// let s : String = ext_bytes;
// ext_bytes.using_encoded(|ref slice| {
//     assert_eq!(slice, &b"\x0f");

            let ex_slice = <ExtrinsicVec as Decode>::decode(&mut ext_bytes.encode().as_slice()).unwrap().0;
            // This works too but unsafe:
            //let ex_slice2: Vec<u8> = unsafe { std::mem::transmute(ext_bytes.clone()) }; 
             
            // use parity_scale_codec::Encode;
            // ext_bytes.encode();
            let ext = decoder::decode_unwrapped_extrinsic(&metad, &mut ex_slice.as_slice()).expect("can decode extrinsic");
            
            exts.push(format!("{:#?}", ext));

          
            // print!("hohoohoohhohohohooh: {:#?}", ext);

            // let ext = decoder::decode_extrinsic(&meta, &mut ext_bytes.0.as_slice()).expect("can decode extrinsic");
        }      
        tx.lock().unwrap().push(PolkaBlock {
            blocknum: block_header.number as usize,
            blockhash: block_hash.to_string(),
            extrinsics: exts,
            events: vec![],
        });
        //TODO: assert_eq!(block_header.hash(), block.hash());
        println!("{block_body:?}");
    }
    }
    Ok(())
}

pub struct PolkaBlock {
    pub blocknum: usize,
    pub blockhash: String,
    pub extrinsics: Vec<String>,
    pub events: Vec<RawEventDetails>,
}

pub async fn block_chain(tx: ABlocks, url: String) -> Result<(), Box<dyn std::error::Error>> {
    let api = ClientBuilder::new()
        .set_url(&url)
        //    .set_url("ws://127.0.0.1:9944")
        //        .set_url("wss://kusama-rpc.polkadot.io:443")
        //wss://kusama-rpc.polkadot.io:443
        .build()
        .await?
        .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();

    let mut event_sub = api.events().subscribe_finalized().await?;

    // let mut ex_sub = api.tx().subscribe().await?;

    let mut blocknum = 1;
    while let Some(events) = event_sub.next().await {
        let events = events?;
        let block_hash = events.block_hash();
        blocknum += 1;

        tx.lock().unwrap().push(PolkaBlock {
            blocknum,
            blockhash: events.block_hash().to_string(),
            extrinsics: vec![],
            events: events.iter_raw().map(|c| c.unwrap()).collect::<Vec<_>>(),
        });

        // for event in events.iter_raw() {
        //     let event: RawEventDetails = event?;
        //     // match event.pallet.as_str() {
        //     //     "ImOnline" | "ParaInclusion" | "PhragmenElection" => {
        //     //         continue;
        //     //     }
        //     //     _ => {}
        //     // }

        //     // if event.pallet == "System" {
        //     //     if event.variant == "ExtrinsicSuccess" {
        //     //         continue;
        //     //     }
        //     // }

        //     let is_balance_transfer = event
        //         .as_event::<polkadot::balances::events::Transfer>()?
        //         .is_some();

        //     let is_online = event
        //         .as_event::<polkadot::im_online::events::AllGood>()?
        //         .is_some();

        //     let is_new_session = event
        //         .as_event::<polkadot::session::events::NewSession>()?
        //         .is_some();

        //     if !is_online && !is_new_session {
        //         tx.lock().unwrap().push(BlockEvent {
        //             blocknum,
        //             raw_event: event.clone(),
        //         });
        //         println!("    {:?}\n", event.pallet);
        //         println!("    {:#?}\n", event);

        //         // stdout()
        //         // .execute(SetForegroundColor(Color::Green)).unwrap()
        //         // .execute(SetBackgroundColor(Color::Black)).unwrap()
        //         // .execute(Print(format!("    {:?}\r\n", event))).unwrap()
        //         // .execute(ResetColor).unwrap();
        //     }
        // }
    }
    Ok(())
}