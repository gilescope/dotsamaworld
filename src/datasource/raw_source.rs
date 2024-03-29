use crate::log;
use primitive_types::H256;
#[cfg(not(target_arch = "wasm32"))]
use {
	async_tungstenite::{tungstenite::Message, WebSocketStream},
	futures::{sink::SinkErrInto, stream::SplitSink},
	parity_scale_codec::Encode,
};

#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
pub struct AgnosticBlock {
	pub block_number: u32,
	pub extrinsics: Vec<Vec<u8>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl AgnosticBlock {
	pub fn to_vec(&self) -> Vec<u8> {
		self.encode()
	}

	pub fn from_bytes(mut bytes: &[u8]) -> Result<Self, parity_scale_codec::Error> {
		parity_scale_codec::Decode::decode(&mut bytes)
	}
}

/// A way to source untransformed raw data.
pub trait Source {
	async fn fetch_block_hash(&mut self, block_number: u32) -> Result<Option<H256>, BError>;

	async fn fetch_block(
		&mut self,
		block_hash: Option<H256>,
	) -> Result<Option<AgnosticBlock>, BError>;

	async fn fetch_storage(
		&mut self,
		key: &[u8],
		as_of: Option<H256>,
	) -> Result<Option<Vec<u8>>, BError>;

	async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<Option<Vec<u8>>, ()>;

	fn url(&self) -> &str;

	// #[cfg(target_arch="wasm32")]
	// async fn process_incoming_messages(&mut self) -> WSBackend;
}

// pub struct RawDataSource {
// 	ws_url: String,
// 	api: Option<RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>,
// }

// type BError = subxt::GenericError<std::convert::Infallible>; // Box<dyn std::error::Error>;

// /// This is the only type that should know about subxt
// impl RawDataSource {
// 	pub fn new(url: &str) -> Self {
// 		RawDataSource { ws_url: url.to_string(), api: None }
// 	}

// 	async fn client(&mut self) -> &mut Client<DefaultConfig> {
// 		&mut self.get_api().await.client
// 	}

// 	async fn get_api(&mut self) -> &mut RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>> {
// 		if self.api.is_some() {
// 			return self.api.as_mut().unwrap()
// 		}

// 		const MAX_RETRIES: usize = 6;
// 		let mut retries = 0;
// 		// println!("retries1 {}", retries);
// 		let client = loop {
// 			// println!("retries2 {}", retries);
// 			if retries >= MAX_RETRIES {
// 				println!("Cannot connect to substrate node after {} retries", retries);
// 			}
// 			// println!("retries {}", retries);

// 			// It might take a while for substrate node that spin up the RPC server.
// 			// Thus, the connection might get rejected a few times.
// 			let res = ClientBuilder::new().set_url(&self.ws_url).build().await;

// 			match res {
// 				Ok(res) => break res,
// 				_ => {
// 					async_std::task::sleep(std::time::Duration::from_secs(1 << retries)).await;
// 					retries += 1;
// 				},
// 			};
// 		};

// 		// println!("Client created........................");
// 		self.api = Some(
// 			client
// 				.to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>>(
// 				),
// 		);
// 		self.api.as_mut().unwrap()
// 	}
// }

// #[async_trait(?Send)]
// impl Source for RawDataSource {
// 	async fn fetch_block_hash(
// 		&mut self,
// 		block_number: u32,
// 	) -> Result<Option<sp_core::H256>, BError> {
// 		self.get_api().await.client.rpc().block_hash(Some(block_number.into())).await
// 	}

// 	/// Return then in bin form rather than link to subxt:
// 	/// subxt::sp_runtime::generic::SignedBlock<
// 	///     subxt::sp_runtime::generic::Block<
// 	///         subxt::sp_runtime::generic::Header<
// 	///             u32,
// 	///             subxt::sp_runtime::traits::BlakeTwo256
// 	///         >,
// 	///         subxt::sp_runtime::OpaqueExtrinsic
// 	///
// 	async fn fetch_block(
// 		&mut self,
// 		block_hash: Option<H256>,
// 	) -> Result<Option<AgnosticBlock>, BError> {
// 		let result = self.get_api().await.client.rpc().block(block_hash).await;
// 		if let Ok(Some(block_body)) = result {
// 			//TODO: we're decoding and encoding here. cut it out.
// 			Ok(Some(AgnosticBlock {
// 				block_number: block_body.block.header.number,
// 				extrinsics: block_body
// 					.block
// 					.extrinsics
// 					.into_iter()
// 					.map(|ex| ex.encode())
// 					.collect::<Vec<_>>(),
// 			}))
// 		} else {
// 			if let Err(err) = result {
// 				Err(err)
// 			} else {
// 				Ok(None)
// 			}
// 		}
// 	}

// 	async fn fetch_chainname(&mut self) -> Result<Option<String>, BError> {
// 		self.client().await.rpc().system_chain().await.map(|res| Some(res))
// 	}

// 	async fn fetch_storage(
// 		&mut self,
// 		key: subxt::sp_core::storage::StorageKey,
// 		as_of: Option<H256>,
// 	) -> Result<Option<subxt::sp_core::storage::StorageData>, BError> {
// 		self.client().await.storage().fetch_raw(key, as_of).await
// 	}

// 	async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<Option<sp_core::Bytes>, ()> {
// 		let mut params = None;
// 		if let Some(hash) = as_of {
// 			params = Some(jsonrpsee_types::ParamsSer::Array(vec![serde_json::Value::String(
// 				hex::encode(hash.as_bytes()),
// 			)]));
// 		}

// 		//        self.client().rpc().metadata_bytes(as_of).await
// 		//TODO: if asof is none then client.Metadata could just be encoded.
// 		let res = self
// 			.get_api()
// 			.await
// 			.client
// 			.rpc()
// 			.client
// 			.request("state_getMetadata", params.clone())
// 			.await;
// 		match res {
// 			Ok(res) => return Ok(Some(res)),
// 			_ => return Err(()),
// 		};
// 	}

// 	/// We subscribe to relay chains and self sovereign chains
// 	async fn subscribe_finalised_blocks(
// 		&mut self,
// 	) -> Result<
// 		// Subscription<
// 		//     subxt::sp_runtime::generic::Header<u32, subxt::sp_runtime::traits::BlakeTwo256>,
// 		// >
// 		Box<dyn futures::Stream<Item = Result<H256, ()>> + Unpin>,
// 		(),
// 	> {
// 		let result = self.get_api().await.client.rpc().subscribe_finalized_blocks().await;
// 		if let Ok(sub) = result {
// 			// sub is a Stream... can we map a stream?
// 			Ok(Box::new(sub.map(|block_header_result| {
// 				if let Ok(block_header) = block_header_result {
// 					let block_header: subxt::sp_runtime::generic::Header<
// 						u32,
// 						subxt::sp_runtime::traits::BlakeTwo256,
// 					> = block_header;
// 					Ok(block_header.hash())
// 				} else {
// 					Err(())
// 				}
// 			})))
// 		} else {
// 			Err(())
// 		}
// 	}

// 	fn url(&self) -> &str {
// 		&self.ws_url
// 	}
// }

// #[cfg(target_arch="wasm32")]
// use ws_stream_wasm::WsStream;
// #[cfg(target_arch="wasm32")]
// type Message = ws_stream_wasm::WsMessage;
// #[cfg(target_arch="wasm32")]
// type WS2 = SplitSink<WsStream, ws_stream_wasm::WsMessage>;

#[cfg(target_arch = "wasm32")]
pub(crate) type WSBackend = polkapipe::ws_web::Backend;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type WSBackend = polkapipe::ws::Backend<
	SinkErrInto<
		SplitSink<
			WebSocketStream<
				async_tungstenite::stream::Stream<
					async_std::net::TcpStream,
					async_tls::client::TlsStream<async_std::net::TcpStream>,
				>,
			>,
			Message,
		>,
		Message,
		polkapipe::Error,
	>,
>;

//#[derive(Clone)]
pub struct RawDataSource {
	ws_url: Vec<String>,
	client: Option<polkapipe::PolkaPipe::<WSBackend>>,
}

type BError = polkapipe::Error;
// type BError = subxt::GenericError<std::convert::Infallible>; // Box<dyn std::error::Error>;

/// This is the only type that should know about subxt
impl RawDataSource {
	pub fn new(url: Vec<String>) -> Self {
		RawDataSource { ws_url: url, client: None }
	}

	#[cfg(target_arch = "wasm32")]
	async fn client(&mut self) -> Option<&mut polkapipe::PolkaPipe::<WSBackend>> {
		if self.client.is_none() {
			let urls: Vec<_> = self.ws_url.iter().map(|s| s.as_ref()).collect();
			if let Ok(client) = polkapipe::ws_web::Backend::new(urls.as_slice()).await  {
				self.client = Some(polkapipe::PolkaPipe::<polkapipe::ws_web::Backend>{rpc:client});
			}
		}
		self.client.as_mut()
	}

	#[cfg(not(target_arch = "wasm32"))]
	async fn client(&mut self) -> Option<&mut polkapipe::PolkaPipe::<WSBackend>> {
		if self.client.is_none() {
			if let Ok(client) = polkapipe::ws::Backend::new(&self.ws_url[0]).await {
				self.client = Some(polkapipe::PolkaPipe::<WSBackend>{rpc:client});
			}
		}
		self.client.as_mut()
	}
}

// #[async_trait(?Send)]
// trait ProcessIncoming {
// 	async fn process_incoming_messages(&self);
// }

impl Source for RawDataSource {
	// #[cfg(target_arch="wasm32")]
	// async fn process_incoming_messages(&mut self) -> WSBackend {
	// 	log!("get message processor");

	// 	// while self.client.is_none() {
	// 	// 	async_std::task::yield_now().await;
	// 	// }
	// 	// log!("await process incoming messages");
	// 	let client = self.client();
	// 		if let Some(_client) = client.await {
	// 			log!("got message processor");
	// 			self.client.as_ref().unwrap().clone()
	// 		} else { panic!("no client could be gottet")}
	// 	// log!("finish process incoming messages");
	// }

	async fn fetch_block_hash(
		&mut self,
		block_number: u32,
	) -> Result<Option<primitive_types::H256>, BError> {
		// log!("get client");
		if let Some(client) = self.client().await {
			// log!("got client");
			client
				.query_block_hash(&[block_number])
				.await
				.map(|res| Some(H256::from_slice(&res[..])))
		} else {
			log!("could not get client");
			Err(polkapipe::Error::Node(format!("can't get client for {}", self.ws_url[0])))
		}
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
	) -> Result<Option<AgnosticBlock>, BError> {
		if let Some(client) = self.client().await {
			let opt = block_hash.map(|b| hex::encode(b.as_bytes()));
			let result = client.query_block(opt.as_deref()).await;

			if let Ok(serde_json::value::Value::Object(map)) = &result {
				// log!("block = {:?}", map);
				if let Some(serde_json::value::Value::Object(map)) = map.get("block") {
					let mut res = AgnosticBlock { block_number: 0, extrinsics: vec![] };
					if let Some(serde_json::value::Value::Object(m)) = map.get("header") {
						// log!("header = {:?}", m);
						if let Some(serde_json::value::Value::String(num_original)) = m.get("number") {
							 let mut num = num_original.trim_start_matches("0x").to_string();
							if num.len() % 2 == 1 {
								// println!("odd found {}", num_original);
								num = format!("0{}", num);
							}

							let _bytes = hex::decode(&num).unwrap();

						//	bytes.reverse(); //TODO: why is this needed? it gets the right number but...
							/* while bytes.len() < 4 {
								bytes.insert(0, 0);
							} */
							// println!("bytes or {}", num_original);
							// println!("bytes is {}", hex::encode(&bytes));
							// use parity_scale_codec::Decode; 

						   let number: u32 = u32::from_str_radix(num_original.trim_start_matches("0x"), 16).unwrap();
//							let number = u32::decode(&mut &bytes[..]).unwrap();

							/* let mut b = [0,0,0,0];
							for (i, bb) in bytes.iter().enumerate() {
								b[i] = *bb;
							} */
							/* use parity_scale_codec::Compact;
					/* 		 */let number = Compact::<u32>::decode(&mut &bytes[..]).unwrap(); */
						/* 	/*  */let re : u32 = number.into(); */
					// println!("bytes {} -> {}",&num_original, number);   
							res.block_number = number;
						}
					}
					if let Some(serde_json::value::Value::Array(extrinsics)) = map.get("extrinsics")
					{
						for ex in extrinsics {
							if let serde_json::value::Value::String(val) = ex {
								/* println!("about to decode '{}'", &val); */
								res.extrinsics
									.push(hex::decode(val.trim_start_matches("0x")).unwrap());
							} else {
								panic!()
							}
						}
						// println!("got 4here aa{}", extrinsics.len());
					}
					return Ok(Some(res))
				}
			}
			result.map(|_| None)
		} else {
			Err(polkapipe::Error::Node(format!("can't get client for {}", self.ws_url[0])))
		}
		// //TODO: we're decoding and encoding here. cut it out.
		// Ok(Some(AgnosticBlock {
		// 	block_number: block_body.block.header.number,
		// 	extrinsics: block_body
		// 		.block
		// 		.extrinsics
		// 		.into_iter()
		// 		.map(|ex| ex.encode())
		// 		.collect::<Vec<_>>(),
		// }))
		// } else {
		// 	if let Err(err) = result {
		// 		Err(err)
		// 	} else {
		// 		Ok(None)
		// 	}
		// }
	}

	async fn fetch_storage(
		&mut self,
		key: &[u8],
		as_of: Option<H256>,
	) -> Result<Option<Vec<u8>>, BError> {
		if let Some(client) = self.client().await {
			if let Some(as_of) = as_of {
				client.query_storage(key, Some(as_of.as_bytes())).await.map(Some)
			} else {
				client.query_storage(key, None).await.map(Some)
			}
		} else {
			Err(polkapipe::Error::Node(format!("can't get client for {}", self.ws_url[0])))
		}
	}

	async fn fetch_metadata(&mut self, as_of: Option<H256>) -> Result<Option<Vec<u8>>, ()> {
		if let Some(client) = self.client().await {
			if let Some(as_of) = as_of {
				client.query_metadata(Some(as_of.as_bytes())).await.map(Some).map_err(|_e| ())
			} else {
				client.query_metadata(None).await.map(Some).map_err(|_e| ())
			}
		} else {
			Err(())
		}
	}

	fn url(&self) -> &str {
		&self.ws_url[0]
	}
}

#[cfg(test)]
mod tests {
	use parity_scale_codec::Encode;

	#[test]
	fn testit() {
		/* let  hex::decode("03ee6c")/*  */.unwrap(); */
		let r: u32 = 10504599;
		let should = r.encode();
		println!("bytes should be {:?}", &should); //bytes [160, 73, 151]

		let bytes = hex::decode("00a04997").unwrap();
		println!("bytes {:?}", &bytes); //bytes [160, 73, 151]
								/* /*  */use parity_scale_codec::Decode; */
		use parity_scale_codec::Decode;
		let mut bytes_rev = bytes.clone();
		bytes_rev.reverse();
		let xg = u32::decode(&mut bytes_rev.as_slice());
		println!("res={:?}.", xg);
		/* /* let x = <u32 as parity_scale_codec::/* Decode */>::decode(&mut
		 * &bytes[..]).unwrap(); */ */
	}
}
