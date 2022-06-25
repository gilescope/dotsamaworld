use crate::datasource::{raw_source::AgnosticBlock, Source};
use async_trait::async_trait;
use bevy::render::render_resource::std140::Std140;
use futures::TryFutureExt;
use sp_core::H256;

pub struct CachedDataSource<S: Source> {
	ws_url: String,
	underlying_source: S,
	urlhash: u64,
}

type BError = subxt::GenericError<std::convert::Infallible>; // Box<dyn std::error::Error>;

impl<S> CachedDataSource<S>
where
	S: Source,
{
	pub fn new(url: &str, underlying_source: S) -> Self {
		let urlhash = super::please_hash(&url);
		Self { ws_url: url.to_string(), underlying_source, urlhash }
	}
}

macro_rules! memoise {
	($datatype:expr, $self:expr, $keybytes:expr, $fetch:expr) => {{
		let path = format!("target/{}.data", $self.urlhash);
		let _ = std::fs::create_dir(&path);

		let filename = format!("{}/{}.{}", path, hex::encode($keybytes), $datatype);

		if let Ok(contents) = std::fs::read(&filename) {
			// println!("cache hit events!");
			if contents.is_empty() {
				Ok(None)
			} else {
				Ok(Some(contents))
			}
		} else {
			println!("cache miss {} {}", filename, &$self.ws_url);
			let result = $fetch.await;
			if let Ok(result) = result {
				if let Some(bytes) = result {
					std::fs::write(&filename, bytes.as_slice())
						.expect(&format!("Couldn't write event output to {}", filename));
				// println!("cache storage wrote to {}", filename);
				} else {
					std::fs::write(&filename, vec![].as_slice())
						.expect(&format!("Couldn't write event output to {}", filename));
					// println!("cache storage wrote empty to {}", filename);
				}

				// Only let data read from cache so you know it's working.
				let contents = std::fs::read(&filename).unwrap();
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
	async fn fetch_block_hash(
		&mut self,
		block_number: u32,
	) -> Result<Option<sp_core::H256>, BError> {
		memoise!(
			"block_hash",
			self,
			block_number.as_bytes(),
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

	async fn fetch_chainname(&mut self) -> Result<Option<String>, BError> {
		memoise!(
			"chainname",
			self,
			b"chainname".as_slice(),
			self.underlying_source
				.fetch_chainname()
				.map_ok(|res| res.map(|name| name.as_bytes().to_vec()))
		)
		.map(|op| op.map(|bytes| String::from_utf8_lossy(bytes.as_slice()).to_string()))
	}

	async fn fetch_storage(
		&mut self,
		key: subxt::sp_core::storage::StorageKey,
		as_of: Option<H256>,
	) -> Result<Option<subxt::sp_core::storage::StorageData>, BError> {
		let mut cache_key = key.0.clone();
		if let Some(as_of) = as_of {
			cache_key.extend(as_of.as_bytes());
		}
		memoise!(
			"storage",
			self,
			cache_key.as_slice(),
			self.underlying_source
				.fetch_storage(key, as_of)
				.map_ok(|res| res.map(|subxt::sp_core::storage::StorageData(bytes)| bytes))
		)
		.map(|op| op.map(|bytes| subxt::sp_core::storage::StorageData(bytes)))
	}

	async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<Option<sp_core::Bytes>, ()> {
		memoise!(
			"metadata",
			self,
			as_of.unwrap_or_default().as_bytes(),
			self.underlying_source
				.fetch_metadata(as_of)
				.map_ok(|res| res.map(|sp_core::Bytes(bytes)| bytes))
		)
		.map(|op| op.map(|bytes| sp_core::Bytes(bytes)))
	}

	/// We subscribe to relay chains and self sovereign chains.
	/// Only used by live mode so should not cache.
	async fn subscribe_finalised_blocks(
		&mut self,
	) -> Result<Box<dyn futures::Stream<Item = Result<H256, ()>> + Unpin>, ()> {
		self.underlying_source.subscribe_finalised_blocks().await
	}
}
