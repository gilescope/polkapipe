use crate::prelude::*;
// use async_trait::async_trait;
pub use jsonrpc::{error, Error, Request, Response};
pub type RpcResult = Result<Box<serde_json::value::RawValue>, error::Error>;
use async_std::stream::{Stream, StreamExt};
use core::str::FromStr;

/// Rpc defines types of backends that are remote and talk JSONRpc
// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Rpc {
	async fn rpc(&self, method: &str, params: &str) -> RpcResult;
}

pub trait Streamable {
	async fn stream(&self, method: &str, params: &str) -> async_std::channel::Receiver<jsonrpc::Response>;
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

pub struct PolkaPipe<R: Rpc> {
	pub rpc: R,
}

// #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
// #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<R: Rpc + Streamable> PolkaPipe<R> {
	pub async fn subscribe_storage(
		&self,
		key: &[u8],
		as_of: Option<&[u8]>,
	) -> impl Stream<Item = crate::Result<Vec<u8>>> {
		// Can't think how we could support http subscriptions?
		// unimplemented!("Please use a websocket implementation for subscriptions.")
				let key_enc = hex::encode(key);
				#[cfg(feature = "logging")]
				log::debug!("StorageKey encoded: {}", key_enc);
				let mut buf;
				let key = format!("\"{}\"", key_enc);
				let key2 = format!("[\"{}\"]", key_enc);
				let params = if let Some(block_hash) = as_of {
					buf = hex::encode(block_hash);
					buf = format!("\"{}\"", buf);
					vec![key2.as_str(), buf.as_str()]
				} else {	
					
					vec![key2.as_str()]
				};
				let res =
					self.rpc.stream("state_subscribeStorage", &convert_params_raw(&params))
						.await
						// .map_err(|e| {
						// 	#[cfg(feature = "logging")]
						// 	log::warn!("RPC failure: {}", &e);
						// 	crate::Error::Node(e.to_string())
						// })
						;
				// let val = res.unwrap();
				// extract_bytes(&val);
		//		stream::from_iter(1u8..6)
				// let v = async_std::stream::repeat(Ok(vec![0u8])).take(5);
				// v
				res.map(|item| {
					print!("item: {:?}", item);
					Ok(vec![])})
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

	pub async fn submit<T>(&self, ext: impl AsRef<[u8]> + Send) -> crate::Result<()> {
		let extrinsic = format!("0x{}", hex::encode(ext.as_ref()));
		#[cfg(feature = "logging")]
		log::debug!("Extrinsic: {}", extrinsic);

		let _res = self
			.rpc
			.rpc("author_submitExtrinsic", &convert_params_raw(&[&extrinsic]))
			.await
			.map_err(|e| crate::Error::Node(e.to_string()))?;

		#[cfg(feature = "logging")]
		log::debug!("Extrinsic {:x?}", _res);
		Ok(())
	}
}
