use crate::datasource::Source;
use async_trait::async_trait;
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
        Self {
            ws_url: url.to_string(),
            underlying_source,
            urlhash,
        }
    }
}

macro_rules! memoise {
    ($self:expr, $keybytes:expr, $fetch:expr) => {{
        let path = format!("target/{}.data", $self.urlhash);
        let _ = std::fs::create_dir(&path);

        let filename = format!("{}/{}.storage", path, hex::encode($keybytes));

        if let Ok(contents) = std::fs::read(&filename) {
            // println!("cache hit events!");
            if contents.is_empty() {
                Ok(None)
            } else {
                Ok(Some(contents))
            }
        } else {
            // println!("cache miss storage {} {}", filename, &$self.ws_url);
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
        self.underlying_source.fetch_block_hash(block_number).await
    }

    async fn fetch_block(
        &mut self,
        block_hash: Option<H256>,
    ) -> Result<Option<(u32, Vec<Vec<u8>>)>, BError> {
        self.underlying_source.fetch_block(block_hash).await
    }

    async fn fetch_chainname(&mut self) -> Result<String, BError> {
        self.underlying_source.fetch_chainname().await
    }

    async fn fetch_storage(
        &mut self,
        key: sp_core::storage::StorageKey,
        as_of: Option<H256>,
    ) -> Result<Option<sp_core::storage::StorageData>, BError> {
        memoise!(
            self,
            key.0.as_slice() + as_of,
            self.underlying_source
                .fetch_storage(key, as_of)
                .map_ok(|res| res.map(|sp_core::storage::StorageData(bytes)| bytes))
        )
        .map(|op| op.map(|bytes| sp_core::storage::StorageData(bytes)))
    }

    async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<sp_core::Bytes, ()> {
        memoise!(
            self,
            as_of,
            self.underlying_source
                .fetch_metadata(as_of.map(|as_of| as_of.as_bytes()))
                .map_ok(|res| res.map(|sp_core::Bytes(bytes)| bytes))
        )
        .map(|op| op.map(|bytes| sp_core::Bytes(bytes)))
    }

    /// We subscribe to relay chains and self sovereign chains
    async fn subscribe_finalised_blocks(
        &mut self,
    ) -> Result<Box<dyn futures::Stream<Item = Result<H256, ()>> + Unpin>, ()> {
        self.underlying_source.subscribe_finalised_blocks().await
    }
}
