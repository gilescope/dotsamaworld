use super::polkadot;
use crate::polkadot::runtime_types::xcm::VersionedXcm;
use crate::ui::DotUrl;
use crate::ABlocks;
use crate::DataEntity;
use crate::DataEvent;
use crate::Details;
use crate::BASETIME;
use crate::DATASOURCE_EPOC;
use async_std::task::block_on;
use bevy::prelude::warn;
use desub_current::value::*;
use desub_current::ValueDef;
use desub_current::{decoder, Metadata};
use parity_scale_codec::Decode;
use parity_scale_codec::Encode;
use sp_core::H256;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryFrom;
use std::hash::Hash;
use std::num::NonZeroU32;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use subxt::rpc::ClientT;
use subxt::ClientBuilder;
use subxt::DefaultConfig;
use subxt::DefaultExtra;

#[derive(Decode, Debug)]
pub struct ExtrinsicVec(pub Vec<u8>);

mod time_predictor;

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

fn please_hash<T: Hash>(val: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::default();
    val.hash(&mut hasher);
    hasher.finish()
}

async fn get_desub_metadata(url: &str, version: Option<(String, H256)>) -> Option<Metadata> {
    let hash = please_hash(&url);
    let mut params = None;

    let mut metadata_path = format!("target/{hash}.metadata.scale");
    if let Some((version, hash)) = version {
        metadata_path = format!("target/{hash}.metadata.scale.{}", version);

        params = Some(jsonrpsee_types::ParamsSer::Array(vec![
            serde_json::Value::String(hex::encode(hash.as_bytes())),
        ]));
    }

    let metadata_bytes = if let Ok(result) = std::fs::read(&metadata_path) {
        result
    } else {
        println!("cache miss metadata for {} from {}", url, metadata_path);
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
                    Ok(c) => c.request("state_getMetadata", params.clone()).await,
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

    let result = Metadata::from_bytes(&metadata_bytes);
    if result.is_err() {
        eprintln!("should be able to get metadata from {}", &url);
    }
    result.ok()
}

pub async fn get_parachain_id<T: subxt::Config>(
    client: &subxt::Client<T>,
    url: &str,
) -> Option<NonZeroU32> {
    if let Some(cached) = get_cached_parachain_id(url) {
        return Some(cached);
    }
    let urlhash = please_hash(&url);
    let path = format!("target/{urlhash}.metadata.scale.events");
    let _ = std::fs::create_dir(&path);
    let filename = format!("{}/.parachainid", path);

    println!("cache miss parachain id! {}", filename);
    let result = fetch_parachain_id(client, url).await;

    if let Some(para_id) = result {
        std::fs::write(&filename, &para_id.to_string().as_bytes())
            .expect(&format!("Couldn't write event output to {}", filename));
        println!("cache parachain id wrote to {}", filename);
        Some(para_id)
    } else {
        // This is expected for relay chains...
        warn!("could not find para id for {}", &url);
        None
    }
}

pub async fn fetch_parachain_id<T: subxt::Config>(
    client: &subxt::Client<T>,
    url: &str,
) -> Option<NonZeroU32> {
    // parachainInfo / parachainId returns u32 paraId
    let storage_key =
        hex::decode("0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f").unwrap();
    let call = client
        .storage()
        .fetch_raw(sp_core::storage::StorageKey(storage_key), None)
        .await
        .unwrap();

    if let Some(sp_core::storage::StorageData(val)) = call {
        let para_id = <u32 as Decode>::decode(&mut val.as_slice()).unwrap();
        println!("{} is para id {}", &url, para_id);
        let para_id = NonZeroU32::try_from(para_id).expect("para id should not be 0");
        Some(para_id)
    } else {
        // This is expected for relay chains...
        warn!("could not find para id for {}", &url);
        None
    }
}

pub fn get_cached_parachain_id(url: &str) -> Option<NonZeroU32> {
    let urlhash = please_hash(&url);
    let path = format!("target/{urlhash}.metadata.scale.events");
    let _ = std::fs::create_dir(&path);
    let filename = format!("{}/.parachainid", path);

    // Relay chains do not have parachain ids.
    if url == "wss://kusama-rpc.polkadot.io:443" || url == "wss://rpc.polkadot.io:443" {
        return None;
    }

    if let Ok(contents) = std::fs::read(&filename) {
        let para_id: NonZeroU32 = String::from_utf8_lossy(&contents).parse().unwrap();
        Some(para_id)
    } else {
        None
    }
}

pub fn get_parachain_id_from_url(url: &str) -> Result<Option<NonZeroU32>, ()> {
    if let Some(cached_id) = get_cached_parachain_id(url) {
        return Ok(Some(cached_id));
    }
    let result: Result<subxt::Client<subxt::DefaultConfig>, _> =
        async_std::task::block_on(ClientBuilder::new().set_url(url).build());
    if let Ok(client) = result {
        let para_id: Option<NonZeroU32> =
            async_std::task::block_on(get_parachain_id(&client, &url));
        Ok(para_id)
    } else {
        eprintln!("COULD NOT GET CLIENT FOR URL {}", url);
        Err(())
    }
}

async fn get_metadata_version<T: subxt::Config>(
    client: &subxt::Client<T>,
    url: &str,
    hash: T::Hash,
    block_number: u32,
) -> Option<String> {
    let urlhash = please_hash(&url);
    let path = format!("target/{urlhash}.metadata.scale.events");
    let _ = std::fs::create_dir(&path);
    let filename = format!("{}/{}.metadataid", path, block_number);

    if let Ok(contents) = std::fs::read(&filename) {
        Some(String::from_utf8(contents).unwrap())
    } else {
        println!("cache miss metadata id! {}", filename);
        // parachainInfo / parachainId returns u32 paraId
        let storage_key =
            hex::decode("26aa394eea5630e07c48ae0c9558cef7f9cce9c888469bb1a0dceaa129672ef8")
                .unwrap();
        let call = client
            .storage()
            .fetch_raw(
                sp_core::storage::StorageKey(storage_key.clone()),
                Some(hash),
            )
            .await;
        let call = match call {
            Ok(call) => call,
            Err(err) => {
                let err = err.to_string();
                // TODO: if we're looking at finalised blocks why are we running into this?
                let needle = "State already discarded for BlockId::Hash(";

                let pos = err.find(needle);
                if let Some(_pos) = pos {
                    eprintln!(
                        "{} is not alas an archive node and does not go back this far in time.",
                        &url
                    );
                    return None; // If you get this error you need to point to an archive node.
                                 // println!("error message (recoverable) {}", &err);
                                 // let pos = pos + needle.len() + "0x".len();
                                 // if let Ok(new_hash) = hex::decode(&err[pos..(pos + 64)]) {
                                 //     println!("found new hash decoded {}", &err[pos..(pos + 64)]);
                                 //     // T::Hashing()
                                 //     let hash = T::Hashing::hash(new_hash.as_slice());
                                 //     let call = client
                                 //         .storage()
                                 //         .fetch_raw(sp_core::storage::StorageKey(storage_key), Some(hash))
                                 //         .await
                                 //         .unwrap();
                                 //     call
                                 // } else {
                                 //     panic!("could not recover from error {:?}", err);
                                 // }
                } else {
                    panic!("could not recover from error2 {:?}", err);
                }
            }
        };

        if let Some(sp_core::storage::StorageData(val)) = call {
            let res = hex::encode(val.as_slice());
            std::fs::write(&filename, &res.as_bytes())
                .expect(&format!("Couldn't write event output to {}", filename));
            println!("cache metadata id wrote to {}", filename);
            Some(res)
        } else {
            warn!("could not find metadata id for {}", &url);
            None
        }
    }
}

use crate::polkadot::RuntimeApi;
use subxt::Client;
// use subxt::DefaultExtraWithTxPayment;
async fn get_parachain_name<T: subxt::Config>(
    api: &polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
    url: &str,
) -> Option<String>
where
    RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>: From<Client<T>>,
{
    let urlhash = please_hash(&url);
    let path = format!("target/{urlhash}.metadata.scale.events");

    let filename = format!("{}/.parachainname", path);
    if let Ok(contents) = std::fs::read(&filename) {
        let para_name = String::from_utf8_lossy(&contents);
        Some(para_name.to_string())
    } else {
        println!("cache miss parachain name!");
        let parachain_name: String = api.client.rpc().system_chain().await.unwrap();
        std::fs::write(&filename, &parachain_name.as_bytes()).expect("Couldn't write event output");
        Some(parachain_name)
    }
}

async fn get_block_hash<T: subxt::Config>(
    api: &polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
    url: &str,
    block_number: u32,
) -> Option<sp_core::H256>
where
    RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>: From<Client<T>>,
{
    let urlhash = please_hash(&url);
    let path = format!("target/{urlhash}.metadata.scale.events");

    let filename = format!("{}/{}.blockhash", path, block_number);
    if let Ok(contents) = std::fs::read(&filename) {
        let para_name = sp_core::H256::from_slice(&hex::decode(contents).unwrap());
        Some(para_name)
    } else {
        println!("cache miss! block hash {} {}", url, block_number);
        if let Ok(Some(block_hash)) = api.client.rpc().block_hash(Some(block_number.into())).await {
            std::fs::write(&filename, &hex::encode(block_hash.as_bytes()))
                .expect("Couldn't write event output");
            Some(block_hash)
        } else {
            None
        }
    }
}

pub type RelayBlockNumber = u32;

pub async fn watch_blocks(
    tx: ABlocks,
    url: String,
    as_of: Option<DotUrl>,
    parachain_doturl: DotUrl,
    recieve_channel: crossbeam_channel::Receiver<(RelayBlockNumber, H256)>,
    sender: Option<HashMap<NonZeroU32, crossbeam_channel::Sender<(RelayBlockNumber, H256)>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let para_id = parachain_doturl.para_id.clone();
    let mut client = ClientBuilder::new().set_url(&url).build().await?;

    let mut api =
        client.to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
    let parachain_name = get_parachain_name(&api, &url).await.unwrap();

    {
        let mut parachain_info = tx.lock().unwrap();
        parachain_info.2.chain_name = parachain_name.clone();
        parachain_info.2.chain_ws = url.clone();
        parachain_info.2.chain_id = para_id
    }

    let our_data_epoc = DATASOURCE_EPOC.load(Ordering::Relaxed);

    if let Some(as_of) = as_of {
        // if we are a parachain then we need the relay chain to tell us which numbers it is interested in
        if para_id.is_some() {
            // Parachain (listening for relay blocks' para include candidate included events.)
            while let Ok((_relay_block_number, block_hash)) = recieve_channel.recv() {
                let _ = process_extrinsics(
                    &tx,
                    parachain_doturl.clone(),
                    block_hash,
                    &url,
                    &api,
                    &sender,
                    our_data_epoc,
                )
                .await;
                if our_data_epoc != DATASOURCE_EPOC.load(Ordering::Relaxed) {
                    return Ok(());
                }
            }
        } else {
            // Relay chain.
            let mut as_of = as_of.clone();

            // Is the as of the block number of a different relay chain?
            if as_of.sovereign != parachain_doturl.sovereign {
                // if so, we have to wait till we know what the time is of that block to proceed.
                // then we can figure out our nearest block based on that timestamp...
                while BASETIME.load(Ordering::Relaxed) == 0 {
                    thread::sleep(Duration::from_millis(250));
                }

                let basetime = BASETIME.load(Ordering::Relaxed);
                let time_for_blocknum = |blocknum: u32| {
                    let block_hash: sp_core::H256 =
                        block_on(get_block_hash(&api, &url, blocknum)).unwrap();

                    block_on(find_timestamp(
                        parachain_doturl.clone(),
                        block_hash,
                        &url,
                        &api,
                    ))
                };
                as_of.block_number = dbg!(time_predictor::get_block_number_near_timestamp(
                    basetime,
                    10000,
                    &time_for_blocknum,
                    None,
                ));
            }

            for block_number in as_of.block_number.unwrap().. {
                let block_hash: Option<sp_core::H256> =
                    get_block_hash(&api, &url, block_number).await;

                if block_hash.is_none() {
                    eprintln!(
                        "should be able to get from relay chain hash for block num {} of url {}",
                        block_number, &url
                    );
                    continue;
                }
                let block_hash = block_hash.unwrap();
                let _ = process_extrinsics(
                    &tx,
                    parachain_doturl.clone(),
                    block_hash,
                    &url,
                    &api,
                    &sender,
                    our_data_epoc,
                )
                .await;
                std::thread::sleep(std::time::Duration::from_secs(6));
                // check for stop signal
                if our_data_epoc != DATASOURCE_EPOC.load(Ordering::Relaxed) {
                    return Ok(());
                }
            }
        }
    } else {
        let mut reconnects = 0;
        while reconnects < 20 {
            if let Ok(mut block_headers) = api.client.rpc().subscribe_finalized_blocks().await {
                while let Some(Ok(block_header)) = block_headers.next().await {
                    let _ = process_extrinsics(
                        &tx,
                        parachain_doturl.clone(),
                        block_header.hash(),
                        &url,
                        &api,
                        &None,
                        our_data_epoc,
                    )
                    .await;
                    // check for stop signal
                    if our_data_epoc != DATASOURCE_EPOC.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(20));
            reconnects += 1;
            client = ClientBuilder::new().set_url(&url).build().await?;
            api = client
                .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>(
                );
        }
    }
    Ok(())
}

async fn process_extrinsics(
    tx: &ABlocks,
    mut blockurl: DotUrl,
    block_hash: H256,
    url: &str,
    api: &RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
    sender: &Option<HashMap<NonZeroU32, crossbeam_channel::Sender<(RelayBlockNumber, H256)>>>,
    our_data_epoc: u32,
) -> Result<(), ()> {
    if let Ok((got_block_num, extrinsics)) = get_extrinsics(&url, &api, block_hash).await {
        let mut timestamp = None;
        blockurl.block_number = Some(got_block_num);
        let block_number = blockurl.block_number.unwrap();

        let version = get_metadata_version(&api.client, &url, block_hash, block_number).await;
        let metad = if let Some(version) = version {
            get_desub_metadata(&url, Some((version, block_hash))).await
        } else {
            //TODO: This is unlikely to work. we should try the oldest metadata we have instead...
            get_desub_metadata(&url, None).await
        };
        if metad.is_none() {
            //println!("skip")
            return Err(());
        }
        let metad = metad.unwrap();

        let mut exts = vec![];
        for (i, encoded_extrinsic) in extrinsics.iter().enumerate() {
            // let <ExtrinsicVec as Decode >::decode(&mut ext_bytes.as_slice());
            let ex_slice = <ExtrinsicVec as Decode>::decode(&mut encoded_extrinsic.as_slice())
                .unwrap()
                .0;

            // let ex_slice = &ext_bytes.0;
            if let Ok(extrinsic) =
                decoder::decode_unwrapped_extrinsic(&metad, &mut ex_slice.as_slice())
            {
                let entity = process_extrisic(
                    (ex_slice).clone(),
                    &extrinsic,
                    DotUrl {
                        extrinsic: Some(i as u32),
                        ..blockurl.clone()
                    },
                    url,
                )
                .await;
                if let Some(entity) = entity {
                    if entity.pallet() == "Timestamp" && entity.variant() == "set" {
                        if let ValueDef::Primitive(Primitive::U64(val)) =
                            extrinsic.call_data.arguments[0].value
                        {
                            timestamp = Some(val);
                        }
                    }
                    exts.push(entity);
                }
            } else {
                println!("can't decode block ext {}-{} {}", block_number, i, &url);
            }
        }
        let events = get_events_for_block(&api, &url, block_hash, &sender, &blockurl)
            .await
            .or(Err(()))?;

        let mut handle = tx.lock().unwrap();
        let current = PolkaBlock {
            data_epoc: our_data_epoc,
            timestamp,
            blockurl,
            blockhash: block_hash,
            extrinsics: exts,
            events,
        };

        //- blocks sometimes have no events in them.
        handle.1.push(current);
    }
    Ok(())
}

// Find the timestamp for a block (cut down version of `process_extrinsics`)
async fn find_timestamp(
    mut blockurl: DotUrl,
    block_hash: H256,
    url: &str,
    api: &RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
) -> Option<u64> {
    if let Ok((got_block_num, extrinsics)) = get_extrinsics(&url, &api, block_hash).await {
        blockurl.block_number = Some(got_block_num);
        let block_number = blockurl.block_number.unwrap();

        let version = get_metadata_version(&api.client, &url, block_hash, block_number).await;
        let metad = if let Some(version) = version {
            get_desub_metadata(&url, Some((version, block_hash))).await
        } else {
            //TODO: This is unlikely to work. we should try the oldest metadata we have instead...
            get_desub_metadata(&url, None).await
        }
        .unwrap_or_else(|| block_on(get_desub_metadata(&url, None)).unwrap());

        for (i, encoded_extrinsic) in extrinsics.iter().enumerate() {
            let ex_slice = <ExtrinsicVec as Decode>::decode(&mut encoded_extrinsic.as_slice())
                .unwrap()
                .0;
            if let Ok(extrinsic) =
                decoder::decode_unwrapped_extrinsic(&metad, &mut ex_slice.as_slice())
            {
                let entity = process_extrisic(
                    (ex_slice).clone(),
                    &extrinsic,
                    DotUrl {
                        extrinsic: Some(i as u32),
                        ..blockurl.clone()
                    },
                    url,
                )
                .await;
                if let Some(entity) = entity {
                    if entity.pallet() == "Timestamp" && entity.variant() == "set" {
                        if let ValueDef::Primitive(Primitive::U64(val)) =
                            extrinsic.call_data.arguments[0].value
                        {
                            return Some(val);
                        }
                    }
                }
            }
        }
    }
    None
}

// fetches extrinsics from node for a block number (wrapped by a file cache).
async fn get_extrinsics(
    // relay_block_number: u32,
    url: &str,
    api: &RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
    block_hash: H256,
) -> Result<(u32, Vec<Vec<u8>>), ()> {
    let urlhash = please_hash(&url);
    let path = format!("target/{urlhash}.metadata.scale.events");
    let _ = std::fs::create_dir(&path);

    let filename = format!("{}/{}.block", path, hex::encode(block_hash.as_bytes()));
    if let Ok(contents) = std::fs::read(&filename) {
        let temp = String::from_utf8(contents).unwrap();
        let mut exs: Vec<_> = temp.lines().collect();
        let block_num_str = exs.remove(0);
        let block_num: u32 = block_num_str.parse().unwrap();
        let exs = exs.iter().map(|ex| hex::decode(ex).unwrap()).collect();
        Some((block_num, exs))
    } else {
        println!("cache miss block! {} {}", url, filename);
        if let Ok(Some(block_body)) = api.client.rpc().block(Some(block_hash)).await {
            let mut vals = block_body.block.extrinsics.iter().fold(
                block_body.block.header.number.to_string(),
                |mut buf, ex| {
                    buf.push('\n');
                    buf.push_str(&hex::encode(ex.encode()));
                    buf
                },
            );
            vals.push('\n');
            // let bytes = block_body.block.extrinsics.encode();
            std::fs::write(&filename, vals.as_bytes()).expect("Couldn't write block");

            // let exts = <Vec<ExtrinsicVec> as Decode >::decode(&mut bytes.as_slice());
            // desub_current::decoder::decode_extrinsics(&metad, &mut bytes.as_slice()).unwrap();
            let exs = vals
                .lines()
                .skip(1)
                .map(|ex| hex::decode(ex).unwrap())
                .collect();

            Some((block_body.block.header.number, exs))
        } else {
            None
        }
    }
    .ok_or(())
}

async fn process_extrisic<'a>(
    // relay_id: &str,
    // metad: &Metadata,
    // para_id: Option<NonZeroU32>,
    // block_number: u32,
    // block_hash: H256,
    ex_slice: Vec<u8>,
    ext: &desub_current::decoder::Extrinsic<'a>,
    extrinsic_url: DotUrl,
    url: &str,
) -> Option<DataEntity> {
    let block_number = extrinsic_url.block_number.unwrap();
    let para_id = extrinsic_url.para_id.clone();
    // let s : String = ext_bytes;
    // ext_bytes.using_encoded(|ref slice| {
    //     assert_eq!(slice, &b"\x0f");

    // let encoded_extrinsic = ext_bytes.encode();
    // let ex_slice = <ExtrinsicVec as Decode>::decode(&mut encoded_extrinsic.as_slice())
    //     .unwrap()
    //     .0;
    // This works too but unsafe:
    //let ex_slice2: Vec<u8> = unsafe { std::mem::transmute(ext_bytes.clone()) };

    // use parity_scale_codec::Encode;
    // ext_bytes.encode();
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
            desub_current::ValueDef::Composite(desub_current::value::Composite::Unnamed(
                chars_vals,
            )) => {
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
        match &ext.call_data.arguments[0].value {
            ValueDef::Composite(Composite::Named(named)) => {
                for (name, val) in named {
                    match name.as_str() {
                        "upward_messages" => {
                            println!("found upward msgs (first time)");
                            print_val(&val.value);
                        }
                        "horizontal_messages" => {
                            if let ValueDef::Composite(Composite::Unnamed(vals)) = &val.value {
                                for val in vals {
                                    // channels
                                    if let ValueDef::Composite(Composite::Unnamed(vals)) =
                                        &val.value
                                    {
                                        for val in vals {
                                            // single channel
                                            if let ValueDef::Composite(Composite::Unnamed(vals)) =
                                                &val.value
                                            {
                                                for val in vals {
                                                    // Should be a msg
                                                    //  for val in vals {
                                                    //msgs
                                                    if let ValueDef::Composite(
                                                        Composite::Unnamed(vals),
                                                    ) = &val.value
                                                    {
                                                        if vals.len() > 0 {
                                                            for m in vals {
                                                                if let ValueDef::Primitive(
                                                                    Primitive::U32(_from_para_id),
                                                                ) = &m.value
                                                                {
                                                                    // println!("from {}", from_para_id);
                                                                }

                                                                if vals.len() > 1 {
                                                                    let mut results =
                                                                        HashMap::new();
                                                                    flattern(
                                                                        &val.value,
                                                                        "",
                                                                        &mut results,
                                                                    );
                                                                    println!(
                                                                        "INNER {:#?}",
                                                                        results
                                                                    );
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
                        }
                        "downward_messages" => {
                            if let ValueDef::Composite(Composite::Unnamed(vals)) = &val.value {
                                for val in vals {
                                    let mut results = HashMap::new();
                                    flattern(&val.value, "", &mut results);
                                    println!("FLATTERN {:#?}", results);
                                    // also .sent_at
                                    if let Some(msg) = results.get(".msg") {
                                        if let Some(sent_at) = results.get(".sent_at") {
                                            let bytes = hex::decode(msg).unwrap();
                                            if let Ok(ver_msg) = <VersionedXcm as Decode>::decode(
                                                &mut bytes.as_slice(),
                                            ) {
                                                match ver_msg {
                                                    VersionedXcm::V0(msg) => {
                                                        // Only one xcm instruction in a v1 message.
                                                        let instruction = format!("{:?}", &msg);
                                                        println!("instruction {:?}", &instruction);
                                                        children.push(DataEntity::Extrinsic {
                                                            // id: (block_number, extrinsic_urlbextrinsic_index),
                                                            args: vec![instruction.clone()],
                                                            contains: vec![],
                                                            raw: vec![], //TODO: should be simples
                                                            start_link: vec![],
                                                            end_link: vec![],
                                                            details: Details {
                                                                pallet: "Instruction".to_string(),
                                                                variant: instruction
                                                                    .split_once(' ')
                                                                    .unwrap()
                                                                    .0
                                                                    .to_string(),
                                                                doturl: extrinsic_url.clone(),
                                                                ..Details::default()
                                                            },
                                                        });
                                                        let inst = msg;
                                                        use crate::polkadot::runtime_types::xcm::v0::Xcm::TransferReserveAsset;
                                                                use crate::polkadot::runtime_types::xcm::v0::multi_location::MultiLocation;
                                                                use crate::polkadot::runtime_types::xcm::v0::junction::Junction;
                                                        if let TransferReserveAsset {
                                                            dest, ..
                                                        } = inst
                                                        {
                                                            if let MultiLocation::X1(x1) = &dest {
                                                                //todo assert parent
                                                                if let Junction::AccountId32 {
                                                                    id,
                                                                    ..
                                                                } = x1
                                                                {
                                                                    let msg_id = format!(
                                                                        "{}-{}",
                                                                        sent_at,
                                                                        please_hash(&hex::encode(
                                                                            id
                                                                        ))
                                                                    );
                                                                    println!(
                                                                        "RECIEVE HASH v0 {}",
                                                                        msg_id
                                                                    );
                                                                    end_link.push(msg_id.clone());
                                                                    start_link.push(msg_id);
                                                                    // for reserve assets received.
                                                                };
                                                            } else {
                                                                panic!("unknonwn")
                                                            }
                                                        }
                                                    }
                                                    VersionedXcm::V1(msg) => {
                                                        // Only one xcm instruction in a v1 message.
                                                        let instruction = format!("{:?}", &msg);
                                                        println!("instruction {:?}", &instruction);
                                                        children.push(DataEntity::Extrinsic {
                                                            // id: (block_number, extrinsic_index),
                                                            args: vec![instruction.clone()],
                                                            contains: vec![],
                                                            raw: vec![], //TODO: should be simples
                                                            start_link: vec![],
                                                            end_link: vec![],
                                                            details: Details {
                                                                pallet: "Instruction".to_string(),
                                                                variant: instruction
                                                                    .split_once(' ')
                                                                    .unwrap()
                                                                    .0
                                                                    .to_string(),
                                                                doturl: extrinsic_url.clone(),
                                                                ..Details::default()
                                                            },
                                                        });
                                                        let inst = msg;
                                                        use crate::polkadot::runtime_types::xcm::v1::Xcm::TransferReserveAsset;
                                                                use crate::polkadot::runtime_types::xcm::v1::multilocation::MultiLocation;
                                                                use crate::polkadot::runtime_types::xcm::v1::multilocation::Junctions;
                                                                use crate::polkadot::runtime_types::xcm::v1::junction::Junction;
                                                        if let TransferReserveAsset {
                                                            dest, ..
                                                        } = inst
                                                        {
                                                            let MultiLocation { interior, .. } =
                                                                &dest;
                                                            //todo assert parent
                                                            if let Junctions::X1(x1) = interior {
                                                                if let Junction::AccountId32 {
                                                                    id,
                                                                    ..
                                                                } = x1
                                                                {
                                                                    let msg_id = format!(
                                                                        "{}-{}",
                                                                        sent_at,
                                                                        please_hash(&hex::encode(
                                                                            id
                                                                        ))
                                                                    );
                                                                    println!(
                                                                        "RECIEVE HASH v1 {}",
                                                                        msg_id
                                                                    );
                                                                    end_link.push(msg_id.clone());
                                                                    start_link.push(msg_id);
                                                                    // for reserve assets received.
                                                                };
                                                            } else {
                                                                panic!("unknonwn")
                                                            }
                                                        }
                                                    }
                                                    VersionedXcm::V2(msg) => {
                                                        for instruction in &msg.0 {
                                                            let instruction =
                                                                format!("{:?}", instruction);
                                                            println!(
                                                                "instruction {:?}",
                                                                &instruction
                                                            );
                                                            children.push(DataEntity::Extrinsic {
                                                                args: vec![instruction.clone()],
                                                                contains: vec![],
                                                                raw: vec![], //TODO: should be simples
                                                                start_link: vec![],
                                                                end_link: vec![],
                                                                details: Details {
                                                                    pallet: "Instruction"
                                                                        .to_string(),
                                                                    variant: instruction
                                                                        .split_once(' ')
                                                                        .unwrap_or((
                                                                            &instruction,
                                                                            "",
                                                                        ))
                                                                        .0
                                                                        .to_string(),
                                                                    doturl: extrinsic_url.clone(),
                                                                    ..Details::default()
                                                                },
                                                            });
                                                        }
                                                        for inst in msg.0 {
                                                            //TODO: should only be importing from one version probably.
                                                            use crate::polkadot::runtime_types::xcm::v2::Instruction::DepositAsset;
                                                                    use crate::polkadot::runtime_types::xcm::v1::multilocation::MultiLocation;
                                                                    use crate::polkadot::runtime_types::xcm::v1::multilocation::Junctions;
                                                                    use crate::polkadot::runtime_types::xcm::v1::junction::Junction;
                                                            if let DepositAsset {
                                                                beneficiary,
                                                                ..
                                                            } = inst
                                                            {
                                                                let MultiLocation {
                                                                    interior, ..
                                                                } = &beneficiary;
                                                                //todo assert parent
                                                                if let Junctions::X1(x1) = interior
                                                                {
                                                                    if let Junction::AccountId32 {
                                                                        id,
                                                                        ..
                                                                    } = x1
                                                                    {
                                                                        let msg_id = format!(
                                                                            "{}-{}",
                                                                            sent_at,
                                                                            please_hash(
                                                                                &hex::encode(id)
                                                                            )
                                                                        );
                                                                        println!(
                                                                            "RECIEVE HASH v2 {}",
                                                                            msg_id
                                                                        );
                                                                        end_link
                                                                            .push(msg_id.clone());
                                                                        start_link.push(msg_id);
                                                                        // for reserve assets received.
                                                                    };
                                                                } else {
                                                                    panic!("unknonwn")
                                                                }
                                                            }
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
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if pallet == "XcmPallet" && variant == "limited_teleport_assets" {
        let mut flat0 = HashMap::new();
        flattern(&ext.call_data.arguments[0].value, "", &mut flat0);
        // println!("FLATTERN {:#?}", flat0);
        let mut flat1 = HashMap::new();
        flattern(&ext.call_data.arguments[1].value, "", &mut flat1);

        if let Some(dest) = flat0.get(".V2.0.interior.X1.0.Parachain.0") {
            let to = flat1.get(".V2.0.interior.X1.0.AccountId32.id");
            let dest: NonZeroU32 = dest.parse().unwrap();

            if let Some(to) = to {
                let msg_id = format!("{}-{}", block_number, please_hash(to));
                println!("SEND MSG v0 hash {}", msg_id);
                start_link.push(msg_id);
            }
            println!("first time seeen");
            println!("v2 limited_teleport_assets from {:?} to {}", para_id, dest);
        } else if let Some(dest) = flat0.get(".V1.0.interior.X1.0.Parachain.0") {
            let to = flat1.get(".V1.0.interior.X1.0.AccountId32.id");
            let dest: NonZeroU32 = dest.parse().unwrap();

            if let Some(to) = to {
                let msg_id = format!("{}-{}", block_number, please_hash(to));
                println!("SEND MSG v0 hash {}", msg_id);
                start_link.push(msg_id);
            }
            println!("first time seeen");
            println!("v1 limited_teleport_assets from {:?} to {}", para_id, dest);
        } else if let Some(dest) = flat0.get(".V0.0.X1.0.Parachain.0") {
            let to = flat1.get(".V0.0.X1.0.AccountId32.id");
            let dest: NonZeroU32 = dest.parse().unwrap();

            if let Some(to) = to {
                let msg_id = format!("{}-{}", block_number, please_hash(to));
                println!("SEND MSG v0 hash {}", msg_id);
                start_link.push(msg_id);
            }
            println!("v0 limited_teleport_assets from {:?} to {}", para_id, dest);
        }

        // println!("FLATTERN {:#?}", flat1);
        // println!("BOB");
        //          print_val(&ext.call_data.arguments[0].value);
        //          println!("BOB");
        //         print_val(&ext.call_data.arguments[1].value);
    }

    // Anything that looks batch like we will assume is a batch
    if pallet == "XcmPallet" && variant == "reserve_transfer_assets" {
        check_reserve_asset(&ext.call_data.arguments, &extrinsic_url, &mut start_link).await;
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
                                        ValueDef::Variant(Variant { name, values }) => {
                                            // println!("{pallet} {variant} has inside a {inner_pallet} {name}");
                                            children.push(DataEntity::Extrinsic {
                                                // id: (block_number, extrinsic_index),
                                                args: vec![format!("{:?}", values)],
                                                contains: vec![],
                                                raw: vec![], //TODO: should be simples
                                                start_link: vec![],
                                                end_link: vec![],
                                                details: Details {
                                                    pallet: inner_pallet.to_string(),
                                                    variant: name.clone(),
                                                    doturl: extrinsic_url.clone(),
                                                    ..Details::default()
                                                },
                                            });

                                            if inner_pallet == "XcmPallet"
                                                && name == "reserve_transfer_assets"
                                            {
                                                match values {
                                                    Composite::Named(named) => {
                                                        let vec: Vec<Value<TypeId>> = named
                                                            .iter()
                                                            .map(|(_n, v)| v.clone())
                                                            .collect();
                                                        check_reserve_asset(
                                                            &vec,
                                                            &extrinsic_url,
                                                            &mut start_link,
                                                        )
                                                        .await;
                                                    }
                                                    Composite::Unnamed(_named) => {
                                                        panic!("unexpected");
                                                    }
                                                }
                                            }
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
        flattern(&arg.value, &arg_index.to_string(), &mut results);
    }
    // println!("FLATTERN UMP {:#?}", results);
    // args.insert(0, format!("{results:#?}"));

    Some(DataEntity::Extrinsic {
        // id: (block_number, extrinsic_url.extrinsic.unwrap()),
        args,
        contains: children,
        raw: ex_slice,
        start_link,
        end_link,
        details: Details {
            hover: "".to_string(),
            doturl: extrinsic_url,
            flattern: format!("{results:#?}"),
            url: url.to_string(),
            parent: None,
            success: crate::ui::details::Success::Happy,
            pallet,
            variant,
        },
    })

    // let ext = decoder::decode_extrinsic(&meta, &mut ext_bytes.0.as_slice()).expect("can decode extrinsic");
}
use desub_current::TypeId;
async fn check_reserve_asset<'a, 'b>(
    args: &Vec<Value<TypeId>>,
    extrinsic_url: &DotUrl,
    start_link: &'b mut Vec<String>,
) {
    let mut flat0 = HashMap::new();
    flattern(&args[0].value, "", &mut flat0);
    // println!("FLATTERN {:#?}", flat0);
    let mut flat1 = HashMap::new();
    flattern(&args[1].value, "", &mut flat1);

    if let Some(dest) = flat0.get(".V2.0.interior.X1.0.Parachain.0") {
        let to = flat1.get(".V2.0.interior.X1.0.AccountId32.id");

        println!("first time!");
        //TODO; something with parent for cross relay chain maybe.(flat1.get(".V1.0.parents"),
        let dest: NonZeroU32 = dest.parse().unwrap();

        if let Some(to) = to {
            let msg_id = format!(
                "{}-{}",
                extrinsic_url.block_number.unwrap(),
                please_hash(to)
            );
            println!("SEND MSG v2 hash {}", msg_id);
            start_link.push(msg_id);
        }
        println!(
            "Reserve_transfer_assets from {:?} to {}",
            extrinsic_url.para_id, dest
        );
    }
    if let Some(dest) = flat0.get(".V1.0.interior.X1.0.Parachain.0") {
        let to = flat1.get(".V1.0.interior.X1.0.AccountId32.id");

        //TODO; something with parent for cross relay chain maybe.(flat1.get(".V1.0.parents"),
        let dest: NonZeroU32 = dest.parse().unwrap();

        if let Some(to) = to {
            let msg_id = format!(
                "{}-{}",
                extrinsic_url.block_number.unwrap(),
                please_hash(to)
            );
            println!("SEND MSG v1 hash {}", msg_id);
            start_link.push(msg_id);
        }
        println!(
            "Reserve_transfer_assets from {:?} to {}",
            extrinsic_url.para_id, dest
        );
    }
    if let Some(dest) = flat0.get(".V0.0.X1.0.Parachain.0") {
        let to = flat1.get(".V0.0.X1.0.AccountId32.id");

        //TODO; something with parent for cross relay chain maybe.(flat1.get(".V1.0.parents"),
        let dest: NonZeroU32 = dest.parse().unwrap();

        if let Some(to) = to {
            let msg_id = format!(
                "{}-{}",
                extrinsic_url.block_number.unwrap(),
                please_hash(to)
            );
            println!("SEND MSG v0 hash {}", msg_id);
            start_link.push(msg_id);
        }
        println!(
            "Reserve_transfer_assets from {:?} to {}",
            extrinsic_url.para_id, dest
        );
    }
}

pub struct PolkaBlock {
    pub data_epoc: u32,
    pub timestamp: Option<u64>,
    pub blockurl: DotUrl,
    // pub blocknum: Option<u32>,
    pub blockhash: H256,
    pub extrinsics: Vec<DataEntity>,
    pub events: Vec<DataEvent>,
}

async fn get_events_for_block(
    api: &polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>,
    url: &str,
    blockhash: H256,
    sender: &Option<HashMap<NonZeroU32, crossbeam_channel::Sender<(RelayBlockNumber, H256)>>>,
    block_url: &DotUrl,
) -> Result<Vec<DataEvent>, Box<dyn std::error::Error>> {
    let mut data_events = vec![];
    let version = get_metadata_version(
        &api.client,
        &url,
        blockhash,
        block_url.block_number.unwrap(),
    )
    .await;
    let urlhash = please_hash(&url);
    let events_path = format!("target/{urlhash}.metadata.scale.events");
    let blocknum = block_url.block_number.unwrap();

    let metad = if let Some(version) = version {
        async_std::task::block_on(get_desub_metadata(&url, Some((version, blockhash))))
    } else {
        //TODO: Should use oldest metadata
        async_std::task::block_on(get_desub_metadata(&url, None))
    }
    .unwrap();
    // TODO: pass metadata into fn.
    let storage = decoder::decode_storage(&metad);
    let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
    let storage_key = hex::decode(events_key).unwrap();
    let events_entry = storage
        .decode_key(&metad, &mut storage_key.as_slice())
        .expect("can decode storage");

    let filename = format!("{}/{}.events", events_path, blocknum);
    let bytes = if let Ok(contents) = std::fs::read(&filename) {
        // println!("cache hit events!");
        Some(hex::decode(contents).unwrap())
    } else {
        println!("cache miss events!");
        let call = api
            .client
            .storage()
            .fetch_raw(
                sp_core::storage::StorageKey(storage_key.clone()),
                Some(blockhash),
            )
            .await?;

        if let Some(sp_core::storage::StorageData(events_raw)) = call {
            std::fs::write(&filename, &hex::encode(&events_raw))
                .expect("Couldn't write event output");
            Some(events_raw)
        } else {
            None
        }
    };

    if let Some(events_raw) = bytes {
        // println!("{} len {}", blocknum, events_raw.len());

        let version = get_metadata_version(&api.client, &url, blockhash, blocknum)
            .await
            .unwrap();
        let metad =
            async_std::task::block_on(get_desub_metadata(url, Some((version, blockhash)))).unwrap();

        if let Ok(val) =
            decoder::decode_value_by_id(&metad, &events_entry.ty, &mut events_raw.as_slice())
        {
            if let ValueDef::Composite(Composite::Unnamed(events)) = val.value {
                let mut inclusions = vec![];
                let mut ext_count_map = HashMap::new();
                for event in events.iter() {
                    let start_link = vec![];
                    let end_link = vec![];
                    let mut details = Details::default();
                    details.url = url.to_string();
                    details.doturl = DotUrl {
                        ..block_url.clone()
                    };

                    // println!("start event");
                    // print_val(&event.value);

                    if let ValueDef::Composite(Composite::Named(ref pairs)) = event.value {
                        for (name, val) in pairs {
                            if name == "phase" {
                                // println!("phase start");
                                if let ValueDef::Variant(ref var) = val.value {
                                    if var.name == "ApplyExtrinsic" {
                                        //Has extrisic

                                        if let Composite::Unnamed(ref vals) = var.values {
                                            for val in vals {
                                                if let ValueDef::Primitive(Primitive::U32(v)) =
                                                    val.value
                                                {
                                                    details.parent = Some(v);
                                                    let count = ext_count_map.entry(v).or_insert(0);
                                                    *count += 1;
                                                    details.doturl.extrinsic = Some(v);
                                                    details.doturl.event = Some(*count);
                                                }
                                            }
                                        }
                                        if details.parent.is_none() {
                                            // system event. increment the system event count:
                                            let count = ext_count_map.entry(u32::MAX).or_insert(0);
                                            *count += 1;
                                            details.doturl.extrinsic = None;
                                            details.doturl.event = Some(*count);
                                        }
                                    }
                                }
                            } else if name == "event" {
                                if let ValueDef::Variant(ref var) = val.value {
                                    details.pallet = var.name.clone();
                                    if let Composite::Unnamed(pairs) = &var.values {
                                        for val in pairs.iter() {
                                            // println!("NANEN unnamed start");
                                            if let ValueDef::Variant(ref variant) = &val.value {
                                                details.variant = variant.name.clone();
                                                //  println!("event data!!!!!! variant = {}", &details.variant);

                                                if details.pallet == "ParaInclusion"
                                                    && details.variant == "CandidateIncluded"
                                                {
                                                    if let Composite::Unnamed(vals) =
                                                        &variant.values
                                                    {
                                                        for val in vals {
                                                            if let ValueDef::Composite(
                                                                Composite::Named(ref named),
                                                            ) = &val.value
                                                            {
                                                                for (name, val) in named {
                                                                    if name == "descriptor" {
                                                                        if let ValueDef::Composite(
                                                                            Composite::Named(named),
                                                                        ) = &val.value
                                                                        {
                                                                            let mut para_head =
                                                                                None;
                                                                            let mut para_id = None;
                                                                            for (name, val) in named
                                                                            {
                                                                                if name
                                                                                    == "para_head"
                                                                                {
                                                                                    let mut para_head_vec : Vec<u8>= vec![];
                                                                                    if let ValueDef::Composite(Composite::Unnamed(unnamed)) = &val.value {
                                                                                        for n in unnamed {
                                                                                            if let ValueDef::Composite(Composite::Unnamed(unnamed)) = &n.value {
                                                                                                for m in unnamed {
                                                                                                    if let ValueDef::Primitive(Primitive::U8(byte)) = &m.value{
                                                                                                        para_head_vec.push(*byte);
                                                                                                        // println!("{}", byte);
                                                                                                    }
                                                                                                    // print_val(&m.value);
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    para_head = Some(para_head_vec);
                                                                                    // println!("para_head 0x{}", hex::encode(&para_head.as_slice()));
                                                                                }
                                                                                if name == "para_id"
                                                                                {
                                                                                    if let ValueDef::Composite(Composite::Unnamed(unnamed)) = &val.value {
                                                                                        for m in unnamed {
                                                                                            if let ValueDef::Primitive(Primitive::U32(parachain_id)) = &m.value {
                                                                                                // println!("para id {}", para_id);
                                                                                                para_id = Some(NonZeroU32::try_from(*parachain_id).unwrap());
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }

                                                                                // println!("{}", &name);
                                                                            }
                                                                            if let (
                                                                                Some(para_id),
                                                                                Some(hash),
                                                                            ) =
                                                                                (para_id, para_head)
                                                                            {
                                                                                inclusions.push((
                                                                                    para_id, hash,
                                                                                ));
                                                                            }
                                                                            // println!("ggogogoogogogogog");
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            //
                                        }
                                    }
                                }
                            } else {
                                // println!("found {}", name);
                                // print_val(&val.value);
                            }
                            // details.pallet = ev.pallet.clone();
                            // details.variant = ev.variant.clone();
                            // if let subxt::Phase::ApplyExtrinsic(ext) = ev.phase {
                            //     details.parent = Some(ext);
                            // }

                            //  if details.pallet == "XcmPallet" && details.variant == "Attempted" {
                            //                                 // use crate::polkadot::runtime_types::xcm::v2::traits::Error;
                            //                                 use crate::polkadot::runtime_types::xcm::v2::traits::Outcome; //TODO version
                            //                                 let result = <Outcome as Decode>::decode(&mut ev.data.as_slice());
                            //                                 if let Ok(outcome) = &result {
                            //                                     match outcome {
                            //                                         Outcome::Complete(_) => details.success = Success::Happy,
                            //                                         Outcome::Incomplete(_, _) => details.success = Success::Worried,
                            //                                         Outcome::Error(_) => details.success = Success::Sad,
                            //                                     }
                            //                                 }
                            //                                 details.flattern = format!("{:#?}", result);
                            //                             }
                            // if details.pallet == "XcmPallet"
                            //     && details.variant == "ReserveAssetDeposited"
                            // {
                            //     println!("got here rnrtnrtrtnrt");
                            //     println!("{:#?}", details);
                            // }
                            //     if let polkadot::Event::Ump(polkadot::runtime_types::polkadot_runtime_parachains::ump::pallet::Event::ExecutedUpward(ref msg, ..)) = event { //.pallet == "Ump" && ev.variant == "ExecutedUpward" {
                            //     println!("{:#?}", event);

                            //     // Hypothesis: there's no sent_at because it would be the sent at of the individual chain.
                            //     // https://substrate.stackexchange.com/questions/2627/how-can-i-see-what-xcm-message-the-moonbeam-river-parachain-has-sent
                            //     // TL/DR: we have to wait before we can match up things going upwards...

                            //     // blockhash of the recieving block would be incorrect.
                            //     let received_hash = format!("{}",hex::encode(msg));
                            //     println!("recieved UMP hash {}", &received_hash);
                            //     end_link.push(received_hash);
                            //     // // msg is a msg id! not decodable - match against hash of original
                            //     // if let Ok(ver_msg) = <VersionedXcm as Decode>::decode(&mut msg.as_slice()) {
                            //     //     println!("decodearama {:#?}!!!!", ver_msg);
                            //     // } else {
                            //     //     println!("booo didn't decode!!!! {}", hex::encode(msg.as_slice()));
                            //     // }
                            // }
                        }
                    }
                    // println!("end event");
                    data_events.push(DataEvent {
                        // raw: ev_raw.unwrap(),
                        start_link,
                        end_link,
                        details,
                    })
                }

                if !inclusions.is_empty() {
                    // For live mode we listen to all parachains for blocks so sender will be none.
                    if let Some(sender) = sender.as_ref() {
                        for (para_id, hash) in inclusions {
                            let mailbox = sender.get(&para_id);
                            if let Some(mailbox) = mailbox {
                                let hash = H256::from_slice(hash.as_slice());
                                if let Err(err) = mailbox.send((blocknum, hash)) {
                                    println!(
                                        "block hash failed to send at {} error: {}",
                                        blocknum, err
                                    );
                                } else {
                                    // println!("block hash sent at {} ", blocknum);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            println!("can't decode events {} / {}", &url, blocknum);
        };
    } else {
        warn!("could not find events {}", &blocknum);
    };
    Ok(data_events)
}

pub fn associate_events(
    ext: Vec<DataEntity>,
    mut events: Vec<DataEvent>,
) -> Vec<(Option<DataEntity>, Vec<DataEvent>)> {
    let mut ext: Vec<(Option<DataEntity>, Vec<DataEvent>)> = ext
        .into_iter()
        .map(|extrinsic| {
            let eid = if let DataEntity::Extrinsic {
                details:
                    Details {
                        doturl:
                            DotUrl {
                                extrinsic: Some(eid),
                                ..
                            },
                        ..
                    },
                ..
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
    use async_std::task::block_on;
    // use subxt::BlockNumber;
    // #[test]
    // fn test() {
    //     use crate::polkadot::runtime_types::xcm::v2::Instruction::DepositAsset;
    //     let msg = "02100104000100000700c817a8040a13000100000700c817a804010300286bee0d01000400010100353ea2050ff562d3f6e7683e8b53073f4f91ae684072f6c2f044b815fced30a4";
    //     let result =
    //         <VersionedXcm as Decode>::decode(&mut hex::decode(msg).unwrap().as_slice()).unwrap();

    //     if let VersionedXcm::V2(v2) = result {
    //         for inst in v2.0 {
    //             if let DepositAsset { beneficiary, .. } = inst {
    //                 let ben_hash = please_hash(beneficiary.encode());
    //                 println!("{:?}", beneficiary);
    //                 println!("{}", ben_hash);
    //             }
    //         }
    //     }
    // }

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
        let metad = async_std::task::block_on(get_desub_metadata(
            "wss://statemint-rpc.polkadot.io:443",
            None,
        ))
        .unwrap();

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

    #[test]
    fn decode_events_polkadot_10_000_001() {
        let metad =
            async_std::task::block_on(get_desub_metadata("wss://rpc.polkadot.io:443", None))
                .unwrap();

        let encoded_events =
            "5c00000000000000a0573b090000000002000000010000003501d2070000f04971f76823b4be7cd4906dbb7da4c8134a458c2b8a796afa78ce465e0390ac903b741cd2f898af63c36c0932b291fc9b235285ea8e34304561a3f0b36b6f69464da131bbc2419737665f44325c2de1f16c659813f44132be252b243c8462ed9e4643d722cc26f42d7db80f5fc0f11895d98d30ecb40e36afa568daa313853be7b5b85efbbd32d1fefabd5847865105b8f7c8ccf748c549ffcf79ebf41635394271c604b7ed06f09f6ecf214200c46191e29c551ddedfeae91b00d3f4a37358c0ef3bfc89fb51f28e789a14694ae37b87c9d756b479838ea092d19369176d8f4802a55a455fa8e86c9d7158e0bdee69fe1c5c1819661093f83c057012abe0da6ee1ebf8c2e2c44124791cc5d21f624abe120bf5c1797888e0687456b7e8102a1395d71b7bb4ca9393280f3842b529792b4827c95da8a9bc519466f0b22417e489037bb0130c11ef5f83d3a662ee7a66ba4c5703cb66d938fad3f0f97f02c1a7d5a6f6f93200c4e4bfe06aee4e5cd6875892d662f4f0c9c5b73e9ea682f8488b3d991e2d6efe8df70c47a999f37986941ddb1f3b0aefd96b5b06d86a6e8c7c2070d04d0c3c350c0661757261207efc6510000000000466726f6e880191ad71fec688e7cec0daca406be8a44973f99b8c1ed6a4a8fc2ccda623fd9c6d00056175726101015ebe295a32c330566f7a273c40cc8aa7619277163290aef4fad29fcca787e745f448bb2a10dd4f150b1178e61b88026947e75ed6fd4cc3a78849a4fe4c9afb8a020000001c0000000000010000003501d6070000f04971f76823b4be7cd4906dbb7da4c8134a458c2b8a796afa78ce465e0390acd0b084401fb8cdcd96c871f65d13b8e3091a160d13c815c8ab12864650a6cc65c692a77cf0956b2832d3f148b7343cf8ed69213551b0f26dc81dba4a13c4f79fd9f73082976e32d83f0605e122b16277992dab0a4f40fa56abe58b389957a5c9d290ddef7e88606a99064c606f4ce4455ac337e7a67cc55f7c2599c27732864a32cc34070b29e48b38dbf5eb51b7cbdf6491fc18fef34835540bf2288b81794a46319928dd274a56868c8cddc0b2244d12a70c4871ab039b515cd7a630a95a8ab16206031329a396621888755dbefdda9b108efac72335760e6b7ffb68d09f5820dbec2b0d90aa7a1520eeaa7edbc2e07de53472d20a167a9c2156b9eab9650fcd2023533b56063918a70fe2b92c407288ff39e3c3cece77bd146ce71623ada411b29812dbe3ca16ab2a2312d73962de440fa8ef45ce4ef129976dfba4da39536205faeb350063d54c0f91a7a556908e83ccd7ae4e4bd98c1cfcdc94d22d5ad9aebab0c3428cecb5788ae920493de7e7f9775499db75bc35937f090a978711c4433afc95a9710c0661757261203ffe3208000000000466726f6e0daf01d9d8ef34fda38bd92be92842f38b9ab941617e6b41b640a2c2bcceee09b05e9075052347ca6b1f90f51a552f327515cc093b534ca61735fabef28b17bd3492844f4d286fe2944b790efa4c372a526933a3e0a5a26fd3936dbb0de1e887dbddf3db52dbce81d8ad90deebe14dabd6748a810d514a1451898df33c10eb931fa1d41584edd5db8720591da82b2a35b857cbeeff2df4a051b52be2f853e4a805d9c91c0cfd4c9a62ed2647118ed692983c9aafc2b4854b1b64974252421213b61c710b24dd9c9381b643a99cab59d837580091bb043107b88a6ae7698e6ccf07199c5394764ca437e2917f84855bfe7f93cb571a4d7e4193972a91cddbcca0b4795688079c074eca422bac2dd953fd2318d40d2fdea5565aa87975a5643332ed01a14254f59e2bed58acb31f043d8b00cb14e2ac9e2b30680c4385acf8eed7dbce2529e902c3ea7ebf9d688f27a1585fad26fe76afbfcb2fca32e9bd634e0482b4e9cfc2fffaa19fe8ef300e64153bdde509115c73394612f192b2661fc2c0ce6439ba7950b6e02b07f85d5b086e51559d32008904ecbd685397ec34ef5c3db0cbd3ba85c92c289e7281d43086977da933bdc52d20721729442de328ba575544e0a77321ebc8c7aa687091ce485f04ea22a871bfa3111720ae2d80217f065be6f12f043043cb1fb1ea10643d59e68fd7a58b3fd15211f3adccbadd8ab9614b4ef9d025063b27025c5b2105b21045979322599cd2da399d5e41a44eda2ea9f71da175fd4de7e5cb91cca19f1e9a2c8b63db98efb10f6a5464e0e6d791447afd60d57ef2287458c55dfd7a9486e85704cb70473e0ed13c946314c95f1ac3df29347cb16dd1e06d29e4bcfb91b0a42e72f84b2753eee4536ba66b0fc24c03ac67f4439c7bdf34f6d0c84cd26479c24ebc595b508f02114aa5916359ab57923ea1d14194f0ef2122dd3446fabfba3115f070b9b2e035f1155bbe0cc1597454f63e5ac38dbaa5a5ee6852b635052a8e72666e61efb0e9094cde5db45eff2107e8b40b9bb6ba37f632b09c94f2e343949d095d7a4b481ef589926808c219ee743fb60615ef72694f4383c970eb8b3cbaa5493040c011067111c24e5fc0bd05c5bba2370b62fa72ba4df62c7bad311c1086395a19b0060df97f858e1497ac3bbd2067fa1a3089b7ddd7aa65f6ca14f2e42803baf4fdb32cc99544d9b37ec6931282d87f4674232e84c93cdba42f8252b874f5315029490e6585dc42053ddb10812117575d527f25eefdd2fb9bdb12d19823bd313eb921532fe1f800fd393699673a7ace1d10512a91a85a29896ce75e2f51835653e2ed6f5057013e1ccb8755541abb7c01b7ef015eea1232453ee2c2c4d5ed5ecf23f35075b4b70c76a961f7ebc661aadca28a94cafeba99c8e12f38b3d1276ccd45c92cd4803c9a7174142e26195f0b4a597496183cd333f070022fe00505238f21df4ccd7a9ba11199e86bae74c587c75b2ccf63ed86f539ed31c042848cec31a57940bac67cbb653c3aba96aa64fcf7885a476dd63fe9d76ddfd42bce28df17220856e1df19d86a192ae71b67950ffc8cb3b037a684e9e4ba24107d73453a6702557c3e2481e0ac3d2e96020d131a54e5b3b4c4336b36456b950431d9cd52df58ab846a561c59966cc33e95a7cda576941f619b575494ce1db546fadd510289bb94209daefd1728b141eb711f761150963e1c80ec6ef400abf1197eeabb4d70e60920c15fd0fa38787f45da4edfacca0ee4c5c12f7128e02db262983e22c0d005c5cfe00a88ba73f814f414014e3b6e2e92de844bb597d795f8fedf8db226c0dfdc63e6b9c3086bb2a2c450a0dfe1b2adfa4a5e82da5c5ca4803e68521c3b5123674996d1bdb4ded10815f3fd34b30afadaf3736d833c717f9353363ef114558b3052783f00c2422f7b3a8d874d410d13b98244065143d6ad06249edff3c58335f72eb58dfa6880260e292e878ca5f4479639c7c71c84b42d34e25d1cf9cfffc551a5251b3287316862e4486680cbbca586ea215aaf831fffada977cd43aa482e6e1d8ea7cd980d36e02cba6e4a59cb70f8f214940a9a0d31fcf37cc9b85b58a0bf9edba56e362b1c161def391414cf1be30464b4a3d5b1418e111d5a0e686900f427dea3301337733bfac523ed582ba138b2fa97f58f8af121306277e9a7891ee5f2ec6e1847971a325ce6f27286f52ae9721acb6f7ca313960036a51f39e370a55d358058c434085616eda36045f138518dfc8cbb24cf693b83d3a6ad356d4625f6cb89b53f9ba93bb8a37f29c84740db632070b6e3bc617993272e2860984cc5acf844958e9a5847e1e89ef43272f2916e6aa58402b8b3d1256541ccf1652a1e7391eee226f119ac0d3d984762e1cb179407095153d6e36b89b95307af3824e35d80d605782cfbd4317324fe3dfbfafbaaa910cf1c092c8fb77df073dc9edcd76b90ab65e26dc61fbf0dea0d9ee354e88bbe0034431bed02e16319129e5e32ad855c00a384c1460e1ee1677db64a37ea71dcd70c53112683d02515027e0f8257c21f383c5992d11f30f860f415b91a60fefd68f1215ac1e35a49eb75199d9ea83788d5493d887295022817ae59d3dbdb715adec76e490a623ff13619aa96a3346e7147030a05053bd838a5941431fa365ad01c9be96346ff9e3bb3643406795e133ef16a23a449c3404fcaa05c406acfb1a5cadcf5fb24fee56e6c25bf4aa2f79c60579b0f911b0102c47a6cb910de137d987c69f41b74ee65fc4704b96a8a29cc85fefed844805ff68709832db47701da5841574436df7bba00e665b001f3526078a01e88b6a2853b0fa756e539a401ba123afa660a896c3c77221b7b71e7d4203cbef111a3f82b214d2edc782e544c5bdd79efbf31265f37aa4a00e14baa401ced0b3cf3ebddf59f728d350a6c02b64600f2503ca229639b13ceea2852f47191d47a5381cca9fd2bc56568e619ec667b901f36f7c0e6c8a1cded609c92795a21ccdf6ae8c8cdec66ad0daf55169f5ebf486b8103eed075a2544c58763e8639e9873c32840f7310ccc08946fd3f63259c82e48b6d5c433d82e10aa3dc671ef0dd76a587cbff362732cfaa5190c6020ed89e20be69923096405d9bab7905fc0c92eafa33af74b33b18f973a4771cf55f71ebdceb2c35d982532159ea5aca2a6c31bd3c16bc38abd2c9dd78ce026165592dc9d9503c5dacc9f15171c4fd5d2b20b523f8f89e80edb9b237d780c02dc4203c0903f9c84d70c6939b5ddb4ef7b787798a4ffb9ca7530b007c53477368363aff8bf1a71664caafbebd98a7156db4a59c71150c5870365ea4f6fe636b547d629e92dc9e199af0c83c981125d1a1e5389739e60db2c50f3483f140f354ae30b855ca30d67f803fc38370b89fa85641b613151ed7a265830cee4d0bee720ad314f37da30dff6e6524b6628f069078eaf748def734597ece5e493e365e171d889829dbd880968c87744c7810009839783877cf17cc966d706a459631d7614dd0a9c80ca97b304868b303c2703c46cf5d4e6635ead2b421fc9e857314269f84a293ef2d3aa4a36ce7de84f940dffffca6b6e34c205c9b3138e73fec08bc80ebf9b54d057154cf065bc45415e9c8adc9161346392dbcfca1ad6de59854eca58b45c81a7a3d8fb387ec9c7844c1d99ffb937e7eaca31d9eb81c66ec6f7b8fccd1b7b1adf9ef6085de7b177832546418c1387119c864d204d1000496ff551d5239b3fb3c1ddd7647309bf05777e58076abab355b046f942e57cdcfff98d7991a09c8fcf3dd4a80f801532e14a1a57a7095449966aab87695858c1c7d60696dba9a72e57bd6c8e4e8b9b9ea405ab6e77d6b145a26e3e1831961418fb59f623c28c394fbcd75ca3e27f277dd837143d0d4dc50c1c58a2023ab44a0e4d98a7420ccad4258b33b88c6f9c5df4fe6e45e8bfbe35357a1bf075fc648b7dc98f442c21686b930e4b751d20bcba63f3ab6ce55f577c9b42ae2872f45c0169c0189fd553e8572009e3b245a46021c325152965e911c24191d6a5264c9213f57ecf8f7940361b827fa73c878b96b60fcfa3589c047f068e5ad0bb18d466736402612c2c339016afee5faafccf27188b51b495aea49bc53cad0b3e6e4e81feddac93c9ebb7a1d67bb7621847d0a4cf4166465dce84adad789ab632c648d29722992afae94f64500f5b2e2920740981306bfd310c29af431bfefa7454ea3076bedfa512a7ae92f423f49a58d405799a77afd17427fe69c5d87e51aa6cacacfa9096f16e21bd76700f390e0834c69660b541d330815ce54cd9ecfec29e1b871a7482ac2a365b16c565239f69f2ffb23aae31b6768fcc98ffd9de1b44b1b52b10f677155de166f6813ec066a79f5fff6cf87095c03e60989edd3b14730d2a123554a7c1116491e36c632e2e9e129c524938141080c2f085ecc0ec05afc79b7281caa427d9e7818eb86d5e3add31f020ae4e880473fbbc1125d07c90ead721662bd8ad712bdb22e6932e3f65ba2946b1d3d2d2b3061e8360343b9cd57a92f4773601e770b1db6714d990a376bf6102eeede7ce0140947ed2a01a27fbfcc4f9ae39e72131ecfc43d4108a46959979d97ee1d373dd6207c19e0b38a63801a1460d6b024e03a73144dc8f42b5711cfbf09898d30716b4a57748b80d8616a14cbe747507975b966ec2c9f7aa896d2e29bc48af67d3bfa03d2534ca0c5cec8c142464af90bbdbe28cc2665f9e4e3f55246f1657d8bb4681557bcda608cb2d1ac1c076976a1e5fb977bce65b266d4218e2a5273e6035fa2325f1a586e180574901ff1981ad4b3e5dec3723df7170893500a64f89c9c11c1717146bdbc4796130ea38e51cd5912a49373a71a173b84c3e602a45dc4043c1595b30a4acfd5a6a3b07f05340ac09eb61f7629a4ba89aa8150987587e9c5077f1be1cf8fbe1fc14c050e8b4e4818498c9e8383abf2b15cd446ee5866e3b0aa1b8bbf386b12a3e7a59813856c651cebbe27d0f1c948cef5855dbf29f9ca9e5f61060fea2087795635a4e48a2f24816fb4bce18444c4e190a923a3f35b6ebbee5cedcb888fa82c9a2a92d3298d72f0a4ecc8d23db5e48d2fc611c704c61eba1a96d1df42193bc27b88755f182c40cedf0e780d70ca9191f1491a6d87ef646e2d39d018382d821913684cc63e0dc5e5ff12caf29e76928198074fe8d0350905d9d9106861ac4dd1285924b039dbb37fa10a3086de1409ba5a3b1c238ad80325780adfe29e437533685a84d9c4e0a86b03f3ab9808460c5ca26a4b2d374111a7fe332b396bc45a0be06cfb0597c326cf6731afaedb486c0f898dfbed65ea0a38a83c87c17a1dfbf03c0f3f17a827db9893be5ae214950793031a73c616125229ac3b0272970a5650c84d607b56651a8fbef1dd970b63d2a299e3a651ae34adc15288442ee3b9eb9fce7b138a8401f196fc10a0b44c100cc1a3206cf34a11d9b1d9ecfd25ad51a07aab54ddc7f5712764eb9e7fb319715f3f8d948379baadf2b3ccb71bbf71e233183fef07b9e58b7a1a98a6f9810f41b1aadcc28daf432ac9abbdad557b16971fad42367e70009e00f7ce119a962a8000cc7ba2c7de350b9b391636ebf56da6f3213aa733ad40be6fda17d7b210801e93317cdd86f0f677321d033864477f1e83248393de368d8986c13554e92fcaeca910e49f0161a1b9ea78257ea62273855e23160237b0c097adffd909a61ef64986d8079866c0cc0544c6585e7f5443a1b064a23d7347858e1c57948ef71a366adb2c92bc48f2df6cce3039e210f004d378c429a31bddee15837cae2c56d38ee6d8e3f40b650a9ae50a00870d72eb817fd4d92d68c287e78b3a9037e9a3a470beaadb8cda1c7da46edd15c46a26dc14da314af9b59fdb9e70a4de2e1a5d3c03a306b30ed5f2879d789a9f4a8dad6b1b318d106001199d8fbc732dfa77225346654ee8b3815bcdc067e19ad43a8f464f7e0f2b7607b0980aab84a5db5444167340ba30dc431b46d19ed6a82b36a3912328a1bc5e82415fd07f4028d7275ef4a3077ebc1df4ee48d0bb352eef10d191765e2392676eea38ee9e6c3f87bd0e0d71607420a5467fc9807a65ecd23430ef00fd8b62b3575613eb94d9f1b2e895c41ef8531b521209fcc0a9ec9417b1c407a1a962ddd65020379ff0602b0804fe1bd29f3a62969a30ca3ba5fbd7f617bd1a14e2352445a5ccadefe1dfe0db43e310230e0db0e4be938ee881ee6d553a8501053dff51a0783acb1c23044a46d9699ec0c09a9a89461a4d59aa95c927c103b2082b6308fe9509f85c4c8685950d6d97e0f52d768538b0b97e4dc433dccb9a3007ce023013a58cf605650becfbfc97df42130728edc58625d54de18b106e883d4d825f26bc5c619f0d1dcd967be822b20c69250febc3c53e93e554880830c25a14b1c9831d1cb9693a10a0f5a03925858b99770f2a578f7147cab7675998eb20d40659f7d8729c34032728aa6ca37faac622fafee07225ea39ed2f2263b898ed6a24e03d2a1f42bf9f22d81b50823b5c35a109aea124c1bd94a863c050d3dcd65e7fcaf73a744e0a6b841254d2e8b56478ebc55a257cf057ed6817d819ce1a78f48f12d8a60834a6dd0c263b06228c2ea6606959b48b34730911ce7f433686e88defc84cb5e505435f19066756a654bbd8ec77c5b395b735a26edb0b87285718c152481139758cc5e03e5c76f865f79f8c4819f1958ff3d700464a04f874a934d300b4564846d0a88865a306be98f8acff4de7d09e75aab1e219df355af90e6af04908e1dd5f6d26c55eea639b26fb0797d4395a2eb15418b08e32f03d5ca3ce5241a1e3f0842ff134573bd276f9efa7b123595bc4d307fbdf9f4dc17e5cada98f7443920e06ff1502289426696e177d93836314dcea4ce4d03809a12c0fb2cb2ebc2ba53655c2499bab152620872b4938591c1b25f8c41077f1c4a4d77d77adb75aa7be82cece95e94a3a5ea5599b5303593d84e56ab14f1b56022aee7253b9e58f5f82f215485711e0646d13a37f5f3e1cd194baf19f85fe8a46cb63f67296e462232aae5f314dde5681599cd1f1e319515cad710e77598bb5897526ffcfa1015598e0d7327db9e8f38b18de5aae251d561f690defb848826519b8c5d00197dd628867c1b58e9234a4d9eadea4892417129020a040251a6afb6fe2d50364607cdd47ce993cbf26f642078c0ba3b7c07cbd36ac736ef15f820a45118f03b7b883fef3d33f8e3997ebfbac949a00cc92b78f4c77bae1958ecc18dc41b408b46b38bf26d477aef00d9116771e0fb8bbb2347db5b9330853a091d201f41181589bcbec8886de99a156872f2a0fe706152afc8dde824314b090a5d25e0449c75c0c0106ec9a9cc0098dee82ce73861d669d91df5517a51c484c91fe6b6ceb3fadde596bd856e13819cc9c2ae40b45aec12d6da1958094872427ae97f0153af2522e5a0b6ee63355cb07bd1cc38928b3741f1377fdc7b6f81c2038260cfa61d3e971c04654678fccb2fe36058b00bf39354e8295e7438ec5c9c68d0d1a530be09a64b90b1bbbf5cf5c71aa68a5a4d21877194ade3d435fba539dcd2ac2333b7f6759f5874e51a9bd8a47b3af5fba8ae09a91b85d770dcc5142202df623bb2e5272dde595631d469ebd796803c37433270b9a8d1a162b03758b277446006450bf2921d869b72ca530893e2c9912b262a72268f282877898bef0e6caa1e4cd4cf9ea50c6e3a4d91ef27388ea284df349f6a41d19c85d8bdfc6e496078d75e7c9000bc3e8891de2b57bce304be1ecb06b0c5ba6ac3a59aea9f9c6a675af54a80ab9048b00bdabeb4ced065c0a280e0d0d7887eb90eb192ba54f716749ec3262daa924c208ec6c62d2b89def4f9524d3727fecf45736dd68f1354855363c648d0dcdb80c2889652ae33af96f7cf48d5c8e3018f670e019438c73289da1b41d9706aa0a4d2377766d5c0835177f47ee5c81c22069c846e7704aec27b398e356636d2e779be53151f86d6fdc1606d019b38d2fbf5e79edf84e86d928789178af5579320501a298bee85b9576f1fe98d3b9ee6f046be34d619e1325d30976f9fccd20541665a897e724e0df88f48b877eff57a842d82b73415fd44b748617c130f586e37da5addbee7038247ad57fd7debea7bbe1682e514f5447163cf5d44471c64883bc4f1a314871335e47648512c5af7f6612f1435cb46b8a29ff941a401be344241c542d94ff88af90b747865bb23c2c28e8888d972b8d444582d825a84e5d6b8f77ba3cdcaf9b00a014d556dd842693d0ee0b4cc78f65cf679b12ce4d1509c09098aca1c1504cd93b785afef00d0eb11040695a485d969e637708966b04dd7f31b740309893a2bfe3813a9eba642a7f2b5fe4c75cbcdc3d367ad863396fcae905193245dc8db5896f217a3e3bde69122a9673a86d7a45f5a5b7d8144d73d71cf7bdb7b692ae0bf455a05f2fde7b9b9b58b943d3b9d22a097600f6a6435e37e8b18e5017ed10500eeb14af36341659c8b640ed4a55f4df700810664a7c5079b29802d2353253ffc6bc008fbf28b9d6ffffcef0a9d8cf9989154281e3ab0efab6c1b2edd7a764c42e62a0738691606426d580fd0cac84a3dd0b6c35bcba508582d55e9b2e2b226803d6d5a62b3d1aec8e9562f34a4c67bdf940c9475d7c637c660f8c128ee1b2133dfa6053435cd4e94e83428c46cbac16da63651bd981ee31dee5be8b8c4fb537593ae01f76f5f2216df254a477a6999a99d335a21f951e6ef5ee19fdc46a6e2c64dbe1e85347175fd7adf757c17679771c28980487fa746a505a83369b18d20b3217b5df8591095ce38c5916b9879e287e45b67110a63500a1584b4e7840cc888157cfe944cd5fa724f6a13eabe6926e057938d3c86759d47feaa72b6301ca8f6d87f729703e4b3a70632227a30bf8e786d9319f8219ef775adbc45269fc3c6c7dbf6688dc1cd65392ac69c5d60c4a601d8e4af0edf9aca0d5349ff2cd8bfa666553961d0be61dfdeb3a1591a8ac90e5b93806b56c3008ab1477d48bbb67c3fa1ab4d2de93a8bc33c4215ae18238351177e75cf69ce6c229ff62834897af4169980f427a40333b5f6cb660b3eadb6a67e188ff49e102db878b6eec86cdcff503ad04890a7c9c1de1bc14e37e6758fa40b5fb1ad08e3c308495de9ab8c8c2569475090094a64572e5edb0f2d2a6a277a669e5bec060b19d2e903423888b3fb03910035ae3a6f32e6bd2cbfe296dbc68f95e384c83d5d6c75c59897031e95136fa94e01fe1d0f0db3697b96b5c4fe65dcbf0074f3fb5a7243f11c5a5753eecec0ebac93da341825be707fdaf69ef1bfdf161b768b0b5dc1d04962f7a1e1b8f0a76a526477a18955f3113565de44bf639d1d25719e56fe47995a30899e2209d5095739006a17355c15459dfafc54ed61f6ac2ca93000bd85f7c30d68ef62ce096a8efb6f49a834e3deff88577afcb7b4735d1dc5279ca68c4a677b37785e19763b7df73674c2b704344dd1a7b594e279b245cb528e3660ab6cc2b90135fa76f2bd71b67c739c18571f0dde1d1c0ec6ff81b89ba456b564f0c6e7827a9ffac4b5e0cec026915183bbdbe86e902815958bf627e92d553dd666fe5cfcf0b785631fe553cddeb803cc933ad194ffd266b33d4a16a890e4fd639d17664ffee405668cac917a545f6dc06918fd3648d4f85ab7eb7c4a383d2ffe558f0dfe432e4e481bc9d2b9d163a8ed88f018f1c37a5b1600273007c0099b0048adb0ce93ef73b59f4afd4aa0fc6dac34df9e36c1bf0985d84f818cb5ebd80297be4486851af0a3ec367b45c32303ad33cc2c5e0ae9bf0af1c2694acb19814c13fdb69107befee4a8ee00d75f4e8d034957a3bc86ee41e5545eeef1ad43a7ef322f3b6e3ec041a61f9a1a77a8b2a8e9ff61deaf302418cead39cbc45341fbb5bcd21868612b9af29cf9e9e6747e1b3965c250d0d3d0ebc955f9d4d9b7ed2b30d0ad67135993fea2fe5413d464e282b7c5ca8fdec3aa954e698a7297e28603ec3a25cd1592481327b7e5cc1aea90178dee7225ffde511e37ff68aef702f7d5fc0ea710be782f48b15e580a73f0b1b8334024f8233edcc7523a109e1f0a598e5deb1d6f8da145a964ee00f4e46af09fefdf680117c39fdf312420a36872dffd1310f32454dff25fe9db0a5a3d6e70e020acbb5add84d1c932a7ace7572ce7a3b6a6affad43354e2e3aad02f7d91a942dcc210e894647932d2b43b31d643861751725a4d920d9511bdd9a734e1ae0b2868d2d03ddecf0ce2501c347e3765ee9a70ea6aec65ef6fdd274ec35f19af61fc757c334e964db870fd52257853a8a71620667ae1826964e76b42a3a0dc2319d574eb89cc4e9f0e9e9fdc28b6f2e601c95b2becb7123f4dee4d9450383df4506652962d7287fca7339a7773411157b40b4479cf6ddfbf9162cd02b4a8c2aecd37b066ea41fe8d2895c1d595aee840b1a6b319ea61b16add0295e042c392cab1d439716fcb73aa730beb90969a00803faa3043516d837b411f2838f05a7c594e021dc7e7b8e145f8efe037c9586218a7c8ad800828cb41728f06786e180a9ce8eeb72f8beedbdb2009a0e2c2395b69cefa561fb94206e386cb919e10c2ffc05649c83b870fa16108ac0b1870224d4da1b585f587426148a36c7ffcb3506b17a35ceded2c9f3edb61c3d23eb807696e9274c660c083a700b3c40275bcbb5dba2a6464d88e6bb93bd965f3bd038578626d1a0e1017fce382645a8fefd7847026c6677ef82a4512a961dacbcb173f594c3b6e18a3048e7d66e9d2df44e1e879f4e4cce263355ae99b9dd439b5b85774a742754dec9a9709ff1a19ed90fd0340ce69394b6a326d400db4ce649e766608181308342b3c26106188d2b850ab8e6f443f6d3c4c259149c5d4418c72a25483c67a36199da0feb612eaed3f65595b9b50e0f9460c1c63193fdb2a0e0d3c07dd1cd7873f03701fc67206eb6c7b91650f8b7240bbb5a51d0e5967cc8c836b43b8ea68982cad85ab3a35b67a8a08c61b3590444703092bda49e617d5c36d93a7e65f9c0b933866abc447b7296248c9e924ae80c425d1ef1deb989a3c1b36fb14ac9de29707d8a2aae6dde69518fe8576d31f20923ea4ee32b78d1395842b6458e2574df02176f96c3782c9c2d6268ba45d8938eb8e8564863dd22f46a6bd981ea71ba379d6420cb7ea48b55ef3ce3836b8fbc0b445e7d7699975adab5252f04bb29a02ed6aa403b6a5e31f7e1cb7ca8f414c2537f4ca26c175200eb0ba4e795514a93f7575a1606e0dce8b58fe71c4ca48187af0406e8a0f91cdba326aea8c906866c880a56a5f4f4e8f05bddda67bdeed77da5f47bafaa4ef33aca989208740a084c0681775134bc6814268035353cc4ca512062e5307b174c7356660c9ac462101ece221a8bf37c70be761aee93289060d2e68e16ef3bcbbf9c8c51296b12b17ad5bf5a6f7566b2af20343ca8912261c62b080cc4811cb6c36272d9eccf1f2ae5f88979ffb547bc30969e624451c8c7c11e0c391c0b214a015d28d592b64143a0dc9fe03b64bbdd3eb0917b79331b206559c88cdf9144369ab77b0afaeb6bd57f7a7be387c890296c574f1d07d28e4871df6817c4d7b24362988ef2baa8228c48dfe32b9bcb344eb5bb21791df776e12c33067d48ae810001d2a2d67ee4479ce3af7f6e697fffe4291bb0cb517cb110110e3df87364d773842732112132f6c211c4fbc122b20fa8c1c8ba37cf8c06c2e3d6a4014abecbadbb0195382496fc663880b8629a727bc63206910516c60b039591dbb5660643001c350951035ae9eeec4eadb935b8ef5e9aa043553801f34a2054487391aac07dd288203df8f080f896bfa72736deb3022d5e9ea85f234bde4a293b7b333bf4d8b0644afbaaab4f8a1b03c6c8fb2a0467f8e9e28ffcbc113f956c689cc983cc32818cb513937c70a56e019aea9f38ec28f560c10fc1b6f04a0d2e08ece35e20320f21b37ce08e193470ac72634c941a6bdaaa01ae2c9ce8ed9cb41ac9de375db12d9591654b0c4e0e722f4e637bc50492afaebde5aae0a8eb5aa7e88e5169b1cdee50b0c2699c32ccb44dfadaf51f7da7e0cd53e2157cc256d21523ffed72a6c682373ba3f116ae1828bee298fd28a285cc0082884935d98f6a68c2777263d4bb9e1029c6e74fa8ed39dc6098194fd119a1f8a5f0d397cc1947bc58b40d23ea2de540604f9ea246e8345fd0f65dc11314409869e1a2eb3cb16d5b33857c364608a58ca9a42cc08f3b38aa8009ebca50cf737ade4f1299bf029f36c26d3a1735ca90da504cb735e1473fdc70096ac0895d3ff467597fb1ec3e84dd4f9c48910068d4d664b5a0720c80ebbf0366591c642824c2f6f3cfe4fda0c70f04543171ac8be3fd97c973a715952e5b32c79adaa8b7fff4fefa0cbae7c6abb6fc8bc54c861b5eab9f675d7e7238028e519f0097c2421ec5b16a0205f72ee9a887e3dc742484520fea44a275e9eafdd49cca6fdf2563b4be12d35f4a1a9fb65c52141482c92cf0aaa1191a8c8493c9b192db81157b2daa4027364f3447c21c2a3b9e78a5da7ed6e3269ae6e62cd492c5bbbd8581f542686e1234271e5f0bea08df0df70a496192ac77867488b6f915686f1b61f7940510ba6826c4fa7e3ccc79d39a51dd11ee8e19a100e24b3906a5b9afe4ce83ca17abfc102f909d292ce9f1e77f641b9ba2006ee8dea87fdc06f411b9fc0cd97760e93cd540555b7ff451a280564665ce6d548653677bda8720bac8feb2b3b9c6920bf25f566fbe19eabc18cbf370f3474dc317b3f149758dab39529076376324bfd8c6e579d8a7a2fda068a252c8cd1c87a6f34ee001cc62bce3405800999ff2dcec83dbb6d238f21feea79af27dce2af1398202aaeb4d4af5bbb47e10620ed3f1049752958f765b9e16d2c8f8cf5c4ae8569749a7c15c228a25772cc0d6806a21e3ffd3e1bfe4227cad9fed3350531d49c6b5b9b5bddf28cc9c05fac7af53de75aefca377cd2a57c8a5e67d8e09ecd15e33475f4030f40185de385f3f7b5fac223c057adaa36f9445572312fba301fdeff81ea19230ac9a4e563ab696b843838335c0ba42224771f38f55b10e9221d0f215a5b5aaa62ebbc9e3ff9918086fc056a8626b46bde57a7778a7a3b33e7f50eaf6ed9d8da700914d7435b80d14e3b0791db87c19f26d6e456410487e0dd4241ca51a7cbe19a7501ba424181ebf446d64451d85b3e93998358e8e2503effe2db34281c1f2e2c51188dae6d620a92605e7218526846b09fd03f635ed272c407fd6e7f434257b8cd654b1bb404ebda3cb7c8906c2537429aa243cd99fb376a96e5a92d3d4160ef44308cf9f0d62bd01eea9a2d2fdb1144f9ffa74efda465eff0b01d325cc8d94e8335a37e3cec52ffa68523c1e8aebbb58cd66d505c3b8f58bd12751413f6607313355b8337748e99ec0474965ce5e658c9e7b314e3fc18de870a66f6ffde6dbbab24c9f40f0c358b45c3a4086d0c36d3d23e5982b47978a1c5a8861adf5641e8b4cac94ec8bc5ca5cc418e7a05eff8bcd7002d131e48dc3bea343df807caf836216eebc0c68ac4339f232f95e240386965a67e48f3cafdae62a68c76be3f134450390f85a7a9e056ea06cea3d53cf041079763fd3bd3c29645455b224ae1bd6dfa7171c6009f14c0df9ba0df577fbc1ea9d79029a443b0d6e611a54b9039e164ccfa227fead19e9b5ff6fcb45cf6ca9e4fa4b2d6ce1a9f6e506b9a103446d2f48a153b4314e7f3024e202d505bf5edf43432dc5ad04d96e7f1e29c73c325192dc08a29480792b03aa8f9c14565e96b3450682862088b3968ccb30dc0b5ae32c2b5e9bbd96c7618fadac3919f97f18ac4fcf29ff20aa736504956a1fdb33d29612a816bbdd1e350bb6580a067be292ae7cde17db090ae20d5818c275765275671d3a8891d3426d7e15eef22fe728b7a06c8c28c817ed9af6b3c9d7cacf76e65860bc79a6aa2c6ba0384afa168c1bead38ae9dff3095e328171dc401e8bfb0347172aad3844e0f217665e819597eb6d4c1363f5ee5a217387e85b5a484ec8c2761b7e27bb3cae7cc2b8f0a8b30c3d71ab029046c02a55ee8f64977d2d26bce72fef127f45146a184213a1e6fd015a1b5b2adfc1a4b6e64f373d543add43b737f525ce9151290cff9bc3dc65dc52e9738a58487e5fe278db0b7b0bf8f390d4a290f132a1ac317d5f04b338c8087eadb6e02aa62ad19f94a4f205598d678527c129928ef6e3ea0ad29129b1246d9563ce65dfd8aa68242101e554d8df999a55db00c3432fd16841f5945c4e63031ad2cb148c856bfbfcac5a09182b82356f10cfd428eabf6b2a161cc55dfdfc8c746d1e563449cdacb81230e82ab1b8c0ec272089946afb0da10220e074037759dc943b439e8cafd515b879393713cf46a6989b081e61ec3215d0eb48f2fa8db838851b97a8ffaa5df6096bcf7c3b830f414163e6ff52b439ce8c3644c619b44b733bea01ad9e2e4ff7ec167ab8d74966971541aacf81f0a0b5af315b5f9e3494db4264ab8c3b6de60785eac487c59b41c4422e67e4c0c7941cefb1ec2365b5efbf3b419a7f38d85b24825c3eb2f4f9089559a32a9e77fa972c09dca7ac625976019920d2055b0214d03ab9b7a3daacded1310327a709e0205d33c4ee93f43ea8e68975fbe9f117d23ad41d0fcae6c24f5e656b03e7d253596857e8a1677b1ab53d8c3245c299bb10e731c3752158e568eff108e9ece26a7e1d5ac23a64938766e6e5580e918681b38f6c62b15672221f108209c9d186d6e4c2b216efaefce83710ca49b1a965985b2db3007c1b1bd78d07d65b9df9d3575374e3dc30421314d3877a2ed1bc3121d6c130a0d3ed85597b010f61730a849d1db02cdb169932b8270a1465d4bf5a02a61bb5a48e2e3d4690c5a50658722c752053a697e677e55a17fc45be2663f825c48143f0e674dbfff7eda223881db96dadf790e5d7af267121bac4fb242c4546cfc1d9d8ed549f5f5118320bc51e9a5d8443b205218be8d98eb8fb23625b2673f3e716f429697e835ab7c307fd1b9c2d363041913a55f76352f51711bd4024aeed4083aa03f55994665eb7c87129f655cd103aa7dd7871c11f688682a9df0b550b878383e8cd3b2f5a2ebc163e2846056418fe49efb89f4391fe8fcb0ea619b8d67454b0936b8864de570d7ced2a0574a4f26d4d9dc708d3e0e2add58b1636c96adbd2fb54581a0cb81299526fbe9c043236813be3452841aa9e52fced780f7b44467301d7f73c9dc3b9050c565bd097e35fe49e3c087570482ae04e821234013e1c2e576f0a4583527fce430433911d11e93ac1c05def252aab91641baf18c25c491c81f8d82121b02e48a06b53925313faca7806accfc354a11e68d65fb813381d875c03b01ed5886b032a158bab074ce54437187b5f090e5eb2a602d4323d4e8ab07c67707ae58c6c1ca03e96ef4f4a4f68efb9237abd22b9222b27b0d065aac21f728ac61abe984162589268bc8bd096a1c01e35dfe34ac58f9bc08aea63dca6380e596f3376f0ecb248fdc789198652c194a9cf9a014c817b8835185d269ce93b4dc310bd6cdffce0ea16b2c22cefad6b1992dd20a2ab0d092041beca76b98cb742d0b61cb53c6a85b29b8c322e841fdf24485f09791c39f035a81c4150c882c1902496e07823170a159ff5a4107790ad436d55c586f1ec45d3685d2efaf1a1f6fa24cbb43233d9e9f24448924e49566da7016437805b8e11d712eec65405622956eca5037b5fe252a16d5e14c2056175726101010c393c525046dee77d40dbaae6c5a5b6cf084c860c2f326705a4086120658f4f122c6271a1ff54e4eac4f5df1148135e3a169eab1f1368fb4deda1258ed0a88c040000001e0000000000010000003501e5070000f04971f76823b4be7cd4906dbb7da4c8134a458c2b8a796afa78ce465e0390ac9e2366f04fc799f078b96841fb2ed03b2c42491d4b4fc138235181e1f5950124f711a43ee5546d9db39551cdb288c76a3bad7c1d71bb0cd4aabd7ea502af1f272d4c7463e3a8bdf133e873d39d355785410b94e9473b5e1b17c98609a156ee0fd3d1dd71b1396f94a56b12c0fbb8b3fe0f23383565603e8c8d9401364baaaf5078d36076dddfc7ca31749f15430aa2e4267093a1913164cde16bf1c8be50302f5ec005fa8c5e86c77ec6da5b3064efc7f46fe8d88485f0deaaba40fce013d58b64ee1af74cdab863517dbebcd0447260ff71e94d2919e6fd28673b2d42232e59f580edc8b2c4f493ffa410eb67531744c52313a7662994c90314c160d7dad91ef54658f3e07f13537abd042e75ddb0c756c0580f80d9747707850a176e2124858103191af735879c64149260bcd5843c6d369fa1b25b85d5e4a9fb2011dd031178721ea811003906d290f8c3014920b1b5ac2759b1334f96b0e901f4d208e14b5a8ccc099c02848ffc4444451a233787bac9df884d3bfa156f6e1b8eee7e17cb0896f77273a30c0661757261203ffe3208000000000470726f64803e9a1aa6cf1dd7cf111fc686bed06ef72beb346208cbf1e16fa7334d8fb130690561757261010194107a0bb2300c520f469525be6e972a9ea3fb3e69e2c9d3379ec0127909aa3503e54b57de5188a8a121934bc9235e7e81a39194c683a1e81be8a8a1f62e508608000000220000000000010000003501f0070000f04971f76823b4be7cd4906dbb7da4c8134a458c2b8a796afa78ce465e0390ace8ecc7d3bb7cefaf04a31274fd8f8a0a9e1d1ebc95d824f28801f83f8945f444f20bcf16891fa0fad6bf73295bc74f00c311c95bc4a74f46dc0ca744fbaffac6844a8883183791a17a0a52c16604ce87dacc314031cb4c49293c00c329f96082c78c3a190cef9953494ae0937d435fefe2a13f1c81e159bc231f16affe2a84605cea22141327596522d06cd5784163b4e6f95a96102fb3691fe1a2345e8d2c01826f072027af9c5ac0bba2032c4fad2198539ab8af86d31bdcb96a4b2113ee87c9ab4c7013a8e087a3744d729ee4b3401e47a819c5fa3211260e4972c1c025da878fe2c8fc4ca553a13d4814f37bc478c85d9148097cf68949e5282d19d00ed5e03ce915236f7acd36a2c05740e29173663a7c7c8dd9d39a4c9f14d905fe3656e902815171c229313e702cf2abb696213c641f1975dd7cc1ce871acb83e954255b86fe680c00fac52d18476b06fc19be2289b8bd37f8bf557249745998faced3a3e06f97ee2c2c8e17c7f0ca8b30bab559e31f7283a50d4dae447619459f2493be04a8462584080661757261203ffe320800000000056175726101015675b4121dba62fe9a5eab632897eb0e4073bad618d910156e22c920aa39401c1593654ec3adffcf2eacfcd293860f065e709e0cde6fa7b371c21ba08bf4c0860b000000250000000000010000003501f2070000f04971f76823b4be7cd4906dbb7da4c8134a458c2b8a796afa78ce465e0390ac6249bc37357e2caa32a7eac1f11dcc2a7c4a83533d2b7135a489ed23ed56ea59eb0dba463dfc2fa393264000e6c02d1b88d8090d059532f559d921b8a8d2b1a0c463939815c9139e0250c22d4415c1568674effb0167f5dffd8f4ce30a55e159e2cc19c19b35d5e7ef8d2e5ca05b79b1662ec69dae422de45137ca329d61a2ac2e79ca827be0869f424777a16729ac52f0f302e54311796ab917aee243c52e3e3acfa2bc331e4c09b4e934fb8b9fd6d34ae7ad245d3d82fd2de912bd918731856ae3d3d510e50ace6c36b3162b756f70e2ffb4ae8daab30bcac8b8a362f5fd3863d38059448511d9bac00338e32a61a88c352a9585d18ed1d59cfb8d78cf83e960014015f505abbf8daaa6d821d09dc257c3af36d31c7c8c078de60baca94f97e902fc2be024903c375d09d74f5c771f94b0c03f7d8d2b631927deac320eeef929333ece0e00baa4c7f38e3ce9d40be7b97e16a20a30f097148f09d1ce04cb8ab73585b4e22454ef2d69ef94291df99bab02eff024d4e8b2e6efc1fe0b762ea0371efa36d68f080661757261203ffe32080000000005617572610101042ce8fd97fb27a916337f39b0234d62fbdabdd21afe66308823e12aae7b387ab5594d905258523b32571722012dca52013bdf8f9c3646c511efb7127f7726800c000000260000000000010000003500e80300006c51377632106ddba379416f2ef5e24450e90cee8c78a70daa262b2ae6f27e931e70cbdfb8be51c4ab07b76094ceffe24e0ab1e94c5b1181a8a55e9ffba321723eca8f6323e7145f85b6a6ee1cf5a76ed3b0b825e70f9ec8c103fc87ec1d41bc471a6ca0dec2185fe53fbe685ebbf0b5ed34e8f363801195c538cce0c1a7a3b13670a7f5ca62abf077b04a186cd85ff7e3f224dec28a894656ba5812d29f3d9c14fa9dd4103de55393a67ff430b2f83214c1dd4e03789a0b3162dd5b1b788e5013035f178c78e78d814359d472bee5b86dd7ae4623ec934fd1dd1560e4ff6185623955d1e6708279249f774458c03f99a3ca97a288a23ca958f007d5602ab82082a7312d98e4ae4eed52be99f781b669ff9a57f3ab451b5c4cfe6b35601caa2b975b8f6b0a0896114fcef260f4d821f52b536e82fd9628ca9a3575f604a612d0e902eb98527fe3bcc1e89e37c3377397826eef95043da7b0577088e641ab8b9737ab625b450087da8ce30a912b6040c346daf953f37bba1dbcc8fce369e8c985194daeee2bd75bea8207c26c7e84863ab9b94c0fa13122c51ace8b21e9fe4c0437124996f45f080661757261203ffe32080000000005617572610101208865233a426b20c8634570a966804fa4f972acf4cf15d27d8f994f18362e6f4d695294f23a0e065facea8f9ecea21f9e37bcdb3b374db89e986f0e7c9c8508000000001a0000000000010000003500d00700006c51377632106ddba379416f2ef5e24450e90cee8c78a70daa262b2ae6f27e93f65e9d4919a33689872aee620fcd869729f6118130c9c2637ca28440670fe54dd3321d3a2104f586988c422007a0b78b79aee49da6fadd08bae423e7cc475f4d135c0d931c6cce4075b89c509ea90c8ae5742cd3acc283509f60e49aa122e139f52b3a1690fcf0b321515118d1d3b79ba2555effbae03626a3a5524b21576c047aea82a164a8fa4c41715d1a5000b44e6558706c6d4e34d38e550076b6e27547e8238b2327ef4426ccbed77799e381178a9d360b4fa7b9f969abadef8e32d089deebdfe89b981d63491a8909ea24ea94ac365f0c66a1bd261e8682653418a30d2e103ca967b16d0938084828ce508287432bcc12d71937c11cc2f49d932a5ef8b8d060a2016f89f7d8ef678885b2e31c731106bf10e2d8c0364a9f5185aa261ee9027b5bd7f0ba96728057aeaf41c9713dc5ddfe013d010d4670ccea9043e344d2ecea0f35004474e94d9735a5bb00dff1c6d88212e042fbbbc0b6806830861830348248d9c8d2de26018605c5223c7b65e2e57b14a665d8e05d36d634d5622f50d6df49f6ae080661757261203ffe3208000000000561757261010102231920fea61aeb461e7a9de9c990d588390e7ef2f433d25c0342f3b3f4f319ed46c243d17bfeeeea9f83de9632a88f697f55e4a09fdc6f67057f9498b1028c010000001b0000000000010000003500db0700006c51377632106ddba379416f2ef5e24450e90cee8c78a70daa262b2ae6f27e939c6712de13e97ea5c981650760b014e280681c456b72fbffe332c0873ead5926567fcedd0351c51a31580ec401f4abe0aed860e37e62f5241bbde647ce15eeae47926531b3d833b92d669f91d0bfb747fe3a5a9a8d018c9d1804ecd26aed4f5561971d7d5706a307c02d49f527d8ad1847dadb903a1c578916c57f4f1ff431d2c4bd3b41e605dcd7cd1e8d1326baf57bf5b74c3ca16290bc2b5d3aaa90ec975e16d57ce98d7c7b6ac6b4cb394e85f309514113570b1c05245b2283b230f93b8d65939e180190ac2701952bdf174ba69b265c2977045f6a6187a0492676ca1153d1dcce63a55a08b397ab69a06124332b424ae8935871d3c0e9c7bff19a6370f1553a86a694bbd974041c569c90991ce429e8e53a38dd8337b87ce4b44b64c2c3e902c136db6ff28b18408f5f14076764f76d424bffa5de5a893e8cc4550129efbb0852960600532a380b1cd9343b3ba46f218392082a9e2d5940c0f2c8528d4101287edaab17954f93ecaad979e418ba4c7d9adfc5624a588082e647c55316b7f80945935f3e080661757261203ffe320800000000056175726101012cec396f631c6bdc45381610ca9b64b1589eb188495501e1cec8ca2b7c971c1be10d2ac551fda1bcef9738b4564fe1ca0fbe9263024357f15b59670dfe9d1284050000001f0000000000010000003500dc0700006c51377632106ddba379416f2ef5e24450e90cee8c78a70daa262b2ae6f27e93747cfb1aacda7206a125c66dcd4b9026d2d9d1e5aa253db1fff66ed7bca3766914210bd32b13c04e721189394e86958e1c3a2ab33fadfde0ab54a098e7533fd8d10bf5d7af57b0db2caec0f1f658dadbb320f96d53772c734c9b17950666665bb1c378fba38ffe4e0fb3069c3238be0aa14c923e04336246cd31ea4edebaab28d4e8a373d2928e034c5186b04ca4bcb63a703151c76f9eb5042eb9e4aaa49469095c6e1f88a93fa0b9604bd06472bb97d56488c907248a4bdee3a4619d9cc88315b78b9638346476db286866bb0230fd3e0508c7d04d4079ecef1c7a90b99c898ce8224df71ac76563a1f175ef3f7a2fa98e26f04af960f511ce0df796850a00bfb226a3ca9beae375fd1b3bc237c7b607ecfa031b4dd2a473e9560d4e955d7de9028d1af7fba7076e80b22c6ca3a12f90e967d5ffa1d107af22cce75c51fe14ee4cb2a131007cfbc9a34ee9fc18972e49cd412eccc1128793c150b7deded4fbbd9de2a0eb468411db287451b1e32cef0adf58e7fa8edcb2b05470476902f2bbf5e50bf9986d080661757261203ffe32080000000005617572610101020e289a9c5851ba3b704368724077319a2bd312995062d2a131cce31f05ab0cf562174a6e5cf60108007c17b2180b14d61eebd1f2a3a421c1b64fe79276138206000000200000000000010000003500ef0700006c51377632106ddba379416f2ef5e24450e90cee8c78a70daa262b2ae6f27e9366b381c4f884091031c28ef5bb1b8f5e7a2a17b8cbe2168bc467a9eecdc46d7362dc467fecf359186ac1dd3487948bb67bfda8cc0c8231af335d18c952559f3ff5559e448df81fdffeeb58755c6e3eb93b6f5887f0ed2adddcddc23aad7c331250ef3e81556387e237d3550a6d885f6a7d39849b07629edb71050e5e0d0b85c7ecaf202f36329e840e00e0b88a1cf7c3036f22174903596abd41e252184bbd40875d93083d5cd60e77650e711fb19f18bdfe6ef89a75a807a25dd2607e40d28a9a62126564e07a47f77bf2dff52bb9fd44109751227945a6c7c310be85eee28d024aab592ae76eaa458cc18f9a80819fcec1caec087eb5263efcd4a1d163facbe545f736d071d44823ab5bf8a513bd6ccddfdc4c37a800fa5b3389e8f606ac73e90237a3717548a120ecd6d3ab55f337fc8dd362a947ce3ff24856fea53dce4af4db4e5911007bbf8bd957bf579c58b4c6ca84786d2db8d1197e0e5b490e9484d6ca086b3f138c96dc874771ef57a2d055bb9935e6ece3099db73b4b334b5736864fd3eb514c080661757261203ffe320800000000056175726101016e7e4ec19a22f91f7edfddefd687dfeb84dc14a870e39d2ba4b47b4807864762fdc264b17ba184b7483b1a07051722d7fbaccfa477777ada1948b18978556f8f0a0000002400000000000100000000006046fa148600000002000000020000000508023b58b4edcc0ba1000ae7b75552774be47075ddd547ba9c8d297314c98d6a36d4460a0c0000000000000000000000000000020000001a02000002000000050281f06d1ca5f7094e8fb7398ca7c1af73310015b25b22e144ddbe5dc175cd26cb6d6f646c70792f6366756e64240000000000000000000000000000000000000000e87648170000000000000000000000000002000000490181f06d1ca5f7094e8fb7398ca7c1af73310015b25b22e144ddbe5dc175cd26cbf807000000e876481700000000000000000000000000020000001d00000000020000001a020000020000001a0100000200000005076d6f646c70792f7472737279000000000000000000000000000000000000000043d2a109000000000000000000000000000002000000130643d2a1090000000000000000000000000000020000000507000b93d72dcc12bd5577438c92a19c4778e12cfb8ada871a17694e5a2f86c374917468020000000000000000000000000000020000000000803fe83e00000000000000";
        // hex33992
        // bin16996
        println!("hex{}", encoded_events.len());
        let bin = hex::decode(encoded_events).unwrap();
        println!("bin{}", bin.len());
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
        // println!("{:#?}", val);
    }

    #[test]
    fn decode_events_kusama_10_000_000() {
        let metad =
            async_std::task::block_on(get_desub_metadata("wss://kusama-rpc.polkadot.io:443", None))
                .unwrap();

        let encoded_events =
            "4000000000000000f8c880090000000002000000010000003501e8030000970a67f13c63f060eb87777c4b68a4493931715530f8d9f69f6ff0ed87918844329d9d03239fcabec65712c2d49eaab86c6e99bdd228e7abc90f2b5223fe441159fbb85ae749506da26bb91a2a1ffd5a06e9e22be5bd0925ae68c2ac0bbf6265b6038ab854eaa4c8067f318575870d14a1692ade681ddffd9e6f37abf1ec11850cd1fc4ae08b277eae64be8d85d2165be01b04505a72e6be823f1b24df628ea792a73b0c4bdfc952b5d7b05b9f48b63a6ffd6e350821893f285f971ba94a0e73786251cdc7534c2ee471058f48673b9d934c33b4f20259dda40e8c09bbeeeb824ef7473442e90cb811e48c83a18c5c230e2c0da4a1ee565e0c1e758cc62bfac8671aa5730f5f72e4b17a9fcb4ff7dbfa006f68e01f280873eeedaaea9d325636d298faf2cf23958355804c94691a610d792f5c518b1f08fd78fc95fa5d202d21e90246c627785b5f9555633b5015d54a823def397ea3f6ef7033f24b4053b15aec1e5eea4300c973cb3e159225a21e3dc266b857f6f8c82e85d4fd5df91bfe6baaa354cceb7ce4da5629ee687ddc8d3906e981c901d56c5e8715edae94bee4e5c35af63645e10806617572612084b720080000000005617572610101849a2f00f099c903fd1b776553d57ca1be3a02ddf44546b6bd4f3924b41b1e3e08c3d6c3ef5ee2c44466708b98096fca2af94d57d70d03b6d8131ea7e6832b8f00000000030000000000010000003501d4070000970a67f13c63f060eb87777c4b68a4493931715530f8d9f69f6ff0ed8791884442ed128f3070d8c0f4efa2cdb1d676ceb4705f8c6bc6a3b71a6f1b0afdbc100086b57c8ab049fa13e2d6983afae568923575a336a70f87c05c544d35618d455e188ba822f58033031686f1943ce828b692228f18e48376df0a5dfb344e68184dcd6528cfd9e1104fcf8aec144e25bf1c0b8b790626717f998fd7b46124525d0f367c58adefda1299cc62c2ff06a9a7fcb1db3144192b161a55205f9e808ea902e2ae9f836e73650aaa5ac6c9bc505ad4778a38673545bcaa159b918c8acd138b7d448ffb20374dbb6301a1029ab8ab1504b12c449d00d0ace2b10659de1dcead35159d29f1bc7ca22f86300da98e02d8b101cc2714b4e6681616aa5e53c51ebbb239c7b99e6c929c0170442d1c29df94da60a8debf90af7a9d2777be0abe9d24e902f8be335bab1a6288460d7368f54f15df926a9e474a5a3a13dea9824c49e54db4da7f2a009de56bb25fe8e12a18771592b8eaa6b524fa494922dded016723f93fbe82645e88e3563ea816a0f97859cc73b31083e7f2ff1ef911e7049773eb355a4756e25e0806617572612084b72008000000000561757261010130436cb29d3fd4ddd996b078f23e28698f983609a60bc51b19333b96d235896c9c2e274eacbec55463476c8efb51261b5073871051db165e1e8ad5761aede78f0300000006000000000001000000350125080000970a67f13c63f060eb87777c4b68a4493931715530f8d9f69f6ff0ed8791884462e2563cabc32bfb099d3d296f99b431b18b74456ec785c65dafe2be800e3f421c9760f2a40d129d0de6ead24a40feed91b3bdd789ffe1fae76dd7a550bdc6041783bc5fecbce3fe1d823e89ad76640af60681b9eaf0539869eea5535de8ed03d87a6334d5b5140c176ea0acf80ecc5d82686924886aa8cc1de55ea8b5ceda7f84e5935b347bde0d4824b8dfeb95cbba94234fdf19e40bd09b8adae361de842cd4d368f11aad8f76598a8af0c9f870ffcd1d9f4381ef714c3b519eced963c287de323dba5a720417911226164623153c1bbdd3f33916b9be07364376eefb76c491d949884c9bc067a7cfcd2ee808e89625035c5b1092ac483340957277bfaa4b6300ad5264de6e4d4e35a6cf24332a87be8f4f9dd7ebb34f5cf317ea583c4358e902cc3db54bdd3e0da6c288f43f02dbd808a29eeb10acc9b469e4dfacfe3060c9b0a6300b001d7d784e48bdcdea1ced937f7d2565c08492cb5519d4c46399843052474798c1d943a0c9693898c92ab55625cb3f889f26e6b8ed47272ed76cc3f2521c7f21cd0806617572612084b72008000000000561757261010160d4877ac5087983c1e2d7048a42622ca727835b2ae123cad93b192b0b4db6126cb8db5ed6efe8e22d5de6d8244c30bb177b34b6fefc9d7a99671e28452e668e070000000a000000000001000000350126080000970a67f13c63f060eb87777c4b68a4493931715530f8d9f69f6ff0ed879188442e5a70c523364f4187e3873e9b494552975192f2d6570f332ba0b819ff128062412fb9b1343ebd65da1dbbad0f28fa74f3138eb1f5ac42dc4b087bf6aae9c0f1c10428db12c688784d12425e257f1436f1a13e12b0fac7e09c2ef782884fdae288a70a0827669d8c4c1e153f9699157c249c91fd171288c1c82c343e2e5329052a2cbd6da16d035fedcf8286363b5cfff146ac95534b1eb9df515757ff41473f899ad3f78b12ad415a2fb3b0af28eed976ef75bf910c63891918463ed196af83daf67887703f0d72a942f0666c82a6ef8df9f3419a7c7fbd9c021c76deaa98431d842461610b3b08c6ca9cf6a102719e6e67c672f449fb8ffb3ce6e0af3236b045cba2d8e3f0bc713d27a97b6868ec464e32b9e1ce40729f08a242a8c0749e46e9025dbc8c945c125682246a5a3a4dc4f5db1071bacbead573fcb36e20406d5e95e926e31600ccb6952f6b8451707882e07dbfce85a875e7204744c1f9baee14cf272ffff392c5967d1da53c318c529c319fef924106be7b9a8ed6aafd2d96c9c133fefc63aa0806617572612084b7200800000000056175726101019486d01932086dd193762964375e7bd944eda068c51369fd930b79e8e0437d209f4238bd0431b3cb61be4ba663bbbda1a53730f13001027ff2adc811f3a35885080000000b0000000000010000003500d007000052396b4d81153bcab7f383bf870c3d271b19e35fc834fa7f89fc0baf19823eb3c45e3f3817cff0461c43385de165f5eb18b6cdc9147bf2eef3f4ce99afc7485ed943f09bde7d796ebcf315308e21ecb1f2c61f977f8a92e0dbc0105c6f963dd95d6ed9829561c39d688447413ef6c0372df019ad69f75f58402eabc60d646224109eff7ea1f7a869883d14c2ae823ee289393aa2b9358f7d8ee760f316c0196d24a0dda1d29558bc5331036721a5111826c6efd04104fefb61d3eec27bf74d2f6bcef1abe2b44bbcfb29552d3009901f05238adbb7abf10ecc1e354ae1e70584ff42916291fe587927d39279e75552858ea30f183aed9d840848f16ed3d537ccce864af51bd0b781efb619d2d7f5c360dc72c15ea0be3646823c62166fe6fa9f3bcbfe68574af4d8876eb0a4e75c6b3a1d15222da3ff78aa069e3da0ffd5eb1ce902a133a650f06bb019427e57dfba29d2bf510c26d473a5ebc8ad31acfced1eb1ce6ae236001a2fd43761f8e9de2527af8ce4404fbfe612819e7d2b6c73e6dcc287c9d30ba180b7a1125b2b6818f64c9c6f350fade37ff515db3ade66540351bb0051c9075d0806617572612084b7200800000000056175726101018800ee6efd5a9834a93f60e9ce7c124107994f6a2b23f7cd870d1a89e2d9720f41b5dd276dff91cc15fa06050ec0195c1ae4c09f925659171706863940bd3f83010000000500000000000100000035002408000052396b4d81153bcab7f383bf870c3d271b19e35fc834fa7f89fc0baf19823eb3a45fe489f08876dd93330850f2a34dc246de42c5ba0efd0b51d890ef72775739cd1d18b75d97821c2192669a1b1cd9fb4caeefeb2498d2b53a8620e96fa1a24cbc67f2ee28aa00117d3ec90f88baa6f56663574212bd7c08015ad28733f397e7e4f3321c654a88f24a51219ea984e907122ab162dbcce2bb6a977c3f74f74bf7306320a43f4d10de4a00a454aee0918ed1619acf1101af3066972e74823eb458e0982a6a8397b8cf7685ad308ced6adfd1fa49216211ba7425e8e018a179ca81ffdc72f87fc34c40a529809c399cf99fedd996370cc772b5c51595e0fb2f7dd369248ec4a86bacbc60bc4d65df89ce7dedc436fa8a3a1934ee96f74de1af145d80e7b186c7085b0e369ede9d73686ea83a19a3a25005d9c11f9e4a9351df040fe90224132a3b1246d7efc37e373f66a85ece7cd369efac6aebda6dc7a13eb6089a8862bd1400725d7a343e29444c67f393be4212a8df930a5a7c223837f4f867e487005c88df943ab103cbef0125518e5049dc8926ea696778464a8c6b59728ca1e394af36240806617572612084b720080000000005617572610101a46b0efcd4328030c0bb20ef440e2f63364522ce5a3eb676de601115ead40345cf840fd60da25446d1b0bc87f1a96cd9464a9d520db4d8605d36e3fb08254286060000000a00000000000100000035002808000052396b4d81153bcab7f383bf870c3d271b19e35fc834fa7f89fc0baf19823eb30098a000c36260ebbcb781b8d131ed52358675f23ec929de5f050ecf0c5c9d0aa6e713246e85a5b1d6c51a674062a1a2a0cc75bbb13d25f1cda026e9b4b701edf8d7147305b71558bca2781b5598be832de0a149b9dc304487e0361079b21a9cb5d04f6939b13e7e6c0e73b2a2b0ae9d1301144015a40fa2417007df2b482d6f9a91a41bb6dd39e62666786ed316962b94b45d867ff3e8b55322109f0525e259d67cdc0464aa52aa728fd21f34c9563d527b2e3a911dde4c5c84746ae7ac9c8f8fcdd64c63856787d8d4cc66cb1c49ab1c94491eac066330b82a433db65617643cd446c9a1bd71d0ce47495a92d001a3047b17ea1afafade18e5892db4ebce4748f5790be22f893695b5f019a1ca9cd016a88ad8423e15e302a77a7433cc8010e9020d9e2b85621d5809954468660eef7dafb0a3e11770ff21b1f6572a279b1de2108a100f005ed61ac8275af9f9381c71dc986b9f87c1af2d9f1bd8ecddd55d4a3eb078a0dff9bbaf689cdbfd11dbdf1fb15f1b005fb946571c7e2be758f7bc1246eda551d10806617572612084b720080000000005617572610101aecac16fb06ad772b90075039210f99b80259a51a868541b17ebac4c418450068df560e35e8f219c54a3ec819af49f25b75a09defe27990fe0e62e5676894089090000000d00000000000100000000006046eb0e00000000020000000200000004086fd30ccc612a93f7c1cb592ab90b31984e3280230eb67c19aeb84ca7b34715192f891e0300000000000000000000000000000200000004026fd30ccc612a93f7c1cb592ab90b31984e3280230eb67c19aeb84ca7b3471519e5837ba18c94d76d165780873a85da12cd4c7e0715bd9c64370ede7b706041fc00b0ed347f0d0000000000000000000000000200000004076d6f646c70792f7472737279000000000000000000000000000000000000000025d47e02000000000000000000000000000002000000120625d47e02000000000000000000000000000002000000040708e34cd66129e197697554aa67af8b35d1daf4f6ed431afa2701bcbc320fe2070ab59f00000000000000000000000000000002000000040708e34cd66129e197697554aa67af8b35d1daf4f6ed431afa2701bcbc320fe2070ab59f0000000000000000000000000000000200000000006812490a00000000000000";

        println!("hex{}", encoded_events.len());
        let bin = hex::decode(encoded_events).unwrap();
        println!("bin{}", bin.len());
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
        // println!("{:#?}", val);
    }

    #[test]
    fn polkadot_millionth_block_hash() {
        let url = "wss://rpc.polkadot.io:443";
        let client = async_std::task::block_on(ClientBuilder::new().set_url(url).build()).unwrap();
        let api = client
            .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
        let blockhash = async_std::task::block_on(get_block_hash(&api, &url, 1000_000)).unwrap();
        let actual = hex::encode(blockhash.as_bytes());

        assert_eq!(
            actual,
            "490cd542b4a40ad743183c7d1088a4fe7b1edf21e50c850b86f29e389f31c5c1"
        );
    }

    #[test]
    fn polkadot_millionth_block_3_extrinsics() {
        let url = "wss://rpc.polkadot.io:443";
        let client = async_std::task::block_on(ClientBuilder::new().set_url(url).build()).unwrap();
        let api = client
            .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
        let blockhash = async_std::task::block_on(get_block_hash(&api, &url, 1000_000)).unwrap();
        let actual = hex::encode(blockhash.as_bytes());

        assert_eq!(
            actual,
            "490cd542b4a40ad743183c7d1088a4fe7b1edf21e50c850b86f29e389f31c5c1"
        );
    }

    #[test]
    fn get_extrinsics_test() {
        let url = "wss://rpc.polkadot.io:443";
        let client = block_on(ClientBuilder::new().set_url(url).build()).unwrap();
        let api = client
            .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>();
        let blockhash = block_on(get_block_hash(&api, &url, 1000_000)).unwrap();

        let (_block_num, results) = block_on(get_extrinsics(url, &api, blockhash)).unwrap();

        let metad = block_on(get_desub_metadata(&url, None)).unwrap();
        if let Ok(extrinsic) =
            decoder::decode_unwrapped_extrinsic(&metad, &mut results[0].as_slice())
        {
            println!("{:#?}", extrinsic);
        } else {
            println!("could not decode.");
        }

        assert_eq!(3, results.len());
    }
}
