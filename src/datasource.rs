use super::polkadot;
use crate::ABlocks;
use crate::DataEntity;
use async_std::stream::StreamExt;
use desub_current::{decoder, Metadata};
// use frame_metadata::RuntimeMetadataPrefixed;
use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
use std::collections::hash_map::DefaultHasher;
// use std::collections::hash_map::Entry;
use std::hash::Hash;
use subxt::rpc::Subscription;
use subxt::sp_runtime::generic::Header;
use subxt::sp_runtime::traits::BlakeTwo256;
// use subxt::sp_runtime::Deserialize;
use subxt::ClientBuilder;
// use subxt::Config;
use desub_current::value::*;
use desub_current::ValueDef;
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
// use std::path::Path;
// use std::time::Duration;
use subxt::rpc::ClientT;
#[derive(Decode)]
pub struct ExtrinsicVec(pub Vec<u8>);

fn print_val<T>(dbg: &desub_current::ValueDef<T>) {
    match dbg {
        desub_current::ValueDef::BitSequence(..) => {
            println!("bit sequence");
        }
        desub_current::ValueDef::Composite(..) => {
            println!("composit");
        }
        desub_current::ValueDef::Primitive(..) => {
            println!("primitiv");
        }
        desub_current::ValueDef::Variant(..) => {
            println!("variatt");
        }
    }
}

pub async fn watch_blocks(tx: ABlocks, url: String) -> Result<(), Box<dyn std::error::Error>> {
    // use core::slice::SlicePattern;
    // use scale_info::form::PortableForm;
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

    let client = ClientBuilder::new().set_url(url).build().await?;

    let api =
        client.to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();

    let parachain_name = api.client.rpc().system_chain().await?;
    // println!("system chain: {}", parachain_name);

    tx.lock().unwrap().2.chain_name = parachain_name.clone();

    //     ""), None).await?;

    // Fetch the metadata
    // let bytes: subxt::Bytes = self
    //     .client
    //     .request("parachainInfo_getChainId", rpc_params!["0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f"])
    //     .await?;
    // let meta: RuntimeMetadataPrefixed = Decode::decode(&mut &bytes[..])?;
    // let metadata: Metadata = meta.try_into()?;

    // let store_loc = "0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";
    // let fromto =hex::decode(&store_loc.trim_start_matches("0x")).unwrap();
    // let fromto = sp_core::H256::from_slice(fromto.as_slice());
    // let res = api.client.rpc().query_storage(vec![], fromto, None).await?;
    // println!("{res:?}");

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

                    let mut args: Vec<_> = ext
                        .call_data
                        .arguments
                        .iter()
                        .map(|arg| format!("{:?}", arg).chars().take(500).collect::<String>())
                        .collect();

                    if pallet == "System" && variant == "remark" {
                        match &ext.call_data.arguments[0].value {
                            desub_current::ValueDef::Composite(
                                desub_current::value::Composite::Unnamed(chars_vals),
                            ) => {
                                let bytes = chars_vals
                                    .iter()
                                    .map(|arg| match arg.value {
                                        desub_current::ValueDef::Primitive(
                                            desub_current::value::Primitive::U8(ch),
                                        ) => ch,
                                        _ => b'!',
                                    })
                                    .collect::<Vec<u8>>();
                                let rmrk = String::from_utf8_lossy(bytes.as_slice()).to_string();
                                args.insert(0, rmrk);
                            }

                            _ => {}
                        }
                    }

                    let mut children = vec![];
                    // println!("checking batch");
                    // Anything that looks batch like we will assume is a batch
                    if variant.contains("batch") {
                        for arg in ext.call_data.arguments {
                            //just first arg
                            match arg.value {
                                ValueDef::Composite(Composite::Unnamed(chars_vals)) => {
                                    for v in chars_vals {
                                        match v.value {
                                            ValueDef::Variant(Variant {
                                                ref name,
                                                values: Composite::Unnamed(chars_vals),
                                            }) => {
                                                println!("{parachain_name} varient pallet {name}");
                                                let inner_pallet = name;

                                                for v in chars_vals {
                                                    match v.value {
                                                        ValueDef::Variant(Variant {
                                                            name,
                                                            values,
                                                        }) => {
                                                            println!("{pallet} {variant} has inside a {inner_pallet} {name}");
                                                            children.push(DataEntity::Extrinsic {
                                                                id: (block_header.number, i as u32),
                                                                pallet: inner_pallet.to_string(),
                                                                variant: name.clone(),
                                                                args: vec![format!("{:?}", values)],
                                                                contains: vec![],
                                                            });
                                                        }
                                                        _ => {
                                                            println!("miss yet close");
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {
                                                // println!("inner miss");
                                                // print_val(&v.value);
                                            }
                                        }
                                    }
                                }

                                _ => {
                                    // println!("miss");
                                }
                            }
                        }
                    }
                    exts.push(DataEntity::Extrinsic {
                        id: (block_header.number, i as u32),
                        pallet,
                        variant,
                        args,
                        contains: children,
                    });
                }
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
            if !current.events.is_empty() {
                let mut current = handle.0.remove(&block_hash.to_string()).unwrap();
                current.extrinsics = ext_clone;
                handle.1.push(current);
            }
            //TODO: assert_eq!(block_header.hash(), block.hash());
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
