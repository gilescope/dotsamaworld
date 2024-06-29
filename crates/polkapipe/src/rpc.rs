use crate::prelude::*;
// use async_trait::async_trait;
pub use jsonrpc::{error, Response};
pub type RpcResult = Result<Box<serde_json::value::RawValue>, error::Error>;
use async_std::stream::Stream;
use core::str::FromStr;

/// Scale state changes
#[derive(Debug)]
pub struct StateChanges {
	pub block: Vec<u8>,
	pub changes: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Rpc defines types of backends that are remote and talk JSONRpc
// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Rpc {
	async fn rpc(&self, method: &str, params: &str) -> RpcResult;
}

pub trait Streamable {
	async fn stream(
		&self,
		method: &str,
		params: &str,
	) -> async_std::channel::Receiver<StateChanges>;
}

fn convert_params_raw(params: &[&str]) -> String {
	let mut msg = String::from("[");
	for p in params {
		let first = msg.len() == 1;
		if !first {
			msg.push(',')
		}
		msg.push_str(p);
	}
	msg.push(']');
	msg
}

fn extract_bytes(val: &serde_json::value::RawValue) -> crate::Result<Vec<u8>> {
	let val2 = serde_json::Value::from_str(val.get());
	if let Some(result_val) = val2.unwrap().get("result") {
		if let serde_json::Value::String(meta) = result_val {
			Ok(hex::decode(&meta[(1 + "0x".len())..meta.len() - 1])
				.unwrap_or_else(|_| panic!("shoudl be hex: {}", meta)))
		} else {
			#[cfg(feature = "logging")]
			log::warn!("RPC failure : {:?}", &result_val);
			Err(crate::Error::Node(format!("{:?}", result_val)))
		}
	} else {
		let meta = val.get();
		Ok(hex::decode(&meta[(1 + "0x".len())..meta.len() - 1])
			.unwrap_or_else(|_| panic!("should be hex: {}", meta)))
	}
}

pub fn parse_changes(value: &serde_json::Value) -> Option<(&str, StateChanges)> {
	if let serde_json::Value::Object(map) = value {
		if let Some(serde_json::Value::Object(params_map)) = map.get("params") {
			if let Some(serde_json::Value::String(subscription_id)) = params_map.get("subscription")
			{
				if let Some(serde_json::Value::Object(result)) = params_map.get("result") {
					if let Some(serde_json::Value::String(block)) = result.get("block") {
						if let Some(serde_json::Value::Array(changes)) = result.get("changes") {
							debug_assert!(block.starts_with("0x"));
							let block = hex::decode(&block[2..]).unwrap();
							let mut state_changes = StateChanges { block, changes: vec![] };

							for change in changes {
								if let serde_json::Value::Array(key_val) = change {
									debug_assert!(key_val.len() == 2);
									if let serde_json::Value::String(key) = &change[0] {
										let key = hex::decode(&key[2..]).unwrap();
										if let serde_json::Value::String(value) = &change[1] {
											let value = hex::decode(&value[2..]).unwrap();
											state_changes.changes.push((key, value));
										}
									}
								}
							}
							return Some((subscription_id.as_str(), state_changes))
						}
					}
				}
			}
		}
	}
	None
}

// subscription id used to unsubscribe
pub(crate) fn extract_subscription(val: &serde_json::value::RawValue) -> crate::Result<&str> {
	let val2 = serde_json::Value::from_str(val.get());
	if let Some(_result_val) = val2.unwrap().get("result") {
		panic!("unexpected");
	} else {
		let meta = val.get();
		Ok(&meta[1..meta.len() - 1])
	}
}

pub struct PolkaPipe<R: Rpc> {
	pub rpc: R,
}

// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<R: Rpc + Streamable> PolkaPipe<R> {
	pub async fn subscribe_storage(
		&self,
		keys: &[&[u8]],
		as_of: Option<&[u8]>,
	) -> impl Stream<Item = StateChanges> {
		let buf: String;
		let mut keys_encoded = String::from("[");
		let mut first = true;
		for key in keys {
			let key_encoded = hex::encode(key);
			#[cfg(feature = "logging")]
			log::debug!("StorageKey: {}", key_encoded);
			if !first {
				keys_encoded.push(',');
			}
			keys_encoded.push('"');
			keys_encoded.push_str(&key_encoded);
			keys_encoded.push('"');
			first = false;
		}
		keys_encoded.push(']');
		let mut params = vec![keys_encoded.as_str()];
		if let Some(block_hash) = as_of {
			buf = format!("\"{}\"", hex::encode(block_hash));
			params.push(&buf);
		}
		self.rpc.stream("state_subscribeStorage", &convert_params_raw(&params)).await
	}

	//state_queryStorage for multiple keys over a hash range.
	pub async fn query_storage(&self, key: &[u8], as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
		let key_enc = hex::encode(key);
		#[cfg(feature = "logging")]
		log::debug!("StorageKey encoded: {}", key_enc);
		let mut buf;
		let key = format!("\"{}\"", key_enc);
		let params = if let Some(block_hash) = as_of {
			buf = hex::encode(block_hash);
			buf = format!("\"{}\"", buf);
			vec![key.as_str(), buf.as_str()]
		} else {
			vec![key.as_str()]
		};

		let val = if as_of.is_some() {
			// state_queryStorageAt
			self.rpc
				.rpc("state_getStorage", &convert_params_raw(&params))
				.await
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::debug!("RPC failure: {}", &e);
					crate::Error::Node(e.to_string())
				})
		} else {
			self.rpc
				.rpc("state_getStorage", &format!("[\"0x{}\"]", key_enc))
				.await
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::debug!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		};
		let val = val?;
		extract_bytes(&val)
	}

	//state_queryStorage for multiple keys over a hash range.
	pub async fn query_state_call(
		&self,
		method: &str,
		key: &[u8],
		as_of: Option<&[u8]>,
	) -> crate::Result<Vec<u8>> {
		let key_enc = hex::encode(key);
		#[cfg(feature = "logging")]
		log::debug!("StorageKey encoded: {}", key_enc);
		let mut buf;
		let key = format!("\"0x{}\"", key_enc);
		let method_quoted = format!("\"{}\"", method);

		let params = if let Some(block_hash) = as_of {
			buf = hex::encode(block_hash);
			buf = format!("\"0x{}\"", buf);
			vec![method_quoted.as_str(), key.as_str(), buf.as_str()]
		} else {
			vec![method_quoted.as_str(), key.as_str()]
		};

		let val = if as_of.is_some() {
			// state_queryStorageAt
			self.rpc.rpc("state_call", &convert_params_raw(&params)).await.map_err(|e| {
				#[cfg(feature = "logging")]
				log::debug!("RPC failure: {}", &e);
				crate::Error::Node(e.to_string())
			})
		} else {
			self.rpc
				.rpc("state_call", &format!("[\"{}\", \"0x{}\"]", method, key_enc))
				.await
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::debug!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		};
		let val = val?;
		extract_bytes(&val)
	}

	pub async fn query_block_hash(&self, block_numbers: &[u32]) -> crate::Result<Vec<u8>> {
		let num: Vec<_> = block_numbers.iter().map(|i| i.to_string()).collect();
		let n: Vec<_> = num.iter().map(|i| i.as_str()).collect();

		let res = self.rpc.rpc("chain_getBlockHash", &convert_params_raw(&n)).await.map_err(|e| {
			#[cfg(feature = "logging")]
			log::warn!("RPC failure: {}", &e);
			crate::Error::Node(e.to_string())
		});
		let val = res?;
		extract_bytes(&val)
	}

	pub async fn query_block(
		&self,
		block_hash_in_hex: Option<&str>,
	) -> crate::Result<serde_json::value::Value> {
		if let Some(block_hash_in_hex) = block_hash_in_hex {
			let res = self.rpc.rpc("chain_getBlock", &format!("[\"{}\"]", block_hash_in_hex)).await;
			res.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::warn!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		} else {
			self.rpc
				.rpc("chain_getBlock", "[]")
				.await
				.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::warn!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		}
	}

	pub async fn state_get_keys(
		&self,
		key: &str,
		as_of: Option<&[u8]>,
	) -> crate::Result<serde_json::value::Value> {
		if let Some(as_of) = as_of {
			let buf = hex::encode(as_of);
			let buf = format!("\"0x{}\"", buf);
			let params = vec![key, buf.as_str()];

			self.rpc
				.rpc("state_getKeys", &convert_params_raw(&params))
				.await
				.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::debug!("RPC failure: {}", &e);
					crate::Error::Node(e.to_string())
				})
		} else {
			self.rpc
				.rpc("state_getKeys", &format!("[\"{}\"]", key))
				.await
				.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::warn!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		}
	}

	pub async fn state_get_keys_paged(
		&self,
		key: &str,
		count: u32,
		as_of: Option<&[u8]>,
	) -> crate::Result<serde_json::value::Value> {
		if let Some(as_of) = as_of {
			let buf = hex::encode(as_of);
			let buf = format!("\"0x{}\"", buf);
			let count = count.to_string();
			let params = vec![key, &count, buf.as_str()];

			self.rpc
				.rpc("state_getKeysPaged", &convert_params_raw(&params))
				.await
				.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::debug!("RPC failure: {}", &e);
					crate::Error::Node(e.to_string())
				})
		} else {
			self.rpc
				.rpc("state_getKeysPaged", &format!("[\"{}\", {}]", key, count))
				.await
				.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					#[cfg(feature = "logging")]
					log::warn!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		}
	}

	pub async fn query_metadata(&self, as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
		self.query_state_call("Metadata_metadata", b"", as_of).await.map(|mut v| {
			//TODO find a more efficient way
			v.remove(0);
			v.remove(0);
			v.remove(0);
			v.remove(0);
			v
		})
	}

	pub async fn submit(&self, ext: impl AsRef<[u8]> + Send) -> crate::Result<()> {
		let extrinsic = format!("\"0x{}\"", hex::encode(ext.as_ref()));
		#[cfg(feature = "logging")]
		log::debug!("Extrinsic: {}", extrinsic);

		let _res = self
			.rpc //could do author_submitAndWatchExtrinsic
			.rpc("author_submitExtrinsic", &convert_params_raw(&[&extrinsic]))
			.await
			.map_err(|e| crate::Error::Node(e.to_string()))?;

		#[cfg(feature = "logging")]
		log::debug!("Extrinsic {:x?}", _res);
		Ok(())
	}
}
