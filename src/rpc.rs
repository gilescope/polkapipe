use async_trait::async_trait;
use jsonrpc::serde_json::value::RawValue;
pub use jsonrpc::{error, Error, Request, Response};

// use crate::meta::{self, Metadata};
use crate::prelude::*;
use crate::Backend;
pub type RpcResult = Result<Vec<u8>, error::Error>;

/// Rpc defines types of backends that are remote and talk JSONRpc
#[async_trait]
pub trait Rpc: Backend + Send + Sync {
    async fn rpc(&self, method: &str, params: &[&str]) -> RpcResult;

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
    async fn query_storage(&self, key: &[u8]) -> crate::Result<Vec<u8>> {
        todo!();
        // let key = key.to_string();
        // log::debug!("StorageKey encoded: {}", key);
        // self.rpc("state_getStorage", &[&key]).await.map_err(|e| {
        //     log::debug!("RPC failure: {}", e);
        //     // NOTE it could fail for more reasons
        //     crate::Error::StorageKeyNotFound
        // })
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

    async fn metadata(&self, as_of: Option<&[u8]>) -> crate::Result<Vec<u8>> {
        let buf;
        let params = if let Some(block_hash) = as_of {
            buf = hex::encode(block_hash);
            vec![buf.as_str()]
        } else {
            vec![]
        };
        let meta = self
            .rpc("state_getMetadata", &params[..])
            .await
            .map_err(|e| crate::Error::Node(e.to_string()))?;

        log::trace!("Metadata {:#?}", meta);
        Ok(meta)
    }
}
