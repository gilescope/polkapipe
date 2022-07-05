use async_trait::async_trait;
use jsonrpc::serde_json::value::RawValue;
pub use jsonrpc::{error, Error, Request, Response};

// use crate::meta::{self, Metadata};
use crate::{prelude::*, Backend};
pub type RpcResult = Result<Vec<u8>, error::Error>;

/// Rpc defines types of backends that are remote and talk JSONRpc
#[async_trait]
pub trait Rpc: Backend + Send + Sync {
	async fn rpc(&self, method: &str, params: Vec<Box<RawValue>>) -> RpcResult;

	async fn rpc_single(&self, method: &str, params: Box<RawValue>) -> RpcResult;

	fn convert_params(params: &[&str]) -> Vec<Box<RawValue>> {
		params
			.iter()
			.map(|p| format!("\"{}\"", p))
			.map(RawValue::from_string)
			.map(Result::unwrap)
			.collect::<Vec<_>>()
	}
}

#[async_trait]
impl<R: Rpc> Backend for R {
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

		// state_queryStorageAt
		self.rpc("state_getStorage", Self::convert_params(&params)).await.map_err(|e| {
			log::debug!("RPC failure: {}", &e);
			crate::Error::Node(e.to_string())
		})
	}

	async fn query_block_hash(&self, block_numbers: &[u32]) -> crate::Result<Vec<u8>> {
		let num: Vec<_> = block_numbers.iter().map(|i| i.to_string()).collect();
		// let params = block_numbers;/
		let n: Vec<_> = num.iter().map(|i| i.as_str()).collect();

		self.rpc("chain_getBlockHash", Self::convert_params(&n)).await.map_err(|e| {
			log::debug!("RPC failure: {}", &e);
			crate::Error::Node(e.to_string())
		})
	}

	async fn query_block(&self, block_hash_in_hex: &str) -> crate::Result<Vec<u8>> {
		// let hash = hex::encode(block_hash);
		// let params = vec![block_hash_in_hex];

		self.rpc_single(
			"chain_getBlock",
			RawValue::from_string(format!("\"{}\"", block_hash_in_hex)).unwrap(),
		)
		.await
		.map_err(|e| {
			log::debug!("RPC failure: {}", &e);
			crate::Error::Node(e.to_string())
		})
	}

	// async fn submit<T>(&self, ext: T) -> crate::Result<()>
	// where
	//     T: AsRef<[u8]> + Send,
	// {
	//     let extrinsic = format!("0x{}", hex::encode(ext.as_ref()));
	//     log::debug!("Extrinsic: {}", extrinsic);

	//     let res = self
	//         .rpc("author_submitExtrinsic", &[&extrinsic])
	//         .await
	//         .map_err(|e| crate::Error::Node(e.to_string()))?;
	//     log::debug!("Extrinsic {:x?}", res);
	//     Ok(())
	// }

	async fn query_metadata(&self, as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
		let buf;
		let params = if let Some(block_hash) = as_of {
			buf = hex::encode(block_hash);
			vec![buf.as_str()]
		} else {
			vec![]
		};
		let meta = self
			.rpc("state_getMetadata", Self::convert_params(&params[..]))
			.await
			.map_err(|e| crate::Error::Node(e.to_string()))?;

		log::trace!("Metadata {:#?}", meta);
		Ok(meta)
	}
}
