use crate::{
	alloc::borrow::ToOwned,
	rpc::{self, extract_subscription, parse_changes, Rpc, RpcResult, StateChanges, Streamable},
	Error,
};
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc};
use async_mutex::Mutex;
use core::time::Duration;
use jsonrpc::serde_json;
#[cfg(feature = "logging")]
use log::info;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{MessageEvent, WebSocket};

type Id = u8;

#[derive(Clone)]
pub struct Backend {
	stream: WebSocket,
	messages: Arc<Mutex<BTreeMap<Id, async_oneshot::Sender<rpc::Response>>>>,
	streams: Arc<Mutex<BTreeMap<String, async_std::channel::Sender<StateChanges>>>>,
}

impl Streamable for Backend {
	async fn stream(
		&self,
		method: &str,
		params: &str,
	) -> async_std::channel::Receiver<StateChanges> {
		let result = self.rpc(method, params).await;
		let (sender, recv) = async_std::channel::unbounded();
		if let Ok(result_subscription) = result {
			if let Ok(result) = extract_subscription(&result_subscription) {
				self.streams.lock().await.insert(result.to_owned(), sender);
			}
		}

		recv
	}
}

// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Rpc for Backend {
	async fn rpc(&self, method: &str, params: &str) -> RpcResult {
		let id = self.next_id().await;
		#[cfg(feature = "logging")]
		log::trace!("RPC normal `{}`", method);

		// Store a sender that will notify our receiver when a matching message arrives
		let (sender, recv) = async_oneshot::oneshot::<rpc::Response>();
		let messages = self.messages.clone();
		messages.lock().await.insert(id, sender);

		// send rpc request
		let msg = format!(
			"{{\"id\":{}, \"jsonrpc\": \"2.0\", \"method\":\"{}\", \"params\":{}}}",
			id, method, params
		);

		#[cfg(feature = "logging")]
		log::trace!("RPC Request {} ...", &msg[..msg.len().min(150)]);
		{
			let lock = &self.stream;
			#[cfg(feature = "logging")]
			log::trace!("RPC got lock now sending {} ...", &msg[..50]);
			while lock.ready_state() < web_sys::WebSocket::OPEN {
				let delay = 300;
				#[cfg(target_arch="wasm32")]
				{
					use gloo_timers::future::sleep;
					sleep(Duration::from_millis(delay as u64)).await;
				}
				#[cfg(not(target_arch="wasm32"))]
				async_std::task::sleep(Duration::from_millis(delay as u64)).await;
			}

			lock.send_with_str(&msg).unwrap();
		}
		#[cfg(feature = "logging")]
		log::trace!("RPC now waiting for response ...");

		match recv.await {
			Ok(msg) => Ok(msg.result.unwrap()),
			Err(_err) => Err(jsonrpc::Error::EmptyBatch),
		}
	}
}

impl Backend {
	pub async fn new(urls: &[&str]) -> core::result::Result<Self, Error> {
		for url in urls {
			#[cfg(feature = "logging")]
			log::info!("WS connecting to {}", url);

			if let Ok(stream) = web_sys::WebSocket::new(url) {
				#[cfg(feature = "logging")]
				info!("Connection successfully created");

				let backend =
					Backend { stream, messages: Default::default(), streams: Default::default() };

				let messages = backend.messages.clone();
				let streams = backend.streams.clone();
				let onmessage_callback: Closure<dyn Fn(MessageEvent)> =
					Closure::wrap(Box::new(move |e: MessageEvent| {
						use pollster::FutureExt as _;
						if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
							let msg: alloc::string::String = txt.into();
							let res: rpc::Response = serde_json::from_str(&msg).unwrap();
							if res.id.is_u64() {
								let id = res.id.as_u64().unwrap() as Id;

								let mut messages = messages.lock().block_on();
								if let Some(mut channel) = messages.remove(&id) {
									channel.send(res).expect("receiver waiting");
									#[cfg(feature = "logging")]
									log::debug!("Answered request id: {}", id);
								}
							} else {
								let res: Result<serde_json::Value, _> = serde_json::from_str(&msg);
								if let Ok(res) = res {
									if let Some((subscription_id, state_changes)) =
										parse_changes(&res)
									{
										let mut streams = streams.lock().block_on();
										let sender = streams.get_mut(subscription_id).unwrap();
										sender
											.send(state_changes)
											.block_on()
											.expect("receiver waiting");
									}
								}
							}
						}
					}));

				// forget the callback to keep it alive
				backend.stream.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
				onmessage_callback.forget();

				return Ok(backend)
			}
		}
		Err(Error::ChainUnavailable)
	}

	async fn next_id(&self) -> Id {
		self.messages.lock().await.keys().last().unwrap_or(&0) + 1
	}
}

#[cfg(feature = "ws-web")]
#[cfg(test)]
mod tests {
	use super::*;
	use wasm_bindgen_test::*;
	wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

	#[wasm_bindgen_test]
	fn no_op() {}

	#[cfg(target_arch = "wasm32")]
	pub fn set_panic_hook() {
		// When the `console_error_panic_hook` feature is enabled, we can call the
		// `set_panic_hook` function at least once during initialization, and then
		// we will get better error messages if our code ever panics.
		//
		// For more details see
		// https://github.com/rustwasm/console_error_panic_hook#readme
		#[cfg(feature = "console_error_panic_hook")]
		console_error_panic_hook::set_once();
	}

	async fn polkadot_backend() -> crate::PolkaPipe<super::Backend> {
		crate::PolkaPipe {
			rpc: crate::ws_web::Backend::new(vec!["wss://rpc.polkadot.io"].as_slice())
				.await
				.unwrap(),
		}
	}

	#[wasm_bindgen_test]
	fn can_get_metadata() {
		async_std::task::block_on(can_get_metadata_test());
	}

	async fn can_get_metadata_test() {
		set_panic_hook();
		// wasm-pack test --headless --firefox --no-default-features --features ws-web

		let backend = polkadot_backend().await;
		let latest_metadata = backend.query_metadata(None).await.unwrap();
		assert!(latest_metadata.len() > 100);
	}

	#[wasm_bindgen_test]
	fn can_get_metadata_as_of() {
		async_std::task::block_on(can_get_metadata_as_of_test());
	}

	async fn can_get_metadata_as_of_test() {
		let block_hash =
			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
				.unwrap();
		let as_of_metadata =
			polkadot_backend().await.query_metadata(Some(&block_hash)).await.unwrap();
		assert!(as_of_metadata.len() > 100);
	}

	#[wasm_bindgen_test]
	fn can_get_block_hash() {
		async_std::task::block_on(can_get_block_hash_test());
	}

	async fn can_get_block_hash_test() {
		env_logger::init();
		let polkadot = polkadot_backend().await;
		let hash = polkadot.query_block_hash(&vec![1]).await.unwrap();
		assert_eq!(
			"c0096358534ec8d21d01d34b836eed476a1c343f8724fa2153dc0725ad797a90",
			hex::encode(hash)
		);

		let hash = polkadot.query_block_hash(&vec![10504599]).await.unwrap();
		assert_eq!(
			"e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16",
			hex::encode(hash)
		);
	}

	#[wasm_bindgen_test]
	fn can_get_full_block() {
		async_std::task::block_on(can_get_full_block_test());
	}

	async fn can_get_full_block_test() {
		let hash = Some("c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a");
		let block_bytes = polkadot_backend().await.query_block(hash).await.unwrap();
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	#[wasm_bindgen_test]
	fn can_get_latest_full_block() {
		async_std::task::block_on(can_get_latest_full_block_test());
	}

	async fn can_get_latest_full_block_test() {
		let block_bytes = polkadot_backend().await.query_block(None).await.unwrap();
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	#[wasm_bindgen_test]
	fn can_get_storage_as_of() {
		async_std::task::block_on(can_get_storage_as_of_test());
	}

	async fn can_get_storage_as_of_test() {
		//env_logger::init();
		let block_hash =
			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
				.unwrap();

		let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
		let key = hex::decode(events_key).unwrap();

		let as_of_events = polkadot_backend()
			.await
			.query_storage(&key[..], Some(&block_hash))
			.await
			.unwrap();
		assert!(as_of_events.len() > 0);
	}

	#[wasm_bindgen_test]
	fn can_get_storage_now() {
		async_std::task::block_on(can_get_storage_now_test());
	}

	async fn can_get_storage_now_test() {
		// env_logger::init();
		let key = "0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";
		let key = hex::decode(key).unwrap();
		let parachain = crate::PolkaPipe {
			rpc: super::Backend::new(vec!["wss://calamari-rpc.dwellir.com"].as_slice())
				.await
				.unwrap(),
		};

		let as_of_events = parachain.query_storage(&key[..], None).await.unwrap();
		assert_eq!(hex::decode("e8030000").unwrap(), as_of_events);
		// This is statemint's scale encoded parachain id (1000)
	}
}
