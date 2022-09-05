use crate::datasource::{raw_source::AgnosticBlock, Source};
use async_trait::async_trait;
use futures::TryFutureExt;
use primitive_types::H256;

use rexie::ObjectStore;
#[cfg(target_arch = "wasm32")]
use rexie::{Rexie, TransactionMode};
use wasm_bindgen::JsValue;

//#[derive(Clone)]
pub struct CachedDataSource<S: Source> {
	store: Option<rexie::Rexie>,
	underlying_source: S,
	urlhash: u64,
}

// #[cfg(target_arch="wasm32")]
// type WSBackend = polkapipe::ws_web::Backend;

// macro_rules! log {
//     // Note that this is using the `log` function imported above during
//     // `bare_bones`
//     ($($t:tt)*) => (super::super::log(&format_args!($($t)*).to_string()))
// }

type BError = polkapipe::Error; //  Box<dyn std::error::Error>;

impl<S> CachedDataSource<S>
where
	S: Source,
{
	// pub fn new(underlying_source: S) -> Self {
	// 	let urlhash = super::please_hash(&underlying_source.url());
	// 	Self { store:  None,
	// 	underlying_source, urlhash }
	// }
}

fn conv(val: JsValue) -> Result<Vec<u8>, polkapipe::Error> {
	Ok(hex::decode(&val.as_string().unwrap()).unwrap())
	//TODO: To avoid the copying and re-encoding,
	// consider the JsString::try_from() function from js-sys instead.
}

macro_rules! memoise {
	($datatype:expr, $self:expr, $keybytes:expr, $fetch:expr) => {{
		let path = format!("target/{}.data", $self.urlhash);
		let _ = std::fs::create_dir(&path);

		let table_name = format!("{}.{}", path, $datatype);
		if $self.store.is_none() {
			$self.store = Some(
				Rexie::builder("DotsarmaWorld")
					.version(1)
					.add_object_store(ObjectStore::new("kv").key_path("id"))
					.build()
					.await
					.unwrap(),
			);
		}

		let key_encoded = format!("{}{}", table_name, hex::encode($keybytes));
		// let filename = format!("{}/{}.{}", path, key_encoded, $datatype);

		let tx = $self
			.store
			.as_ref()
			.unwrap()
			.transaction(&["kv"], TransactionMode::ReadOnly)
			.unwrap();
		let store = tx.store("kv").unwrap();
		let res: Result<Vec<u8>, _> =
			conv(store.get(&JsValue::from_str(&key_encoded)).await.unwrap());
		if let Ok(contents) = res {
			// println!("cache hit events!");
			if contents.is_empty() {
				Ok(None)
			} else {
				Ok(Some(contents))
			}
		} else {
			// println!("cache miss {} {}", filename, &$self.ws_url);
			let result = $fetch.await;
			if let Ok(result) = result {
				let tx = $self
					.store
					.as_ref()
					.unwrap()
					.transaction(&["kv"], TransactionMode::ReadWrite)
					.unwrap();
				if let Some(bytes) = result {
					tx.store("kv")
						.unwrap()
						.add(&JsValue::from_str(&hex::encode(bytes)), None)
						.await
						.unwrap();
				// $self.store.set(&filename, &bytes)
				// 	.expect(&format!("Couldn't write event output to {}", filename));
				// println!("cache storage wrote to {}", filename);
				} else {
					tx.store("kv").unwrap().add(&JsValue::from_str(""), None).await.unwrap();
					// $self.store.set(&filename, &Vec::<u8>::new())
					// 	.expect(&format!("Couldn't write event output to {}", filename));
					// println!("cache storage wrote empty to {}", filename);
				}

				// Only let data read from cache so you know it's working.
				let res: Result<Vec<u8>, _> =
					conv(store.get(&JsValue::from_str(&key_encoded)).await.unwrap());
				let contents = res.unwrap();
				if contents.is_empty() {
					Ok(None)
				} else {
					Ok(Some(contents))
				}
			} else {
				// println!("could not find storage for {}",&self.ws_url);
				result
			}
		}
	}};
}

#[async_trait(?Send)]
impl<S> Source for CachedDataSource<S>
where
	S: Source,
{
	// #[cfg(target_arch="wasm32")]
	// async fn process_incoming_messages(&mut self) -> WSBackend {
	// 	// log!("cached process incoming run");
	// 	self.underlying_source.process_incoming_messages().await
	// 	// log!("cached process incoming fin");
	// }

	async fn fetch_block_hash(
		&mut self,
		block_number: u32,
	) -> Result<Option<primitive_types::H256>, BError> {
		memoise!(
			"block_hash",
			self,
			block_number.to_le_bytes(),
			self.underlying_source
				.fetch_block_hash(block_number)
				.map_ok(|res| res.map(|hash| hash.as_bytes().to_vec()))
		)
		.map(|op| op.map(|bytes| H256::from_slice(bytes.as_slice())))
	}

	/// This is not as clean because SignedBlock takes Block as a generic arg and we want
	/// to be blockchain agnostic.
	async fn fetch_block(
		&mut self,
		block_hash: Option<H256>,
	) -> Result<Option<AgnosticBlock>, BError> {
		if let Some(block_hash) = block_hash {
			memoise!(
				"block",
				self,
				block_hash.as_bytes(),
				self.underlying_source
					.fetch_block(Some(block_hash))
					.map_ok(|res| res.map(|block| block.to_vec()))
			)
			.map(|op| op.map(|bytes| AgnosticBlock::from_bytes(bytes.as_slice()).unwrap()))
		} else {
			// Don't cache latest block (maybe cache the result though?)
			self.underlying_source.fetch_block(None).await
		}
	}

	async fn fetch_storage(
		&mut self,
		key: &[u8],
		as_of: Option<H256>,
	) -> Result<Option<Vec<u8>>, BError> {
		let mut cache_key = key.to_vec();
		if let Some(as_of) = as_of {
			cache_key.extend(as_of.as_bytes());
		}
		memoise!(
			"storage",
			self,
			cache_key.as_slice(),
			self.underlying_source.fetch_storage(key, as_of) /* .map_ok(|res|
			                                                  * res.map(|subxt::sp_core::
			                                                  * storage::StorageData(bytes)|
			                                                  * bytes)) */
		)
	}

	async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<Option<Vec<u8>>, ()> {
		memoise!(
			"metadata",
			self,
			as_of.unwrap_or_default().as_bytes(),
			self.underlying_source.fetch_metadata(as_of) /* .map_ok(|res|
			                                              * res.map(|sp_core::Bytes(bytes)|
			                                              * bytes)) */
		)
	}

	fn url(&self) -> &str {
		&self.underlying_source.url()
	}
}
