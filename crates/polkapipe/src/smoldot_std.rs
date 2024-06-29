use crate::{
	prelude::*,
	rpc::{self, Rpc, RpcResult, StateChanges, Streamable},
};
use async_std::{sync::Mutex, task};
use futures::{channel::mpsc, prelude::*};
use futures_channel::oneshot;
use jsonrpc::{
	error::{result_to_response, standard_error, RpcError, StandardError},
	serde_json,
};
use lazy_static::lazy_static;
use smoldot_light::ChainId;
use std::{
	collections::{hash_map::DefaultHasher, BTreeMap},
	hash::{Hash, Hasher},
	sync::Arc,
};

type Id = u8;

lazy_static! {
	static ref CLIENT: Arc<
		Mutex<
			Option<smoldot_light::Client<smoldot_light::platform::async_std::AsyncStdTcpWebSocket>>,
		>,
	> = Arc::new(Mutex::new(None));
	/// Chainspec hash to backend so we only initialise each chain once.
	static ref CHAINS: Arc<Mutex<Vec<(u64, Backend)>>> = Arc::new(Mutex::new(vec![]));
}

#[derive(Clone)]
pub struct Backend {
	chain_id: ChainId,
	messages: Arc<Mutex<BTreeMap<Id, oneshot::Sender<rpc::Response>>>>,
}

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
	pub async fn new(chainspec: &str, parent_chain: Option<Backend>) -> Result<Self, crate::Error> {
		let (json_rpc_responses_tx, json_rpc_responses_rx) = mpsc::channel(32);

		let mut client = CLIENT.lock().await;

		if client.is_none() {
			*client = Some(smoldot_light::Client::<
				smoldot_light::platform::async_std::AsyncStdTcpWebSocket,
			>::new(smoldot_light::ClientConfig {
				// The smoldot client will need to spawn tasks that run in the background. In
				// order to do so, we need to provide a "tasks spawner".
				tasks_spawner: Box::new(move |_name, task| {
					async_std::task::spawn(task);
				}),
				system_name: env!("CARGO_PKG_NAME").into(),
				system_version: env!("CARGO_PKG_VERSION").into(),
			}));
		}

		let mut chains = CHAINS.lock().await;

		let mut hasher = DefaultHasher::new();
		chainspec.hash(&mut hasher);
		let hash: u64 = hasher.finish();

		if let Some((_, backend)) = chains.iter().find(|(h, _)| *h == hash) {
			Ok(backend.clone())
		} else {
			let potential_relay_chains =
				if let Some(parent) = parent_chain { vec![parent.chain_id] } else { vec![] }
					.into_iter();
			let chain_id = (*client)
				.as_mut()
				.expect("client set to Some above.")
				.add_chain(smoldot_light::AddChainConfig {
					// The most important field of the configuration is the chain specification.
					// This is a JSON document containing all the information necessary for the
					// client to connect to said chain.
					specification: chainspec,

					// See above.
					// Note that it is possible to pass `None`, in which case the chain will not be
					// able to handle JSON-RPC requests. This can be used to save up some resources.
					json_rpc_responses: Some(json_rpc_responses_tx),

					// This field is necessary only if adding a parachain.
					potential_relay_chains,

					// After a chain has been added, it is possible to extract a "database" (in the
					// form of a simple string). This database can later be passed back the next
					// time the same chain is added again.
					// A database with an invalid format is simply ignored by the client.
					// In this example, we don't use this feature, and as such we simply pass an
					// empty string, which is intentionally an invalid database content.
					database_content: "",

					// The client gives the possibility to insert an opaque "user data" alongside
					// each chain. This avoids having to create a separate `HashMap<ChainId, ...>`
					// in parallel of the client.
					// In this example, this feature isn't used. The chain simply has `()`.
					user_data: (),
				})
				.unwrap();

			let backend = Backend { chain_id, messages: Arc::new(Mutex::new(BTreeMap::new())) };
			backend.process_incoming_messages(json_rpc_responses_rx);
			chains.push((hash, backend.clone()));
			Ok(backend)
		}
	}

	async fn next_id(&self) -> Id {
		self.messages.lock().await.keys().last().unwrap_or(&0) + 1
	}

	fn process_incoming_messages(
		&self,
		mut rx: futures_channel::mpsc::Receiver<std::string::String>,
	) {
		let messages = self.messages.clone();

		task::spawn(async move {
			while let Some(msg) = rx.next().await {
				let res: rpc::Response = serde_json::from_str(&msg).unwrap_or_else(|_| {
					result_to_response(
						Err(standard_error(StandardError::ParseError, None)),
						().into(),
					)
				});
				if res.id.is_u64() {
					let id = res.id.as_u64().unwrap() as Id;
					#[cfg(feature = "logging")]
					log::trace!("Answering request {}", id);
					let mut messages = messages.lock().await;
					if let Some(channel) = messages.remove(&id) {
						channel.send(res).expect("receiver waiting");
						#[cfg(feature = "logging")]
						log::debug!("Answered request id: {}", id);
					}
				}
			}
			#[cfg(feature = "logging")]
			log::warn!("WS connection closed");
		});
	}
}

// #[async_trait]
impl Rpc for Backend {
	/// HTTP based JSONRpc request expecting an hex encoded result
	async fn rpc(&self, method: &str, params: &str) -> RpcResult {
		let id = self.next_id().await;
		#[cfg(feature = "logging")]
		log::debug!("RPC `{}` (ID={})", method, id);

		// Store a sender that will notify our receiver when a matching message arrives
		let (sender, recv) = oneshot::channel::<rpc::Response>();
		let messages = self.messages.clone();
		messages.lock().await.insert(id, sender);

		let msg = format!(
			"{{\"id\":{}, \"jsonrpc\": \"2.0\", \"method\":\"{}\", \"params\":{}}}",
			id, method, params
		);

		#[cfg(feature = "logging")]
		log::debug!("RPC Request {} ...", &msg[..msg.len().min(150)]);
		CLIENT
			.lock()
			.await
			.as_mut()
			.expect("client to have been inititiated in new")
			.json_rpc_request(msg, self.chain_id)
			.map_err(|_e| {
				jsonrpc::Error::Rpc(standard_error(StandardError::InternalError, None))
			})?;

		// wait for the matching response to arrive
		let res = recv.await;
		// println!("RPC response: {:?}", &res);
		let res = res.map_err(|_| standard_error(StandardError::InternalError, None))?;
		if let Some(result) = res.result {
			Ok(result)
		} else {
			Err(jsonrpc::Error::Rpc(RpcError {
				code: 42,
				message: format!("Error result: {:?}", &res),
				data: None,
			}))
		}
	}
}

/// For smoldot tests we just check that we can retrieve the latest bits.
#[cfg(test)]
mod tests {
	fn init() {
		let _ = env_logger::builder().is_test(true).try_init();
	}

	async fn polkadot_backend() -> crate::PolkaPipe<super::Backend> {
		if cfg!(debug_assertions) {
			panic!("This is not the mode you are looking for. Smoldot is slow (minutes) in debug mode.");
		}
		crate::PolkaPipe {
			rpc: super::Backend::new(include_str!("../chainspecs/polkadot.json"), None)
				.await
				.unwrap(),
		}
	}

	async fn statemint_backend() -> crate::PolkaPipe<super::Backend> {
		if cfg!(debug_assertions) {
			panic!("This is not the mode you are looking for. Smoldot is slow (minutes) in debug mode.");
		}
		let relay_backend = polkadot_backend().await;
		crate::PolkaPipe {
			rpc: super::Backend::new(
                include_str!("../chainspecs/statemint.json"),
                Some(relay_backend.rpc),
			)
			.await
			.unwrap(),
		}
	}

	#[test]
	fn can_get_metadata() {
		init();
		let backend = async_std::task::block_on(polkadot_backend());
		let latest_metadata = async_std::task::block_on(backend.query_metadata(None)).unwrap();
		assert!(latest_metadata.len() > 0);

		// Check statemint's metadata is different to relay chain.
		let statemint = async_std::task::block_on(statemint_backend());
		let latest_metadata_statemint =
			async_std::task::block_on(statemint.query_metadata(None)).unwrap();
		assert!(latest_metadata.len() > 0);
		assert_ne!(latest_metadata, latest_metadata_statemint);
	}

	// #[test]
	// fn can_get_metadata_as_of() {
	// 	let block_hash =
	// 		hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
	// 			.unwrap();
	// 	let as_of_metadata =
	// 		async_std::task::block_on(polkadot_backend().query_metadata(Some(&block_hash)))
	// 			.unwrap();
	// 	assert!(as_of_metadata.len() > 0);
	// }

	// #[test]
	// fn can_get_block_hash() {
	// 	// env_logger::init();
	// 	let polkadot = polkadot_backend();
	// 	std::thread::sleep(std::time::Duration::from_secs(10));
	// 	let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![1])).unwrap();
	// 	println!("{:?}", String::from_utf8(hash.clone()));
	// 	assert_eq!(
	// 		"c0096358534ec8d21d01d34b836eed476a1c343f8724fa2153dc0725ad797a90",
	// 		hex::encode(hash)
	// 	);

	// 	// let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![10504599])).unwrap();
	// 	// assert_eq!(
	// 	// 	"e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16",
	// 	// 	hex::encode(hash)
	// 	// );
	// }

	// #[test]
	// fn can_get_full_block() {
	// 	let hash = "c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a";
	// 	let block_bytes =
	// 		async_std::task::block_on(polkadot_backend().query_block(Some(hash))).unwrap();
	// 	assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	// }

	#[test]
	fn can_get_latest_block() {
		init();
		let backend = async_std::task::block_on(polkadot_backend());
		let block_bytes = async_std::task::block_on(backend.query_block(None)).unwrap();
		// println!("{:?}", &block_bytes);
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	#[test]
	fn can_get_latest_block_on_parachain() {
		init();
		let statemint = async_std::task::block_on(statemint_backend());
		let block_bytes = async_std::task::block_on(statemint.query_block(None)).unwrap();
		// println!("{:?}", &block_bytes);
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	// #[test]
	// fn can_get_storage_as_of() {
	// 	// env_logger::init();
	// 	let block_hash =
	// 		hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
	// 			.unwrap();

	// 	let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
	// 	let key = hex::decode(events_key).unwrap();

	// 	let as_of_events = async_std::task::block_on(
	// 		polkadot_backend().query_storage(&key[..], Some(&block_hash)),
	// 	)
	// 	.unwrap();
	// 	assert!(as_of_events.len() > 0);
	// }

	#[test]
	fn can_get_storage_latest() {
		init();

		let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
		let key = hex::decode(events_key).unwrap();

		let backend = async_std::task::block_on(polkadot_backend());
		let as_of_events =
			async_std::task::block_on(backend.query_storage(&key[..], None)).unwrap();
		assert!(as_of_events.len() > 0);
	}
}
