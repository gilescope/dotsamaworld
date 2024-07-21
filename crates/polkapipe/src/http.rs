use crate::{
	prelude::*,
	rpc::{StateChanges, Streamable},
};
use core::{convert::TryInto, fmt};
use jsonrpc::{
	error::{standard_error, StandardError},
	serde_json::value::to_raw_value,
};
pub use surf::Url;

use crate::rpc::{self, Rpc, RpcResult};

#[derive(Debug)]
pub struct Backend(Url);

impl Streamable for Backend {
	async fn stream(
		&self,
		method: &str,
		params: &str,
	) -> async_std::channel::Receiver<StateChanges> {
		let _result = self.rpc(method, params).await;
		panic!("unsupported for now");
		// let (sender, recv) = async_std::channel::unbounded();
		// if let Ok(result_subscription) = result {
		// 	if let Ok(result) = extract_subscription(&result_subscription) {
		// 		self.streams.lock().await.insert(result.to_owned(), sender);
		// 	}
		// }

		// recv
	}
}

impl Backend {
	pub fn new(urls: &[Url]) -> Self
	{
		Backend(urls[0].clone())
	}
}

// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Rpc for Backend {
	/// HTTP based JSON RPC request expecting valid json result.
	async fn rpc(&self, method: &str, params: &str) -> RpcResult {
		#[cfg(feature = "logging")]
		log::info!("RPC `{}` to {}", method, &self.0);
		let id = 1_u32;
		let body = format!(
			"{{\"id\":{}, \"jsonrpc\": \"2.0\", \"method\":\"{}\", \"params\":{}}}",
			id, method, params
		);
		#[cfg(feature = "logging")]
		log::debug!("outgoing request was: `{}`", body);
		let req = surf::post(&self.0).content_type("application/json").body(body);
		let client = surf::client().with(surf::middleware::Redirect::new(2));
		let mut res = client
			.send(req)
			.await
			.map_err(|err| {
				log::error!("error sending: {}", err);
				standard_error(StandardError::InternalError, None)
			})?;

		let status = res.status();
		#[cfg(feature = "logging")]
		log::debug!("outgoing request status: `{}`", status);

		let res = if status.is_success() {
			res.body_json::<rpc::Response>().await.map_err(|err| {
				standard_error(StandardError::ParseError, to_raw_value(&err.to_string()).ok())
			})?
		} else {
			#[cfg(feature = "logging")]
			log::debug!("RPC HTTP status: {}", res.status());
			let err = res.body_string().await.unwrap_or_else(|_| status.canonical_reason().into());
			let err = to_raw_value(&err).expect("error string");
			#[cfg(feature = "logging")]
			log::debug!("RPC Response: {:?}...", &res);

			return Err(if status.is_client_error() {
				standard_error(StandardError::InvalidRequest, Some(err)).into()
			} else {
				standard_error(StandardError::InternalError, Some(err)).into()
			})
		};

		// assume the response is a hex encoded string starting with "0x"
		// let response = hex::decode(&res[2..])
		// 	.map_err(|_err| standard_error(StandardError::InternalError, None))?;
		res.result.ok_or(jsonrpc::error::Error::EmptyBatch)
	}
}

#[cfg(feature = "http")]
#[cfg(test)]
mod tests {
	use super::Backend;
	use surf::Url;

	fn init() {
		let _ = env_logger::builder().is_test(true).try_init();
	}

	fn polkadot_backend() -> crate::PolkaPipe<Backend> {
		crate::PolkaPipe::<Backend> { rpc: Backend::new(&vec![Url::parse("http://rpc.polkadot.io").unwrap()]) }
	}
	//{"id":1,"jsonrpc":"2.0","method":"state_getKeys","params":["1234"]}
	//websocat wss://statemint-rpc-tn.dwellir.com
	//{"id":1,"jsonrpc":"2.0","method":"state_getKeys","params":["0x682a59d51ab9e48a8c8cc418ff9708d2d34371a193a751eea5883e9553457b2e"]}
	//{"id":1,"jsonrpc":"2.0","method":"state_subscribeStorage","params":[["0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"]]}

	//{"id":1,"jsonrpc":"2.0","method":"state_getKeys","params":["0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"]}
	//{"id":1,"jsonrpc":"2.0","method":"state_getKeysPaged","params":["0x26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7", 1, ""]}
	#[test]
	fn can_get_metadata() {
		init();
		let latest_metadata =
			async_std::task::block_on(polkadot_backend().query_metadata(None)).unwrap();
		assert!(latest_metadata.len() > 0);
	}

	#[test]
	fn can_get_metadata_as_of() {
		init();
		let block_hash =
			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
				.unwrap();
		let as_of_metadata =
			async_std::task::block_on(polkadot_backend().query_metadata(Some(&block_hash)))
				.unwrap();
		assert!(as_of_metadata.len() > 0);
	}

	#[test]
	fn can_get_block_hash() {
		init();
		let polkadot = polkadot_backend();
		let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![1])).unwrap();
		assert_eq!(
			"c0096358534ec8d21d01d34b836eed476a1c343f8724fa2153dc0725ad797a90",
			hex::encode(hash)
		);

		let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![10504599])).unwrap();
		assert_eq!(
			"e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16",
			hex::encode(hash)
		);
	}

	#[test]
	fn can_get_full_block() {
		init();
		let hash = "c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a";
		let block_bytes =
			async_std::task::block_on(polkadot_backend().query_block(Some(hash))).unwrap();
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	#[test]
	fn can_get_latest_block() {
		init();
		let block_bytes = async_std::task::block_on(polkadot_backend().query_block(None)).unwrap();
		// println!("{:?}", &block_bytes);
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	#[test]
	fn can_get_state_keys() {
		init();
		let prefix = "c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a";
		let block_bytes =
			async_std::task::block_on(polkadot_backend().state_get_keys(prefix, None)).unwrap();
		// println!("{:?}", block_bytes);
		assert!(matches!(block_bytes, serde_json::value::Value::Array(_)));
	}

	#[test]
	fn can_get_state_keys_as_of() {
		init();
		let block_hash =
			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
				.unwrap();

		let prefix = "c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a";
		let block_bytes =
			async_std::task::block_on(polkadot_backend().state_get_keys_paged(prefix, 0, None))
				.unwrap();
		// println!("{:?}", block_bytes);
		assert!(matches!(block_bytes, serde_json::value::Value::Array(_)));
	}

	#[test]
	fn can_get_storage_as_of() {
		init();
		let block_hash =
			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
				.unwrap();

		let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
		let key = hex::decode(events_key).unwrap();

		let as_of_events = async_std::task::block_on(
			polkadot_backend().query_storage(&key[..], Some(&block_hash)),
		)
		.unwrap();
		assert!(as_of_events.len() > 0);
	}

	#[test]
	fn can_get_storage_now() {
		init();
		let key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
		let key = hex::decode(key).unwrap();
		let parachain = polkadot_backend();

		let as_of_events =
			async_std::task::block_on(parachain.query_storage(&key[..], None)).unwrap();
		assert!(as_of_events.len() > 10);
	}

	#[test]
	fn can_get_state_call_metadata_now() {
		init();
		let key = "";
		let key = hex::decode(key).unwrap();
		let parachain = polkadot_backend();

		let payload = async_std::task::block_on(parachain.query_state_call(
			"Metadata_metadata",
			&key[..],
			None,
		))
		.unwrap();

		let payload2 = async_std::task::block_on(parachain.query_metadata(None)).unwrap();

		assert!(payload.len() > 10);
		//4 extra bytes prefixed if you call state_call: 206, 153, 21, 0, rather than getMetadata.
		assert_eq!(&payload[4..], &payload2[0..]);
	}

	// #[test]
	// fn can_get_state_changes() {
	// 	init();
	// 	async_std::task::block_on(testy());
	// }

	// async fn testy() {
	// 	let key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
	// 	let key = hex::decode(key).unwrap();
	// 	let parachain = polkadot_backend();

	// 	let payload_stream =
	// 		async_std::task::block_on(parachain.subscribe_storage( &key[..], None));

	// }
}
