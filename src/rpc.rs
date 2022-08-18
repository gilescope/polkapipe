use crate::{prelude::*, Backend};
use async_trait::async_trait;
use jsonrpc::serde_json::value::RawValue;
pub use jsonrpc::{error, Error, Request, Response};
pub type RpcResult = Result<Vec<u8>, error::Error>;
use jsonrpc::error::RpcError;

/// Rpc defines types of backends that are remote and talk JSONRpc
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Rpc: Backend + Send + Sync {
	async fn rpc(&self, method: &str, params: Vec<Box<RawValue>>) -> RpcResult;

	/// Returns in json format rather than bytes.
	async fn rpc_single(
		&self,
		method: &str,
		params: Box<RawValue>,
	) -> Result<serde_json::value::Value, RpcError>;

	fn convert_params_to_strings(params: &[&str]) -> Vec<Box<RawValue>> {
		params
			.iter()
			.map(|p| format!("\"{}\"", p))
			.map(RawValue::from_string)
			.map(Result::unwrap)
			.collect::<Vec<_>>()
	}

	fn convert_params_raw(params: &[&str]) -> Vec<Box<RawValue>> {
		params
			.iter()
			.map(|p| p.to_string()) // TODO alloc less
			.map(RawValue::from_string)
			.map(Result::unwrap)
			.collect::<Vec<_>>()
	}
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<R: Rpc> Backend for R {
	//state_queryStorage for multiple keys over a hash range.
	async fn query_storage(&self, key: &[u8], as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
		let key = hex::encode(key);
		log::debug!("StorageKey encoded: {}", key);
		let buf;
		let params = if let Some(block_hash) = as_of {
			buf = hex::encode(block_hash);
			vec![key.as_str(), buf.as_str()]
		} else {
			vec![key.as_str()]
		};

		if as_of.is_some() {
			// state_queryStorageAt
			self.rpc("state_getStorage", Self::convert_params_to_strings(&params))
				.await
				.map_err(|e| {
					log::debug!("RPC failure: {}", &e);
					crate::Error::Node(e.to_string())
				})
		} else {
			let value = self
				.rpc_single(
					"state_getStorage",
					RawValue::from_string(format!("\"0x{}\"", key)).unwrap(),
				)
				.await
				.map_err(|e| {
					log::debug!("RPC failure: {:?}", &e);
					crate::Error::Node(e.message)
				});
			value.map(|result| {
				if let serde_json::value::Value::String(hex_scale) = result {
					hex::decode(&hex_scale[2..]).unwrap()
				} else {
					panic!("{:?}", result)
				}
			})
		}
	}

	async fn query_block_hash(&self, block_numbers: &[u32]) -> crate::Result<Vec<u8>> {
		let num: Vec<_> = block_numbers.iter().map(|i| i.to_string()).collect();
		let n: Vec<_> = num.iter().map(|i| i.as_str()).collect();

		let res = self.rpc("chain_getBlockHash", Self::convert_params_raw(&n)).await.map_err(|e| {
			log::warn!("RPC failure: {}", &e);
			crate::Error::Node(e.to_string())
		});
		log::info!("query_block_hash finished.");
		res
	}

	async fn query_block(
		&self,
		block_hash_in_hex: &str,
	) -> crate::Result<serde_json::value::Value> {
		self.rpc_single(
			"chain_getBlock",
			RawValue::from_string(format!("\"{}\"", block_hash_in_hex)).unwrap(),
		)
		.await
		.map_err(|e| {
			log::warn!("RPC failure: {:?}", &e);
			crate::Error::Node(e.message)
		})
	}

	async fn query_metadata(&self, as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
		let buf;
		let params = if let Some(block_hash) = as_of {
			buf = hex::encode(block_hash);
			vec![buf.as_str()]
		} else {
			vec![]
		};
		let meta = self
			.rpc("state_getMetadata", Self::convert_params_to_strings(&params[..]))
			.await
			.map_err(|e| crate::Error::Node(e.to_string()))?;

		log::trace!("Metadata {:#?}", meta);
		Ok(meta)
	}
}
