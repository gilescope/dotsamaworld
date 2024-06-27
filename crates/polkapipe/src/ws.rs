use crate::{
	rpc::{self, extract_subscription, parse_changes, Rpc, RpcResult, StateChanges, Streamable},
	Error,
};
use alloc::{collections::BTreeMap, sync::Arc};
use async_mutex::Mutex;
use async_std::task;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use core::time::Duration;
use futures_channel::oneshot;
use futures_util::{
	sink::{Sink, SinkExt},
	stream::SplitSink,
	Stream, StreamExt,
};
use jsonrpc::{
	error::{result_to_response, standard_error, StandardError},
	serde_json,
};

type Id = u16;

pub struct Backend<Tx> {
	tx: Mutex<Tx>,
	messages: Arc<Mutex<BTreeMap<Id, oneshot::Sender<rpc::Response>>>>,
	streams: Arc<Mutex<BTreeMap<String, async_std::channel::Sender<StateChanges>>>>,
}

// impl<Tx> BackendParent for Backend<Tx> where Tx: Sink<Message, Error = Error> + Unpin + Send {}

impl<Tx> Streamable for Backend<Tx>
where
	Tx: Sink<Message, Error = Error> + Unpin + Send,
{
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

impl<Tx> Rpc for Backend<Tx>
where
	Tx: Sink<Message, Error = Error> + Unpin + Send,
{
	async fn rpc(&self, method: &str, params: &str) -> RpcResult {
		let id = self.next_id().await;
		#[cfg(feature = "logging")]
		log::trace!("RPC `{}` (ID={})", method, id);

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
		let _ = self.tx.lock().await.send(Message::Text(msg)).await;

		// wait for the matching response to arrive
		let res = recv.await;
		// println!("RPC response: {:?}", &res);
		let res = res.map_err(|_| standard_error(StandardError::InternalError, None))?;
		res.result.ok_or(jsonrpc::Error::Rpc(
			res.error.unwrap_or(standard_error(StandardError::InternalError, None)),
		))
	}
}

impl<Tx> Backend<Tx> {
	async fn next_id(&self) -> Id {
		self.messages.lock().await.keys().last().unwrap_or(&0) + 1
	}
}

#[cfg(not(feature = "wss"))]
pub type WS2 = futures_util::sink::SinkErrInto<
	SplitSink<async_tungstenite::WebSocketStream<async_std::net::TcpStream>, Message>,
	Message,
	Error,
>;

#[cfg(feature = "wss")]
pub type WS2 = futures_util::sink::SinkErrInto<
	SplitSink<
		async_tungstenite::WebSocketStream<
			async_tungstenite::stream::Stream<
				async_std::net::TcpStream,
				async_tls::client::TlsStream<async_std::net::TcpStream>,
			>,
		>,
		Message,
	>,
	Message,
	Error,
>;

impl Backend<WS2> {
	pub async fn new(url: &str) -> core::result::Result<Self, WsError> {
		#[cfg(feature = "logging")]
		log::trace!("WS connecting to {}", url);

		let mut socket;
		let mut tries = 0;
		let (stream, _) = loop {
			socket = async_tungstenite::async_std::connect_async(url).await;
			if let Ok(socket) = socket {
				break socket
			} else if tries > 5 {
				socket?;
			}
			tries += 1;
			async_std::task::sleep(Duration::from_secs(2)).await;
		};

		let (tx, rx) = stream.split();

		let backend = Backend {
			tx: Mutex::new(tx.sink_err_into()),
			messages: Arc::new(Mutex::new(BTreeMap::new())),
			streams: Arc::new(Mutex::new(BTreeMap::new())),
		};

		backend.spawn_process_incoming_message_loop(rx);

		Ok(backend)
	}

	fn spawn_process_incoming_message_loop<Rx>(&self, mut rx: Rx)
	where
		Rx: Stream<Item = core::result::Result<Message, WsError>> + Unpin + Send + 'static,
	{
		let messages = self.messages.clone();
		let streams = self.streams.clone();

		task::spawn(async move {
			while let Some(msg) = rx.next().await {
				match msg {
					Ok(msg) => {
						#[cfg(feature = "logging")]
						log::trace!("Got WS message {}", msg);
						if let Ok(msg) = msg.to_text() {
							let res: rpc::Response =
								serde_json::from_str(msg).unwrap_or_else(|_| {
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
							} else {
								#[cfg(feature = "logging")]
								log::debug!("Got WS message without id: {}", msg);

								let res: Result<serde_json::Value, _> = serde_json::from_str(msg);

								if let Ok(res) = res {
									if let Some((subscription_id, state_changes)) =
										parse_changes(&res)
									{
										let mut streams = streams.lock().await;
										let sender = streams.get_mut(subscription_id).unwrap();
										sender.send(state_changes).await.expect("receiver waiting");
									}
								}
							}
						}
					},
					Err(_err) => {
						#[cfg(feature = "logging")]
						log::warn!("WS Error: {}", _err);
					},
				}
			}
			#[cfg(feature = "logging")]
			log::warn!("WS connection closed");
		});
	}
}

#[cfg(all(feature = "ws", not(feature = "wss")))]
#[cfg(test)]
mod tests {
	#[test]
	fn can_get_metadata() {
		unimplemented!("Use 'wss' feature for testing rather than 'ws'.");
	}
}

#[cfg(feature = "wss")]
#[cfg(test)]
mod tests {
	use crate::ws::{Backend, WS2};
	use async_std::stream::StreamExt;

	fn init() {
		let _ = env_logger::builder().is_test(true).try_init();
	}

	fn polkadot_backend() -> crate::PolkaPipe<Backend<WS2>> {
		let backend =
			async_std::task::block_on(crate::ws::Backend::new("wss://rpc.polkadot.io")).unwrap();
		crate::PolkaPipe { rpc: backend }
	}

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
		// This is statemint's scale encoded parachain id (1000)
	}

	#[test]
	fn can_get_state_changes() {
		init();
		async_std::task::block_on(testy());
	}

	async fn testy() {
		let key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
		let key = hex::decode(key).unwrap();
		let parachain = polkadot_backend();

		let mut payload_stream =
			async_std::task::block_on(parachain.subscribe_storage(&[&key[..]], None));

		for _ in 0..2 {
			let n = payload_stream.next().await.unwrap();
			println!("block #{}", hex::encode(&n.block));
			for c in n.changes {
				println!("key: {}", hex::encode(&c.0));
			}
		}
	}

	// #[test]
	// fn can_get_state_changes_asof() {
	// 	init();
	// 	async_std::task::block_on(testy_asof());
	// }

	// async fn testy_asof() {
	// 	let block_hash =
	// 		hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
	// 			.unwrap();
	// 	let key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
	// 	let key = hex::decode(key).unwrap();
	// 	let parachain = polkadot_backend();

	// 	let mut payload_stream =
	// 		async_std::task::block_on(parachain.subscribe_storage(&[&key[..]], Some(&block_hash)));

	// 	let n = payload_stream.next().await;
	// 	println!("{:?}", n);
	// }
}
