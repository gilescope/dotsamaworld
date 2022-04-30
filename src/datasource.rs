use super::polkadot;
use crate::ABlocks;
use crate::DataEntity;
use async_std::stream::StreamExt;
use desub_current::{decoder, Metadata};
use frame_metadata::RuntimeMetadataPrefixed;
use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry;
use std::hash::Hash;
use subxt::rpc::Subscription;
use subxt::sp_runtime::generic::Header;
use subxt::sp_runtime::traits::BlakeTwo256;
use subxt::sp_runtime::Deserialize;
use subxt::ClientBuilder;
use subxt::Config;
use subxt::DefaultConfig;
use subxt::DefaultExtra;
use subxt::RawEventDetails;

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
use std::path::Path;
use std::time::Duration;
use subxt::rpc::ClientT;
#[derive(Decode)]
pub struct ExtrinsicVec(pub Vec<u8>);

pub async fn watch_blocks(tx: ABlocks, url: String) -> Result<(), Box<dyn std::error::Error>> {
    use core::slice::SlicePattern;
    use scale_info::form::PortableForm;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::default();
    url.hash(&mut hasher);
    let hash = hasher.finish();

    // Save metadata to a file:
    // let out_dir = std::env::var_os("OUT_DIR").unwrap();

    let metadata_path = format!("{hash}.metadata.scale");

    // let meta: RuntimeMetadataPrefixed =
    // Decode::decode(&mut metadata_bytes.as_slice()).unwrap();
    //  match meta

    // Download metadata from binary; retry until successful, or a limit is hit.

    // let client = reqwest::Client::new();

    // // See https://www.jsonrpc.org/specification for more information on
    // // the JSON RPC 2.0 format that we use here to talk to nodes.
    // let res = client.post("http://localhost:9933")
    //     .json(&json!{{
    //         "id": 1,
    //         "jsonrpc": "2.0",
    //         "method": "rpc_methods"
    //     }})
    //     .send()
    //     .await
    //     .unwrap();

    let metadata_bytes = if let Ok(result) = std::fs::read(
        &metadata_path, //    "/home/gilescope/git/bevy_webgl_template/polkadot_metadata.scale"
    ) {
        result
    } else {
        let metadata_bytes: sp_core::Bytes = {
            const MAX_RETRIES: usize = 6;
            let mut retries = 0;

            loop {
                if retries >= MAX_RETRIES {
                    panic!("Cannot connect to substrate node after {} retries", retries);
                }

                println!("trying to get metadata ttnr {url}");
                // It might take a while for substrate node that spin up the RPC server.
                // Thus, the connection might get rejected a few times.
                let res = match subxt::rpc::ws_client(&url).await {
                    Ok(c) => c.request("state_getMetadata", None).await,
                    Err(e) => Err(e),
                };
                println!("finished trying {url} res {res:?}");
                match res {
                    Ok(res) => {
                        // let _ = cmd.kill();
                        break res;
                    }
                    _ => {
                        std::thread::sleep(std::time::Duration::from_secs(1 << retries));
                        retries += 1;
                    }
                };
            }
        };

        println!("writing to {:?}", metadata_path);
        std::fs::write(&metadata_path, &metadata_bytes.0).expect("Couldn't write metadata output");
        std::fs::read(
            metadata_path, //    "/home/gilescope/git/bevy_webgl_template/polkadot_metadata.scale"
        )
        .unwrap()
    };

    let metad = Metadata::from_bytes(&metadata_bytes).unwrap();

    // give back the "result"s to save some lines of code..).
    // let res = rpc_to_localhost("state_getMetadata", ()).await.unwrap();

    // Decode the hex value into bytes (which are the SCALE encoded metadata details):
    // let metadata_hex = res.as_str().unwrap();
    // let metadata_bytes = hex::decode(&metadata_hex.trim_start_matches("0x")).unwrap();

    let api = ClientBuilder::new()
        .set_url(url)
        .build()
        .await?
        .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>(); //  .to_runtime_api::<polkadot::RuntimeApi<MyConfig, DefaultExtra<MyConfig>>>();

    // let metad: subxt::Metadata = api.client.rpc().metadata().await.unwrap();
    // // let metabytes = metad.encode();

    // std::process::exit(-1);
    // let bytes: Bytes = api.client.rpc()
    //         //.client
    //         .request("state_getMetadata", rpc_params![])
    //         .await?;

    // For non-finalised blocks use `.subscribe_finalized_blocks()`
    let mut block_headers: Subscription<Header<u32, BlakeTwo256>> =
        api.client.rpc().subscribe_finalized_blocks().await.unwrap();

    while let Some(Ok(block_header)) = block_headers.next().await {
        let block_hash = block_header.hash();
        // println!(
        //     "block number: {} hash:{} parent:{} state root:{} extrinsics root:{}",
        //     block_header.number,
        //     block_hash,
        //     block_header.parent_hash,
        //     block_header.state_root,
        //     block_header.extrinsics_root
        // );
        if let Ok(Some(block_body)) = api.client.rpc().block(Some(block_hash)).await {
            let mut exts = vec![];
            // println!("block hash! {}", block_hash.to_string());
            for (i, ext_bytes) in block_body.block.extrinsics.iter().enumerate() {
                // let s : String = ext_bytes;
                // ext_bytes.using_encoded(|ref slice| {
                //     assert_eq!(slice, &b"\x0f");

                let ex_slice = <ExtrinsicVec as Decode>::decode(&mut ext_bytes.encode().as_slice())
                    .unwrap()
                    .0;
                // This works too but unsafe:
                //let ex_slice2: Vec<u8> = unsafe { std::mem::transmute(ext_bytes.clone()) };

                // use parity_scale_codec::Encode;
                // ext_bytes.encode();
                if let Ok(ext) =
                    decoder::decode_unwrapped_extrinsic(&metad, &mut ex_slice.as_slice())
                {
                    let pallet = ext.call_data.pallet_name.to_string();
                    let variant = ext.call_data.ty.name().to_owned();
                    let args = ext
                        .call_data
                        .arguments
                        .iter()
                        .map(|arg| format!("{:?}", arg).chars().take(500).collect::<String>())
                        .collect();

                    exts.push(DataEntity::Extrinsic {
                        id: (block_header.number, i as u32),
                        pallet,
                        variant,
                        args,
                    });
                }
                // print!("hohoohoohhohohohooh: {:#?}", ext);

                // let ext = decoder::decode_extrinsic(&meta, &mut ext_bytes.0.as_slice()).expect("can decode extrinsic");
            }
            let ext_clone = exts.clone();
            let mut handle = tx.lock().unwrap();
            let current = handle
                .0
                .entry(block_hash.to_string())
                .or_insert(PolkaBlock {
                    blocknum: block_header.number as usize,
                    blockhash: block_hash.to_string(),
                    extrinsics: exts,
                    events: vec![],
                });
            // let mut remove = false;
            if !current.events.is_empty() {
                // println!("already one there for hash!!!! {}", block_hash.to_string());
                let mut current = handle.0.remove(&block_hash.to_string()).unwrap();
                // let val = Entry::Vacant(());
                // std::mem::swap(entry, val);
                // if let Entry::Occupied(
                current.extrinsics = ext_clone;
                handle.1.push(current);
                //  println!("pushed finished on!!!! {}", block_hash.to_string());

                // remove = true;
            }

            // if remove {

            // }

            // match entry {
            //     Entry::Vacant(())
            // push();
            //TODO: assert_eq!(block_header.hash(), block.hash());
            // println!("{block_body:?}");
        }
    }
    Ok(())
}

pub struct PolkaBlock {
    pub blocknum: usize,
    pub blockhash: String,
    pub extrinsics: Vec<DataEntity>,
    pub events: Vec<RawEventDetails>,
}

pub async fn watch_events(tx: ABlocks, url: String) -> Result<(), Box<dyn std::error::Error>> {
    let api = ClientBuilder::new()
        .set_url(&url)
        .build()
        .await?
        .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();

    let mut event_sub = api.events().subscribe_finalized().await?;

    let mut blocknum = 1;
    while let Some(events) = event_sub.next().await {
        let events = events?;
        let blockhash = events.block_hash().to_string();
        blocknum += 1;

        tx.lock().unwrap().0.insert(
            events.block_hash().to_string(),
            PolkaBlock {
                blocknum,
                blockhash,
                extrinsics: vec![],
                events: events.iter_raw().map(|c| c.unwrap()).collect::<Vec<_>>(),
            },
        );
    }
    Ok(())
}

pub fn associate_events(
    ext: Vec<DataEntity>,
    mut events: Vec<RawEventDetails>,
) -> Vec<(Option<DataEntity>, Vec<RawEventDetails>)> {
    let mut ext: Vec<(Option<DataEntity>, Vec<RawEventDetails>)> = ext
        .into_iter()
        .map(|extrinsic| {
            let eid = if let DataEntity::Extrinsic {
                id: (_bid, eid), ..
            } = extrinsic
            {
                eid
            } else {
                panic!("bad stuff happened");
            };
            // println!("{} count ", events.len());
            (
                Some(extrinsic),
                events
                    .drain_filter(|raw| match &raw.phase {
                        subxt::Phase::ApplyExtrinsic(extrinsic_id) => *extrinsic_id == eid,
                        _ => false,
                    })
                    .collect(),
            )
        })
        .collect();

    for unrelated_to_extrinsics in events {
        ext.push((None, vec![unrelated_to_extrinsics]));
    }

    ext
    //leftovers in events should be utils..
}
