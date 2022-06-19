use super::polkadot;
use crate::polkadot::RuntimeApi;
use async_std::stream::StreamExt;
use async_trait::async_trait;
use parity_scale_codec::Encode;
use sp_core::H256;
use subxt::rpc::ClientT;
use subxt::Client;
use subxt::ClientBuilder;
use subxt::DefaultConfig;
use subxt::DefaultExtra;

/// A way to source untransformed raw data.
#[async_trait(?Send)]
pub trait Source {
    // async fn client(&mut self) -> &mut Client<DefaultConfig>;

    // async fn get_api(&mut self) -> &mut RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>;

    async fn fetch_block_hash(
        &mut self,
        block_number: u32,
    ) -> Result<Option<sp_core::H256>, BError>;

    async fn fetch_block(
        &mut self,
        block_hash: Option<H256>,
    ) -> Result<Option<(u32, Vec<Vec<u8>>)>, BError>;

    async fn fetch_chainname(&mut self) -> Result<String, BError>;

    async fn fetch_storage(
        &mut self,
        key: sp_core::storage::StorageKey,
        as_of: Option<H256>,
    ) -> Result<Option<sp_core::storage::StorageData>, BError>;

    async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<sp_core::Bytes, ()>;

    /// We subscribe to relay chains and self sovereign chains
    /// TODO -> impl Iter<BlockHash>
    async fn subscribe_finalised_blocks(
        &mut self,
    ) -> Result<
        // Subscription<
        //     subxt::sp_runtime::generic::Header<u32, subxt::sp_runtime::traits::BlakeTwo256>,
        // >
        Box<dyn futures::Stream<Item = Result<H256, ()>> + Unpin>,
        (),
    >;
}

pub struct RawDataSource {
    ws_url: String,
    api: Option<RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>,
}

type BError = subxt::GenericError<std::convert::Infallible>; // Box<dyn std::error::Error>;

/// This is the only type that should know about subxt
impl RawDataSource {
    pub fn new(url: &str) -> Self {
        RawDataSource {
            ws_url: url.to_string(),
            api: None,
        }
    }

    async fn client(&mut self) -> &mut Client<DefaultConfig> {
        &mut self.get_api().await.client
        // if self.api.is_some() {
        //     return &mut self.api.as_mut().unwrap().client;
        // }
        // let client = ClientBuilder::new()
        //     .set_url(&self.ws_url)
        //     .build()
        //     .await
        //     .unwrap();
        // self.api = Some(
        //     client
        //         .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>(
        //         ),
        // );
        // &mut self.api.as_mut().unwrap().client
    }

    async fn get_api(&mut self) -> &mut RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>> {
        if self.api.is_some() {
            return self.api.as_mut().unwrap();
        }
        let client = ClientBuilder::new()
            .set_url(&self.ws_url)
            .build()
            .await
            .unwrap();
        self.api = Some(
            client
                .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>(
                ),
        );
        self.api.as_mut().unwrap()
    }
}

#[async_trait(?Send)]
impl Source for RawDataSource {
    async fn fetch_block_hash(
        &mut self,
        block_number: u32,
    ) -> Result<Option<sp_core::H256>, BError> {
        self.get_api()
            .await
            .client
            .rpc()
            .block_hash(Some(block_number.into()))
            .await
    }

    /// Return then in bin form rather than link to subxt:
    /// subxt::sp_runtime::generic::SignedBlock<
    ///     subxt::sp_runtime::generic::Block<
    ///         subxt::sp_runtime::generic::Header<
    ///             u32,
    ///             subxt::sp_runtime::traits::BlakeTwo256
    ///         >,
    ///         subxt::sp_runtime::OpaqueExtrinsic
    ///       
    async fn fetch_block(
        &mut self,
        block_hash: Option<H256>,
    ) -> Result<Option<(u32, Vec<Vec<u8>>)>, BError> {
        let result = self.get_api().await.client.rpc().block(block_hash).await;
        if let Ok(Some(block_body)) = result {
            //TODO: we're decoding and encoding here. cut it out.
            Ok(Some((
                block_body.block.header.number,
                block_body
                    .block
                    .extrinsics
                    .into_iter()
                    .map(|ex| ex.encode())
                    .collect::<Vec<_>>(),
            )))
        } else {
            if let Err(err) = result {
                Err(err)
            } else {
                Ok(None)
            }
        }
    }

    async fn fetch_chainname(&mut self) -> Result<String, BError> {
        self.client().await.rpc().system_chain().await
    }

    async fn fetch_storage(
        &mut self,
        key: sp_core::storage::StorageKey,
        as_of: Option<H256>,
    ) -> Result<Option<sp_core::storage::StorageData>, BError> {
        self.client().await.storage().fetch_raw(key, as_of).await
    }

    async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<sp_core::Bytes, ()> {
        let mut params = None;
        if let Some(hash) = as_of {
            params = Some(jsonrpsee_types::ParamsSer::Array(vec![
                serde_json::Value::String(hex::encode(hash.as_bytes())),
            ]));
        }

        //        self.client().rpc().metadata_bytes(as_of).await
        //TODO: if asof is none then client.Metadata could just be encoded.
        let res = self
            .get_api()
            .await
            .client
            .rpc()
            .client
            .request("state_getMetadata", params.clone())
            .await;
        match res {
            Ok(res) => {
                return Ok(res);
            }
            _ => {
                return Err(());
            }
        };
    }

    /// We subscribe to relay chains and self sovereign chains
    async fn subscribe_finalised_blocks(
        &mut self,
    ) -> Result<
        // Subscription<
        //     subxt::sp_runtime::generic::Header<u32, subxt::sp_runtime::traits::BlakeTwo256>,
        // >
        Box<dyn futures::Stream<Item = Result<H256, ()>> + Unpin>,
        (),
    > {
        let result = self
            .get_api()
            .await
            .client
            .rpc()
            .subscribe_finalized_blocks()
            .await;
        if let Ok(sub) = result {
            // sub is a Stream... can we map a stream?
            Ok(Box::new(sub.map(|block_header_result| {
                if let Ok(block_header) = block_header_result {
                    let block_header: subxt::sp_runtime::generic::Header<
                        u32,
                        subxt::sp_runtime::traits::BlakeTwo256,
                    > = block_header;
                    Ok(block_header.hash())
                } else {
                    Err(())
                }
            })))
        } else {
            Err(())
        }
    }
}
