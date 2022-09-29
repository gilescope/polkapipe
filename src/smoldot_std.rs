use crate::prelude::*;
use async_std::sync::Mutex;
use async_trait::async_trait;
use core::{cell::RefCell, iter};
use futures::{channel::mpsc, prelude::*};
use jsonrpc::{
	error::{standard_error, StandardError},
	serde_json,
};
use serde_json::value::RawValue;
use smoldot_light::ChainId;
use std::sync::Arc;

use crate::rpc::{Rpc, RpcResult};

thread_local! {
	static CLIENT: RefCell<Option<smoldot_light::Client<smoldot_light::platform::async_std::AsyncStdTcpWebSocket>>> = RefCell::new(None);
}

pub struct Backend {
	chain_id: ChainId,
	results_channel: Arc<Mutex<futures_channel::mpsc::Receiver<std::string::String>>>,
}

impl Backend {
	pub fn new(chainspec: String) -> Self {
		let mut client = CLIENT.take();

		if client.is_none() {
			//TODO put in thread local instance.
			client = Some(smoldot_light::Client::<
				smoldot_light::platform::async_std::AsyncStdTcpWebSocket,
			>::new(smoldot_light::ClientConfig {
				// The smoldot client will need to spawn tasks that run in the background. In order
				// to do so, we need to provide a "tasks spawner".
				tasks_spawner: Box::new(move |_name, task| {
					async_std::task::spawn(task);
				}),
				system_name: env!("CARGO_PKG_NAME").into(),
				system_version: env!("CARGO_PKG_VERSION").into(),
			}));
		}

		let (json_rpc_responses_tx, json_rpc_responses_rx) = mpsc::channel(32);

		let mut client = client.unwrap();
		// Ask the client to connect to a chain.

		//TODO: Don't try and add the same chain twice
		let chain_id = client
			.add_chain(smoldot_light::AddChainConfig {
				// The most important field of the configuration is the chain specification. This is
				// a JSON document containing all the information necessary for the client to
				// connect to said chain.
				specification: &chainspec,

				// See above.
				// Note that it is possible to pass `None`, in which case the chain will not be able
				// to handle JSON-RPC requests. This can be used to save up some resources.
				json_rpc_responses: Some(json_rpc_responses_tx),

				// This field is necessary only if adding a parachain.
				potential_relay_chains: iter::empty(),

				// After a chain has been added, it is possible to extract a "database" (in the form
				// of a simple string). This database can later be passed back the next time the
				// same chain is added again.
				// A database with an invalid format is simply ignored by the client.
				// In this example, we don't use this feature, and as such we simply pass an empty
				// string, which is intentionally an invalid database content.
				database_content: "",

				// The client gives the possibility to insert an opaque "user data" alongside each
				// chain. This avoids having to create a separate `HashMap<ChainId, ...>` in
				// parallel of the client.
				// In this example, this feature isn't used. The chain simply has `()`.
				user_data: (),
			})
			.unwrap();
		// println!("got chain id {:?}", &chain_id);

		CLIENT.set(Some(client));

		Backend { chain_id, results_channel: Arc::new(Mutex::new(json_rpc_responses_rx)) }
	}
}

#[async_trait]
impl Rpc for Backend {
	/// HTTP based JSONRpc request expecting an hex encoded result
	async fn rpc(&self, method: &str, params: &str) -> RpcResult {
		let id = 1u32;
		let msg = format!(
			"{{\"id\":{}, \"jsonrpc\": \"2.0\", \"method\":\"{}\", \"params\":{}}}",
			id, method, params
		);
		// println!("request {}", &msg);
		let mut client = CLIENT.take().unwrap();
		client.json_rpc_request(msg, self.chain_id).unwrap();
		CLIENT.set(Some(client));

		let response = self.results_channel.lock().await.next().await;

		if let Some(res) = response {
			// println!("JSON-RPC response: {:?}", &res);
			return Ok(RawValue::from_string(res).unwrap())
		}
		Err(jsonrpc::Error::Rpc(standard_error(StandardError::InternalError, None)))
	}
}

/// For smoldot tests we just check that we can retrieve the latest bits.
#[cfg(test)]
mod tests {
	use crate::Backend;
	fn init() {
		let _ = env_logger::builder().is_test(true).try_init();
	}

	fn polkadot_backend() -> super::Backend {
		super::Backend::new(include_str!("../chainspecs/polkadot.json").to_string())
	}

	#[test]
	fn can_get_metadata() {
		init();
		let latest_metadata =
			async_std::task::block_on(polkadot_backend().query_metadata(None)).unwrap();
		assert!(latest_metadata.len() > 0);
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
		let block_bytes = async_std::task::block_on(polkadot_backend().query_block(None)).unwrap();
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

		let as_of_events =
			async_std::task::block_on(polkadot_backend().query_storage(&key[..], None)).unwrap();
		assert!(as_of_events.len() > 0);
	}
}
