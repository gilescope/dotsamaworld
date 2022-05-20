use super::polkadot;
use crate::details::Success;
use crate::polkadot::runtime_types::xcm::VersionedXcm;
use crate::ABlocks;
use crate::DataEntity;
use crate::DataEvent;
use crate::Details;
use async_std::stream::StreamExt;
use async_std::sync::RwLock;
use bevy::prelude::warn;
use desub_current::value::*;
use desub_current::ValueDef;
use desub_current::{decoder, Metadata};
use lazy_static::lazy_static;
use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
use sp_core::H256;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::Hash;
use std::num::NonZeroU32;
use subxt::rpc::ClientT;
use subxt::ClientBuilder;
use subxt::DefaultConfig;
use subxt::DefaultExtra;
// use subxt::EventDetails;
// use subxt::RawEventDetails;

#[derive(Decode, Debug)]
pub struct ExtrinsicVec(pub Vec<u8>);

#[allow(dead_code)]
fn print_val<T>(dbg: &desub_current::ValueDef<T>) {
    match dbg {
        desub_current::ValueDef::BitSequence(..) => {
            println!("bit sequence");
        }
        desub_current::ValueDef::Composite(inner) => match inner {
            Composite::Named(fields) => {
                println!("named composit (");
                for (n, f) in fields {
                    print!("{n}");
                    print_val(&f.value);
                }
                println!(")");
            }
            Composite::Unnamed(fields) => {
                println!("un named composita(");

                if fields
                    .iter()
                    .all(|f| matches!(f.value, ValueDef::Primitive(_)))
                    && fields.len() > 1
                {
                    println!(" << primitive array >> ");
                } else {
                    for f in fields.iter() {
                        print_val(&f.value);
                    }
                }
                println!(")");
            }
        },
        desub_current::ValueDef::Primitive(..) => {
            println!("primitiv");
        }
        desub_current::ValueDef::Variant(Variant { name, values }) => {
            println!("variatt {name} (");
            match values {
                Composite::Named(fields) => {
                    println!("named composit (");
                    for (n, f) in fields {
                        print!("{n}");
                        print_val(&f.value);
                    }
                    println!(")");
                }
                Composite::Unnamed(fields) => {
                    println!("un named composita(");
                    for f in fields.iter() {
                        print_val(&f.value);
                    }
                    println!(")");
                }
            }
            println!("variatt end {name} )");
        }
    }
}

use std::collections::HashMap;

// THere's better ways but crazy levels of matching...
fn flattern<T>(
    dbg: &desub_current::ValueDef<T>,
    location: &str,
    mut results: &mut HashMap<String, String>,
) {
    match dbg {
        desub_current::ValueDef::BitSequence(..) => {
            // println!("bitseq skipped");
        }
        desub_current::ValueDef::Composite(inner) => match inner {
            Composite::Named(fields) => {
                for (n, f) in fields {
                    flattern(&f.value, &format!("{}.{}", location, n), &mut results);
                }
            }
            Composite::Unnamed(fields) => {
                if fields
                    .iter()
                    .all(|f| matches!(f.value, ValueDef::Primitive(Primitive::U8(_))))
                    && fields.len() > 1
                {
                    results.insert(
                        format!("{}", location),
                        hex::encode(
                            fields
                                .iter()
                                .map(|f| {
                                    if let ValueDef::Primitive(Primitive::U8(byte)) = f.value {
                                        byte
                                    } else {
                                        panic!();
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ),
                    );
                } else {
                    for (n, f) in fields.iter().enumerate() {
                        flattern(&f.value, &format!("{}.{}", location, n), &mut results);
                    }
                }
            }
        },
        desub_current::ValueDef::Primitive(Primitive::U8(val)) => {
            results.insert(location.to_string(), val.to_string());
        }
        desub_current::ValueDef::Primitive(Primitive::U32(val)) => {
            results.insert(location.to_string(), val.to_string());
        }
        desub_current::ValueDef::Primitive(..) => {
            // println!("primitiv skipped");
        }
        desub_current::ValueDef::Variant(Variant { name, values }) => match values {
            Composite::Named(fields) => {
                if fields
                    .iter()
                    .all(|(_name, f)| matches!(f.value, ValueDef::Primitive(Primitive::U8(_))))
                    && fields.len() > 1
                {
                    results.insert(
                        format!("{},{}", name, location),
                        hex::encode(
                            fields
                                .iter()
                                .map(|(_, f)| {
                                    if let ValueDef::Primitive(Primitive::U8(byte)) = f.value {
                                        byte
                                    } else {
                                        panic!();
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ),
                    );
                } else {
                    for (n, f) in fields {
                        flattern(
                            &f.value,
                            &format!("{}.{}.{}", location, name, n),
                            &mut results,
                        );
                    }
                }
            }
            Composite::Unnamed(fields) => {
                if fields
                    .iter()
                    .all(|f| matches!(f.value, ValueDef::Primitive(Primitive::U8(_))))
                    && fields.len() > 1
                {
                    results.insert(
                        location.to_string(),
                        hex::encode(
                            fields
                                .iter()
                                .map(|f| {
                                    if let ValueDef::Primitive(Primitive::U8(byte)) = f.value {
                                        byte
                                    } else {
                                        panic!();
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ),
                    );
                } else {
                    for (n, f) in fields.iter().enumerate() {
                        flattern(
                            &f.value,
                            &format!("{}.{}.{}", location, name, n),
                            &mut results,
                        );
                    }
                }
            }
        },
    }
}

// fn iter(composite: &desub_current::ValueDef<T>) {
//     match composite.value {
//         ValueDef::Composite(Composite::Named(named)) =>  {

//         }
//         ValueDef::Composite(Composite::Unnamed(unnamed))  => {

//         }
//         ValueDef::Variant(_) => todo!(),
//         ValueDef::BitSequence(_) => { }, //not supported
//         ValueDef::Primitive(_) => {
//             //skipping
//         },

//     }
// }
lazy_static! {
    static ref PARA_ID_TO_NAME: RwLock<HashMap<(String, NonZeroU32), String>> =
        RwLock::new(HashMap::new());
}

fn please_hash<T: Hash>(val: T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::default();
    val.hash(&mut hasher);
    hasher.finish()
}

async fn get_desub_metadata(url: &str) -> Metadata {
    let hash = please_hash(url);

    let metadata_path = format!("target/{hash}.metadata.scale");

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

                println!("trying to get metadata from {url}");
                // It might take a while for substrate node that spin up the RPC server.
                // Thus, the connection might get rejected a few times.
                let res = match subxt::rpc::ws_client(&url).await {
                    Ok(c) => c.request("state_getMetadata", None).await,
                    Err(e) => Err(e),
                };
                println!("finished trying {url}");
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

    Metadata::from_bytes(&metadata_bytes).unwrap()
}

pub async fn watch_blocks(
    tx: ABlocks,
    url: String,
    relay_id: String,
    as_of: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // use core::slice::SlicePattern;
    // use scale_info::form::PortableForm;
    // use std::hash::Hasher;
    // let mut hasher = DefaultHasher::default();
    // url.hash(&mut hasher);
    let metad = get_desub_metadata(&url).await;

    let mut client = ClientBuilder::new().set_url(&url).build().await?;

    // parachainInfo / parachainId returns u32 paraId
    let storage_key =
        hex::decode("0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f").unwrap();
    let call = client
        .storage()
        .fetch_raw(sp_core::storage::StorageKey(storage_key), None)
        .await?;

    let para_id = if let Some(sp_core::storage::StorageData(val)) = call {
        let para_id = <u32 as Decode>::decode(&mut val.as_slice()).unwrap();
        println!("{} is para id {}", &url, para_id);

        Some(NonZeroU32::try_from(para_id).expect("para id should not be 0"))
    } else {
        // This is expected for relay chains...
        warn!("could not find para id for {}", &url);
        None
    };

    let mut api =
        client.to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
    let parachain_name = api.client.rpc().system_chain().await?;

    {
        let mut parachain_info = tx.lock().unwrap();
        parachain_info.2.chain_name = parachain_name.clone();
        parachain_info.2.chain_ws = url.clone();
        parachain_info.2.chain_id = para_id
    }

    if let Some(para_id) = para_id {
        PARA_ID_TO_NAME
            .write()
            .await
            .insert((relay_id.clone(), para_id), parachain_name.clone());
    }
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
    let mut reconnects = 0;
    while reconnects < 20 {
        if let Ok(mut block_headers)//: Subscription<Header<u32, BlakeTwo256>> 
        =
            api.client.rpc().subscribe_finalized_blocks().await {

            while let Some(Ok(block_header)) = block_headers.next().await {
                let block_num = block_header.number;
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

                        let encoded_extrinsic = ext_bytes.encode();
                        let ex_slice = <ExtrinsicVec as Decode>::decode(&mut encoded_extrinsic.as_slice())
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
                            let mut start_link = vec![];
                            let mut end_link = vec![];

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

                            // Maybe we can rely on the common type system for XCM versions as it has to be quite standard...

                            let mut children = vec![];
                            // println!("checking batch");

                            /*
                            Value { variant v1
                                value: V1 ( <--unnamed composit
                                    Value { value:  { <-- named composit
                                        parents: Value { value: U8(0), context: TypeId(2) }, 
                                        interior: Value { value: X1 (Value { value:    <-- variant x1, un named composit, 
                                            Parachain (Value {      <-- variant parachain
                                                value: U32(2012),    <-- unnamed composit
                                                context: TypeId(114) },),  
                            context: TypeId(113) },), context: TypeId(112) } }, context: TypeId(111) },), context: TypeId(144) }
                            
                            */

                            // Seek out and expand Ump / UpwardMessageRecieved;
                            // if pallet == "ParaInherent" && variant == "enter" {
                            //     let mut results = HashMap::new();
                            //     flattern(&ext.call_data.arguments[0].value, "",&mut results);
                            //     let _ = results.drain_filter(|el, _| el.starts_with(".bitfields"));
                            //     let _ = results.drain_filter(|el, _| el.starts_with(".backed_candidates"));
                            //     let _ = results.drain_filter(|el, _| el.starts_with(".parent_"));

                            //     println!("FLATTERN UMP {:#?}", results);
                            // }
                            // Seek out and expand Dmp / DownwardMessageRecieved;
                            if pallet == "ParachainSystem" && variant == "set_validation_data" {
                                match &ext.call_data.arguments[0].value
                                {
                                    ValueDef::Composite(Composite::Named(named)) =>
                                    {
                                        for (name, val) in named {
                                            match name.as_str() {
                                                "upward_messages" => {
                                                    println!("found upward msgs (first time)");
                                                    print_val(&val.value);
                                                },
                                                "horizontal_messages" => {
                                                    if let ValueDef::Composite(Composite::Unnamed(vals)) = &val.value {
                                                        for val in vals {
                                                            // channels
                                                            if let ValueDef::Composite(Composite::Unnamed(vals)) = &val.value {
                                                                for val in vals {
                                                                    // single channel
                                                                    if let ValueDef::Composite(Composite::Unnamed(vals)) = &val.value {
                                                                        for val in vals {
                                                                            // Should be a msg
                                                                          //  for val in vals {
                                                                                //msgs
                                                                            if let ValueDef::Composite(Composite::Unnamed(vals)) = &val.value {
                                                                                if vals.len() > 0 {
                                                                                    for m in vals {
                                                                                        if let ValueDef::Primitive(Primitive::U32(_from_para_id))= &m.value {
                                                                                            // println!("from {}", from_para_id);
                                                                                        }

                                                                                        if vals.len() > 1 {
                                                                                            let mut results = HashMap::new();
                                                                                            flattern(&val.value, "",&mut results);
                                                                                            println!("INNER {:#?}", results);
                                                                                            //Could be that these are not yet in the wild 
                                                                                            std::process::exit(1);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                },
                                                "downward_messages" => {
                                                    if let ValueDef::Composite(Composite::Unnamed(vals)) = &val.value {
                                                        for val in vals {
                                                            let mut results = HashMap::new();
                                                            flattern(&val.value, "",&mut results);
                                                            println!("FLATTERN {:#?}", results);
                                                            // also .sent_at
                                                            if let Some(msg) = results.get(".msg") {
                                                                if let Some(sent_at) = results.get(".sent_at") {
                                                                    let bytes = hex::decode(msg).unwrap();
                                                                    if let Ok(ver_msg) = <VersionedXcm as Decode>::decode(&mut bytes.as_slice()) {
                                                                        match ver_msg {
                                                                            VersionedXcm::V0(msg) => {
                                                                                // Only one xcm instruction in a v1 message. 
                                                                                    let instruction = format!("{:?}", &msg);
                                                                                    println!("instruction {:?}", &instruction);
                                                                                    children.push(DataEntity::Extrinsic {
                                                                                        id: (block_header.number, i as u32),
                                                                                        args: vec![instruction.clone()],
                                                                                        contains: vec![],
                                                                                        raw: vec![], //TODO: should be simples
                                                                                        start_link: vec![],
                                                                                        end_link: vec![],
                                                                                        details: Details{  pallet: "Instruction".to_string(),
                                                                                        variant: instruction.split_once(' ').unwrap().0.to_string(), ..Details::default() }
                                                                                    });
                                                                                let inst = msg;
                                                                                use crate::polkadot::runtime_types::xcm::v0::Xcm::TransferReserveAsset;
                                                                                use crate::polkadot::runtime_types::xcm::v0::multi_location::MultiLocation;
                                                                                use crate::polkadot::runtime_types::xcm::v0::junction::Junction;
                                                                                if let TransferReserveAsset{dest, ..} = inst {
                                                                                    if let MultiLocation::X1(x1) = &dest {
                                                                                    //todo assert parent
                                                                                        if let Junction::AccountId32 {id, ..} = x1 {
                                                                                            let msg_id = format!("{}-{}", sent_at, please_hash(hex::encode(id)));
                                                                                            println!("RECIEVE HASH v0 {}", msg_id);
                                                                                            end_link.push(msg_id.clone());
                                                                                            start_link.push(msg_id); // for reserve assets received.
                                                                                        };
                                                                                    } else { panic!("unknonwn") }
                                                                                }
                                                                            }
                                                                            VersionedXcm::V1(msg) => {
                                                                                // Only one xcm instruction in a v1 message. 
                                                                                    let instruction = format!("{:?}", &msg);
                                                                                    println!("instruction {:?}", &instruction);
                                                                                    children.push(DataEntity::Extrinsic {
                                                                                        id: (block_header.number, i as u32),
                                                                                        args: vec![instruction.clone()],
                                                                                        contains: vec![],
                                                                                        raw: vec![], //TODO: should be simples
                                                                                        start_link: vec![],
                                                                                        end_link: vec![],
                                                                                        details: Details{  pallet: "Instruction".to_string(),
                                                                                        variant: instruction.split_once(' ').unwrap().0.to_string(), ..Details::default() }
                                                                                    });
                                                                                let inst = msg;
                                                                                use crate::polkadot::runtime_types::xcm::v1::Xcm::TransferReserveAsset;
                                                                                use crate::polkadot::runtime_types::xcm::v1::multilocation::MultiLocation;
                                                                                use crate::polkadot::runtime_types::xcm::v1::multilocation::Junctions;
                                                                                use crate::polkadot::runtime_types::xcm::v1::junction::Junction;
                                                                                if let TransferReserveAsset{dest, ..} = inst {
                                                                                    let MultiLocation{ interior, .. } = &dest;
                                                                                    //todo assert parent
                                                                                    if let Junctions::X1(x1) = interior {
                                                                                        if let Junction::AccountId32 {id, ..} = x1 {
                                                                                            let msg_id = format!("{}-{}", sent_at, please_hash(hex::encode(id)));
                                                                                            println!("RECIEVE HASH v1 {}", msg_id);
                                                                                            end_link.push(msg_id.clone());
                                                                                            start_link.push(msg_id); // for reserve assets received.
                                                                                        };
                                                                                    } else { panic!("unknonwn") }                                                                                }
                                                                            }
                                                                            VersionedXcm::V2(msg) => {
                                                                                for instruction in &msg.0 {
                                                                                    let instruction = format!("{:?}", instruction);
                                                                                    println!("instruction {:?}", &instruction);
                                                                                    children.push(DataEntity::Extrinsic {
                                                                                        id: (block_header.number, i as u32),
                                                                                        args: vec![instruction.clone()],
                                                                                        contains: vec![],
                                                                                        raw: vec![], //TODO: should be simples
                                                                                        start_link: vec![],
                                                                                        end_link: vec![],
                                                                                        details: Details
                                                                                        {
                                                                                            pallet: "Instruction".to_string(),
                                                                                            variant: instruction.split_once(' ').unwrap_or((&instruction,"")).0.to_string(), 
                                                                                            ..Details::default()
                                                                                        }
                                                                                    });
                                                                                }
                                                                                for inst in msg.0 {
                                                                                    //TODO: should only be importing from one version probably.
                                                                                    use crate::polkadot::runtime_types::xcm::v2::Instruction::DepositAsset;
                                                                                    use crate::polkadot::runtime_types::xcm::v1::multilocation::MultiLocation;
                                                                                    use crate::polkadot::runtime_types::xcm::v1::multilocation::Junctions;
                                                                                    use crate::polkadot::runtime_types::xcm::v1::junction::Junction;
                                                                                    if let DepositAsset{beneficiary, ..} = inst {
                                                                                        let MultiLocation{ interior, .. } = &beneficiary;
                                                                                        //todo assert parent
                                                                                        if let Junctions::X1(x1) = interior {
                                                                                            if let Junction::AccountId32 {id, ..} = x1 {
                                                                                                let msg_id = format!("{}-{}", sent_at, please_hash(hex::encode(id)));
                                                                                                println!("RECIEVE HASH v2 {}", msg_id);
                                                                                                end_link.push(msg_id.clone());
                                                                                                start_link.push(msg_id); // for reserve assets received.
                                                                                            };
                                                                                        } else { panic!("unknonwn") }                                                                                }
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        println!("could not decode msg: {}", msg);
                                                                    }
                                                                }
                                                                // println!("{:#?}", event);
                                                            }
                                                        }
                                                    }
                                                }
                                                _  => {}
                                            }
                                        }
                                    },
                                    _  => {}
                                }
                            }

                            // Anything that looks batch like we will assume is a batch
                            if pallet == "XcmPallet" && variant == "reserve_transfer_assets" {
                                let mut results = HashMap::new();
                                flattern(&ext.call_data.arguments[0].value, "",&mut results);
                                println!("FLATTERN {:#?}", results);

                                if let Some(dest) =  results.get(".V2.0.interior.X1.0.Parachain.0") { 
                                    println!("first time!");                                   
                                    //TODO; something with parent for cross relay chain maybe.(results.get(".V1.0.parents"),
                                    let dest: NonZeroU32 = dest.parse().unwrap();
                                    let name = if let Some(name) = PARA_ID_TO_NAME.read().await.get(&(relay_id.clone(), dest)) { name.clone() } else {  "unknown".to_string() };
                                     let mut results = HashMap::new();
                                    flattern(&ext.call_data.arguments[1].value, "",&mut results);
                                    let to = results.get(".V2.0.interior.X1.0.AccountId32.id");

                                    if let Some(to) = to {
                                        let msg_id = format!("{}-{}", block_num, please_hash(to));
                                        println!("SEND MSG v2 hash {}", msg_id);
                                        start_link.push(msg_id);
                                    }
                                    println!("Reserve_transfer_assets from {:?} to {} ({})", para_id, dest, name);

                                    // if ext.call_data.arguments.len() > 1 {
                                    //     let mut results = HashMap::new();
                                    //     flattern(&ext.call_data.arguments[1].value, "",&mut results);
                                    //     println!("FLATTERN DEST2 {:#?}", results);
                                    //     println!("ARGS {:?}", ext.call_data.arguments);
                                    // } else { 
                                    //     warn!("expected more params...");
                                    // }
                                }
                                if let Some(dest) =  results.get(".V1.0.interior.X1.0.Parachain.0") {                                    
                                    //TODO; something with parent for cross relay chain maybe.(results.get(".V1.0.parents"),
                                    let dest: NonZeroU32 = dest.parse().unwrap();
                                    let name = if let Some(name) = PARA_ID_TO_NAME.read().await.get(&(relay_id.clone(), dest)) { name.clone() } else {  "unknown".to_string() };
                                     let mut results = HashMap::new();
                                    flattern(&ext.call_data.arguments[1].value, "",&mut results);
                                    let to = results.get(".V1.0.interior.X1.0.AccountId32.id");

                                    if let Some(to) = to {
                                        let msg_id = format!("{}-{}", block_num, please_hash(to));
                                        println!("SEND MSG v1 hash {}", msg_id);
                                        start_link.push(msg_id);
                                    }
                                    println!("Reserve_transfer_assets from {:?} to {} ({})", para_id, dest, name);

                                    // if ext.call_data.arguments.len() > 1 {
                                    //     let mut results = HashMap::new();
                                    //     flattern(&ext.call_data.arguments[1].value, "",&mut results);
                                    //     println!("FLATTERN DEST2 {:#?}", results);
                                    //     println!("ARGS {:?}", ext.call_data.arguments);
                                    // } else { 
                                    //     warn!("expected more params...");
                                    // }
                                }
                                if let Some(dest) =  results.get(".V0.0.X1.0.Parachain.0") {    
                                    //TODO; something with parent for cross relay chain maybe.(results.get(".V1.0.parents"),
                                    let dest: NonZeroU32 = dest.parse().unwrap();
                                    let name = if let Some(name) = PARA_ID_TO_NAME.read().await.get(&(relay_id.clone(), dest)) { name.clone() } else {  "unknown".to_string() };
                                     let mut results = HashMap::new();
                                    flattern(&ext.call_data.arguments[1].value, "",&mut results);
                                    let to = results.get(".V0.0.X1.0.AccountId32.id");

                                    if let Some(to) = to {
                                        let msg_id = format!("{}-{}", block_num, please_hash(to));
                                        println!("SEND MSG v0 hash {}", msg_id);
                                        start_link.push(msg_id);
                                    }
                                    println!("Reserve_transfer_assets from {:?} to {} ({})", para_id, dest, name);

                                    // if ext.call_data.arguments.len() > 1 {
                                    //     let mut results = HashMap::new();
                                    //     flattern(&ext.call_data.arguments[1].value, "",&mut results);
                                    //     println!("FLATTERN DEST2 {:#?}", results);
                                    //     println!("ARGS {:?}", ext.call_data.arguments);
                                    // } else { 
                                    //     warn!("expected more params...");
                                    // }
                                }

//                                 print_val(&ext.call_data.arguments[0].value);
//                                 println!("Got here!!!!!");
//                                 println!("{:#?}", &ext.call_data.arguments[0].value);


//                                 let v = &ext.call_data.arguments[0];
//                                 match &v.value {
//                                     // ValueDef::Composite(Composite::Unnamed(chars_vals)) => {
//                                     //     for v in chars_vals {
//                                     //         match &v.value {
//                                     //             ValueDef::Variant(Variant {
//                                     //                 ref name,//V1
//                                     //                 values: Composite::Unnamed(chars_vals),
//                                     //             }) => {
//                                     //                 for v in chars_vals {
//                                     //                     match &v.value {
//                                     //                         ValueDef::Variant(Variant {
//                                     //                             name,
//                                     //                             values,
//                                     //                         }) => {
//                                     //                             println!("{pallet} {variant} has it {name}");
//                                     //                         }
//                                     //                         _ => {
//                                     //                             println!("miss3");
//                                     //                         }
//                                     //                     }
//                                     //                 }

//                                     //             }
//                                     //             _ => {
//                                     //                  println!("inner miss");
//                                     //                  print_val(&v.value);
//                                     //             }
//                                     //         }
//                                     //     }
//                                     // }, 
//                                     ValueDef::Variant(Variant{ name, values:Composite::Unnamed(values)})  => {
//                                         // println!("{} but expecetd V1", var.name);
//                                         // if let ValueDef::Composite(()) = values {
//                                         for v in values {
//                                             match &v.value {
//                                                 ValueDef::Variant(Variant {
//                                                     name,
//                                                     values,
//                                                 }) => {
//                                                     println!("{pallet} {variant} has it {name}");
//                                                 }
//                                                 _ => {
//                                                     println!("misshchg3");
//                                                 }
//                                             }
//                                         }
//                                     // }
//                                     }
//                                     _ => {
//                                          println!("inner misshh");
//                                          print_val(&v.value);
//                                     }
//                                 }

//                                 panic!(
// "op"
//                                 );
                            }
                            if variant.contains("batch") {
                                for arg in &ext.call_data.arguments {
                                    //just first arg
                                    match &arg.value {
                                        ValueDef::Composite(Composite::Unnamed(chars_vals)) => {
                                            for v in chars_vals {
                                                match &v.value {
                                                    ValueDef::Variant(Variant {
                                                        ref name,
                                                        values: Composite::Unnamed(chars_vals),
                                                    }) => {
                                                        let inner_pallet = name;

                                                        for v in chars_vals {
                                                            match &v.value {
                                                                ValueDef::Variant(Variant {
                                                                    name,
                                                                    values,
                                                                }) => {
                                                                    // println!("{pallet} {variant} has inside a {inner_pallet} {name}");
                                                                    children.push(DataEntity::Extrinsic {
                                                                        id: (block_header.number, i as u32),
                                                                        args: vec![format!("{:?}", values)],
                                                                        contains: vec![],
                                                                        raw: vec![], //TODO: should be simples
                                                                        start_link: vec![],
                                                                        end_link: vec![],
                                                                        details: Details { pallet: inner_pallet.to_string(),
                                                                            variant: name.clone(), ..Details::default()}
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

                            let mut results = HashMap::new();
                            for (arg_index, arg) in ext.call_data.arguments.iter().enumerate() {
                                flattern(&arg.value, &arg_index.to_string(),&mut results);
                            }
                            // println!("FLATTERN UMP {:#?}", results);
                            // args.insert(0, format!("{results:#?}"));

                            exts.push(DataEntity::Extrinsic {
                                id: (block_header.number, i as u32),
                                args,
                                contains: children,
                                raw: encoded_extrinsic,
                                start_link,
                                end_link,
                                details: Details{
                                    hover: "".to_string(),
                                    flattern: format!("{results:#?}"),
                                    url:"".to_string(),
                                    parent: None,
                                    success: crate::details::Success::Happy,
                                    pallet,
                                    variant,
                                }
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
                            blockhash: block_hash,
                            extrinsics: exts,
                            events: vec![],
                        });
                    if !current.events.is_empty()  //- blocks sometimes have no events in them.
                    {
                        let mut current = handle.0.remove(&block_hash.to_string()).unwrap();
                        current.extrinsics = ext_clone;
                        handle.1.push(current);
                    }
                    //TODO: assert_eq!(block_header.hash(), block.hash());
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(20));
        reconnects += 1;
        client = ClientBuilder::new().set_url(&url).build().await?;
        api = client
            .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
    }
    Ok(())
}

pub struct PolkaBlock {
    pub blocknum: usize,
    pub blockhash: H256,
    pub extrinsics: Vec<DataEntity>,
    pub events: Vec<DataEvent>,
}
use core::slice::SlicePattern;
pub async fn watch_events(
    tx: ABlocks,
    url: &str,
    as_of: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let metad =
        async_std::task::block_on(get_desub_metadata("wss://statemint-rpc.polkadot.io:443"));
    // TODO: pass metadata into fn.
    let storage = decoder::decode_storage(&metad);
    let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
    let storage_key = hex::decode(events_key).unwrap();
    let events_entry = storage
        .decode_key(&metad, &mut storage_key.as_slice())
        .expect("can decode storage");
    let urlhash = please_hash(url);
    let events_path = format!("target/{urlhash}.metadata.scale.events");
    let _ = std::fs::create_dir(&events_path);

    let client = ClientBuilder::new().set_url(url).build().await?;

    // system.events query is 0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7

    if let Some(as_of) = as_of {
        let mut as_of: u32 = as_of.parse().unwrap();
        for i in as_of..(as_of + 10) {
            let filename = format!("{}/{}.events", events_path, i);

            let bytes = if let Ok(contents) = std::fs::read(&filename) {
                println!("cache hit!");
                Some(contents)
            } else {
                let hash = client.rpc().block_hash(Some(i.into())).await.unwrap();

                let call = client
                    .storage()
                    .fetch_raw(sp_core::storage::StorageKey(storage_key.clone()), hash)
                    .await?;

                if let Some(sp_core::storage::StorageData(events_raw)) = call {
                    std::fs::write(&filename, &events_raw).expect("Couldn't write event output");
                    Some(events_raw)
                } else {
                    None
                }
            };

            if let Some(events_raw) = bytes {
                // let encoded = hex::encode(events_raw.as_slice());
                // println!("events for {} b:{}", &url, encoded);

                if let Ok(val) = decoder::decode_value_by_id(
                    &metad,
                    &events_entry.ty,
                    &mut events_raw.as_slice(),
                ) {
                    if let ValueDef::Composite(Composite::Unnamed(events)) = val.value {
                        for event in &events {
                            println!("start event");
                            print_val(&event.value);

                            if let ValueDef::Composite(Composite::Named(pairs)) = event.value {
                                for (name, val) in pairs {
                                    //phase
                                    // pallet variant?
                                }
                            }


                            println!("end event");
                            


                        }
                        println!("events count {}!", events.len());
                    }
                } else {
                    println!("can't decode events {} / {}", &url, i);
                };

                // let para_id = <u32 as Decode>::decode(&mut events_raw.as_slice()).unwrap();
                // println!("{} is para id {}", &url, para_id);

                // Some(NonZeroU32::try_from(para_id).expect("para id should not be 0"))
            } else {
                warn!("could not find events {}", &i);
                // None
            };

            // Slow things down to reality
            std::thread::sleep(std::time::Duration::from_secs(12));

            // println!("got heree as of ");
            // let hash = api.client.rpc().block_hash(Some(as_of.into())).await.unwrap();
            // println!("got block hash as of ");

            // let res = api.storage().system().events(hash).await;
            // if let Ok(events) = res {
            //     println!("got events as of {} ", events.iter().count());
            //     for event in events {
            //         // let event_d = event.encode();
            //         println!("{:?}", event.event);
            //         // event.event;
            //     }
            // } else {
            //     println!("could not get event {:?}", res);
            // }
            as_of += 1;
        }
    } else {
        let api = client
            .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
    {
            if let Ok(mut event_sub) = api.events().subscribe_finalized().await {
                let mut blocknum = 1;
                while let Some(events) = event_sub.next().await {
                    let events = events?;
                    let blockhash = events.block_hash();
                    blocknum += 1;

                    let mut data_events = vec![];

                    // let events_details: Vec<Details> = events.iter_raw().map(|ev_raw| {
                    //     let mut details = Details::default();
                    //     if let Ok(ev) = &ev_raw {
                    //         details.pallet = ev.pallet.clone();
                    //         details.variant = ev.variant.clone();
                    //     }
                    //     details
                    // }).collect();

                    for ev_raw  in events.iter_raw() {
                        let start_link = vec![];
                        let mut end_link = vec![];
                        let mut details = Details::default();
                        if let Ok(ev) = &ev_raw {
                            
                            details.pallet = ev.pallet.clone();
                        details.variant = ev.variant.clone();
                        
                            if let subxt::Phase::ApplyExtrinsic(ext) = ev.phase {
                                details.parent = Some(ext);
                            }
                           
                            if details.pallet == "XcmPallet" && details.variant == "Attempted" {
                                // use crate::polkadot::runtime_types::xcm::v2::traits::Error;
                                use crate::polkadot::runtime_types::xcm::v2::traits::Outcome; //TODO version
                                let result = <Outcome as Decode>::decode(&mut ev.data.as_slice());
                                if let Ok(outcome) = &result {
                                    match outcome {
                                        Outcome::Complete(_) => details.success = Success::Happy,
                                        Outcome::Incomplete(_, _) => details.success = Success::Worried,
                                        Outcome::Error(_) => details.success = Success::Sad,
                                    }
                                }
                                details.flattern = format!("{:#?}", result);
                            }
                            if details.pallet == "XcmPallet"
                                && details.variant == "ReserveAssetDeposited"
                            {
                                println!("got here WTTTTTTTTTTTTTTTTTTTTTTTtttdrnrtnrtrtnrt");
                                println!("got here WTTTTTTTTTTTTTTTTTTTTTTTtttdrnrtnrtrtnrt");
                                println!("got here WTTTTTTTTTTTTTTTTTTTTTTTtttdrnrtnrtrtnrt");
                                println!("got here WTTTTTTTTTTTTTTTTTTTTTTTtttdrnrtnrtrtnrt");
                                println!("got here rnrtnrtrtnrt");
                                println!("{:#?}", details);
                            }

                            let ev = <polkadot::Event as Decode>::decode(&mut ev.data.as_slice());
                            if let Ok(event) = ev {
                                // println!("{:#?}", ev);
                                // if let EventDetails { event, .. } = ev {
                                if let polkadot::Event::Ump(polkadot::runtime_types::polkadot_runtime_parachains::ump::pallet::Event::ExecutedUpward(ref msg, ..)) = event { //.pallet == "Ump" && ev.variant == "ExecutedUpward" {
                                    println!("got here rnrtnrtrtnrt");
                                    println!("got here rnrtnrtrtnrt");
                                    println!("got here rnrtnrtrtnrt");
                                    println!("got here rnrtnrtrtnrt");
                                    println!("got here rnrtnrtrtnrt");
                                    println!("{:#?}", event);

                                    // Hypothesis: there's no sent_at because it would be the sent at of the individual chain.
                                    // https://substrate.stackexchange.com/questions/2627/how-can-i-see-what-xcm-message-the-moonbeam-river-parachain-has-sent
                                    // TL/DR: we have to wait before we can match up things going upwards...

                                    // blockhash of the recieving block would be incorrect.
                                    let received_hash = format!("{}",hex::encode(msg));
                                    println!("recieved UMP hash {}", &received_hash);
                                    end_link.push(received_hash);
                                    // // msg is a msg id! not decodable - match against hash of original
                                    // if let Ok(ver_msg) = <VersionedXcm as Decode>::decode(&mut msg.as_slice()) {
                                    //     println!("decodearama {:#?}!!!!", ver_msg);
                                    // } else {
                                    //     println!("booo didn't decode!!!! {}", hex::encode(msg.as_slice()));
                                    // }
                                }
                            }
                            // }
                        }
                        data_events.push(DataEvent {
                            // raw: ev_raw.unwrap(),
                            
                            start_link,
                            end_link,
                            details,
                        })
                    }

                    tx.lock().unwrap().0.insert(
                        events.block_hash().to_string(),
                        PolkaBlock {
                            blocknum,
                            blockhash,
                            extrinsics: vec![],
                            events: data_events,
                        },
                    );
                }
            };
        }
    }

    Ok(())
}

pub fn associate_events(
    ext: Vec<DataEntity>,
    mut events: Vec<DataEvent>,
) -> Vec<(Option<DataEntity>, Vec<DataEvent>)> {
    let mut ext: Vec<(Option<DataEntity>, Vec<DataEvent>)> = ext
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
                    .drain_filter(|ev| match ev.details.parent {
                        Some(extrinsic_id) => extrinsic_id == eid,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polkadot::runtime_types::xcm::VersionedXcm;
    // use crate::polkadot::runtime_types::polkadot_core_primitives::DownwardMessage;
    use subxt::BlockNumber;
    #[test]
    fn test() {
        use crate::polkadot::runtime_types::xcm::v2::Instruction::DepositAsset;
        let msg = "02100104000100000700c817a8040a13000100000700c817a804010300286bee0d01000400010100353ea2050ff562d3f6e7683e8b53073f4f91ae684072f6c2f044b815fced30a4";
        let result =
            <VersionedXcm as Decode>::decode(&mut hex::decode(msg).unwrap().as_slice()).unwrap();

        if let VersionedXcm::V2(v2) = result {
            for inst in v2.0 {
                if let DepositAsset { beneficiary, .. } = inst {
                    let ben_hash = please_hash(beneficiary.encode());
                    println!("{:?}", beneficiary);
                    println!("{}", ben_hash);
                }
            }
        }
    }

    #[test]
    fn decode_a_ump_message() {
        let msg = "60ba677909cd50310087509462d25c85010c8ea95d61f2351b42f6b54463f5cf";
        let bytes = hex::decode(msg).unwrap();

        let result = <ExtrinsicVec as Decode>::decode(&mut bytes.as_slice()).unwrap();
        println!("{}", hex::encode(result.0.as_slice()));

        println!("{:?}", &result);
        // let result =
        //     <VersionedXcm as Decode>::decode(&mut result.0.as_slice()).unwrap();

        let result = <crate::polkadot::runtime_types::xcm::v1::Xcm as Decode>::decode(
            &mut result.0.as_slice(),
        );

        println!("{:?}", result);
    }

    #[test]
    fn decode_xcm_cant_transact_error() {
        use crate::polkadot::runtime_types::xcm::v2::traits::Error;
        use crate::polkadot::runtime_types::xcm::v2::traits::Outcome;
        let msg = vec![1u8, 0, 202, 154, 59, 0, 0, 0, 0, 9];
        let result = <Outcome as Decode>::decode(&mut msg.as_slice()).unwrap();
        if let Outcome::Incomplete(_weight, Error::FailedToTransactAsset) = result {
            // The thing only has a string message locally!!!
            //(err_msg)
            //   println!("err msg: {}", err_msg);
        }
        println!("{:?}", result);
    }

    //

    #[test]
    fn decode_events() {
        let metad =
            async_std::task::block_on(get_desub_metadata("wss://statemint-rpc.polkadot.io:443"));

        //Statemint
        let encoded_events =
            "0800000000000000000000000000000002000000010000000000109aa80700000000020000";
        let bin = hex::decode(encoded_events).unwrap();

        let storage = decoder::decode_storage(&metad);

        let key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
        let storage_key = hex::decode(key).unwrap();

        let entry = storage
            .decode_key(&metad, &mut storage_key.as_slice())
            .expect("can decode storage");

        let val = decoder::decode_value_by_id(&metad, &entry.ty, &mut bin.as_slice()).unwrap();

        // let result = <Outcome as Decode>::decode(&mut bin.as_slice()).unwrap();
        // if let Outcome::Incomplete(_weight, Error::FailedToTransactAsset) = result {
        //     // The thing only has a string message locally!!!
        //     //(err_msg)
        //     //   println!("err msg: {}", err_msg);
        // }
        println!("{:#?}", val);
    }
}
