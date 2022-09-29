use crate::{prelude::*, Backend};
use async_trait::async_trait;
pub use jsonrpc::{error, Error, Request, Response};
pub type RpcResult = Result<Box<serde_json::value::RawValue>, error::Error>;
use core::str::FromStr;

/// Rpc defines types of backends that are remote and talk JSONRpc
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Rpc: Backend + Send + Sync {
	async fn rpc(&self, method: &str, params: &str) -> RpcResult;

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
}

fn extract_bytes(val: &serde_json::value::RawValue) -> crate::Result<Vec<u8>> {
	let val2 = serde_json::Value::from_str(val.get());
	if let Some(result_val) = val2.unwrap().get("result") {
		if let serde_json::Value::String(meta) = result_val {
			Ok(hex::decode(&meta[(1 + "0x".len())..meta.len() - 1])
				.unwrap_or_else(|_| panic!("shoudl be hex: {}", meta)))
		} else {
			log::warn!("RPC failure : {:?}", &result_val);
			Err(crate::Error::Node(format!("{:?}", result_val)))
		}
	} else {
		let meta = val.get();
		Ok(hex::decode(&meta[(1 + "0x".len())..meta.len() - 1])
			.unwrap_or_else(|_| panic!("shoudl be hex: {}", meta)))
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<R: Rpc> Backend for R {
	//state_queryStorage for multiple keys over a hash range.
	async fn query_storage(&self, key: &[u8], as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
		let key_enc = hex::encode(key);
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
			self.rpc("state_getStorage", &Self::convert_params_raw(&params))
				.await
				.map_err(|e| {
					log::debug!("RPC failure: {}", &e);
					crate::Error::Node(e.to_string())
				})
		} else {
			self.rpc("state_getStorage", &format!("[\"0x{}\"]", key_enc))
				.await
				.map_err(|e| {
					log::debug!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		};
		let val = val?;
		extract_bytes(&val)
	}

	async fn query_block_hash(&self, block_numbers: &[u32]) -> crate::Result<Vec<u8>> {
		let num: Vec<_> = block_numbers.iter().map(|i| i.to_string()).collect();
		let n: Vec<_> = num.iter().map(|i| i.as_str()).collect();

		let res =
			self.rpc("chain_getBlockHash", &Self::convert_params_raw(&n))
				.await
				.map_err(|e| {
					log::warn!("RPC failure: {}", &e);
					crate::Error::Node(e.to_string())
				});
		let val = res?;
		extract_bytes(&val)
	}

	async fn query_block(
		&self,
		block_hash_in_hex: Option<&str>,
	) -> crate::Result<serde_json::value::Value> {
		if let Some(block_hash_in_hex) = block_hash_in_hex {
			let res = self.rpc("chain_getBlock", &format!("[\"{}\"]", block_hash_in_hex)).await;
			res.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					log::warn!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		} else {
			self.rpc("chain_getBlock", "[]")
				.await
				.map(|raw_val| serde_json::Value::from_str(raw_val.get()).unwrap())
				.map_err(|e| {
					log::warn!("RPC failure: {:?}", &e);
					crate::Error::Node(format!("{}", e))
				})
		}
	}

	async fn query_metadata(&self, as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
		let mut buf;
		let params = if let Some(block_hash) = as_of {
			buf = hex::encode(block_hash);
			buf = format!("\"{}\"", buf);
			vec![buf.as_str()]
		} else {
			vec![]
		};
		let meta = self
			.rpc("state_getMetadata", &Self::convert_params_raw(&params[..]))
			.await
			.map_err(|e| crate::Error::Node(e.to_string()))?;

		log::trace!("Metadata {:#?}", meta);
		extract_bytes(&meta)
	}
}
