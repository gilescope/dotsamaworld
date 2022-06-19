use crate::datasource::Source;
use async_trait::async_trait;
use sp_core::H256;

pub struct CachedDataSource<S: Source> {
    ws_url: String,
    underlying_source: S,
}

type BError = subxt::GenericError<std::convert::Infallible>; // Box<dyn std::error::Error>;

impl<S> CachedDataSource<S>
where
    S: Source,
{
    pub fn new(url: &str, underlying_source: S) -> Self {
        Self {
            ws_url: url.to_string(),
            underlying_source,
        }
    }
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
        self.underlying_source.fetch_storage(key, as_of).await
    }

    async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<sp_core::Bytes, ()> {
        self.underlying_source.fetch_metadata(as_of).await
    }

    /// We subscribe to relay chains and self sovereign chains
    async fn subscribe_finalised_blocks(
        &mut self,
    ) -> Result<Box<dyn futures::Stream<Item = Result<H256, ()>> + Unpin>, ()> {
        self.underlying_source.subscribe_finalised_blocks().await
    }
}
