use crate::prelude::*;
use async_trait::async_trait;
// use core::{convert::TryInto, fmt};
use jsonrpc::{
	error::{standard_error, RpcError, StandardError},
	serde_json,
	// serde_json::{to_string, value::to_raw_value},
};
use std::str::FromStr;
use serde_json::value::RawValue;
// pub use surf::Url;
use async_std::sync::Mutex;
use core::iter;
use futures::{channel::mpsc, prelude::*};
use smoldot_light::ChainId;
use std::sync::Arc;

use crate::rpc::{self, Rpc, RpcResult};

pub struct Backend {
	client:
		Arc<Mutex<smoldot_light::Client<smoldot_light::platform::async_std::AsyncStdTcpWebSocket>>>,
	chain_id: ChainId,
	results_channel: Arc<Mutex<futures_channel::mpsc::Receiver<std::string::String>>>,
}

impl Backend {
	pub fn new(chainspec: String) -> Self {
		//TODO put in thread local instance.
		let mut client = smoldot_light::Client::<
			smoldot_light::platform::async_std::AsyncStdTcpWebSocket,
		>::new(smoldot_light::ClientConfig {
			// The smoldot client will need to spawn tasks that run in the background. In order to
			// do so, we need to provide a "tasks spawner".
			tasks_spawner: Box::new(move |_name, task| {
				async_std::task::spawn(task);
			}),
			system_name: env!("CARGO_PKG_NAME").into(),
			system_version: env!("CARGO_PKG_VERSION").into(),
		});

		let (json_rpc_responses_tx, json_rpc_responses_rx) = mpsc::channel(32);

		// Ask the client to connect to a chain.
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
		println!("got chain id {:?}", &chain_id);
		Backend {
			client: Arc::new(Mutex::new(client)),
			chain_id,
			results_channel: Arc::new(Mutex::new(json_rpc_responses_rx)),
		}
	}
}

#[async_trait]
impl Rpc for Backend {
	/// HTTP based JSONRpc request expecting an hex encoded result
	async fn rpc(&self, method: &str, params: Vec<Box<RawValue>>) -> RpcResult {
		let id = 1u32;
		let msg = serde_json::to_string(&rpc::Request {
			id: id.into(),
			jsonrpc: Some("2.0"),
			method,
			params: &params,
		})
		.expect("Request is serializable");

		// 
		// let msg = r#"{"id":1,"jsonrpc":"2.0","method":"state_getMetadata","params":["0xe33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16"]}"#;
		// let msg = r#"{"method":"state_getMetadata","params":{"hash":"e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16"},"id":1,"jsonrpc":"2.0"}"#;
		println!("request {}", &msg);
		self.client.lock().await.json_rpc_request(msg, self.chain_id).unwrap();

		//     self.client.lock().await
		// .json_rpc_request(
		//     r#"{"id":1,"jsonrpc":"2.0","method":"chain_subscribeNewHeads","params":[]}"#,
		//     self.chain_id,
		// )
		// .unwrap();

		// log::info!("Smoldot `{}` to {}", method, &self.0);
		// let req = surf::post(&self.0).content_type("application/json").body(
		// 	to_string(&rpc::Request { id: 1.into(), jsonrpc: Some("2.0"), method, params })
		// 		.unwrap(),
		// );
		// let client = surf::client().with(surf::middleware::Redirect::new(2));
		// let mut res = client
		// 	.send(req)
		// 	.await
		// 	.map_err(|err| rpc::Error::Transport(err.into_inner().into()))?;

		// let status = res.status();
		// let res = if status.is_success() {
		// 	res.body_json::<rpc::Response>()
		// 		.await
		// 		.map_err(|err| {
		// 			standard_error(
		// 				StandardError::ParseError,
		// 				Some(to_raw_value(&err.to_string()).unwrap()),
		// 			)
		// 		})?
		// 		.result::<String>()?
		// } else {
		// 	log::debug!("RPC HTTP status: {}", res.status());
		// 	let err = res.body_string().await.unwrap_or_else(|_| status.canonical_reason().into());
		// 	let err = to_raw_value(&err).expect("error string");

		// 	return Err(if status.is_client_error() {
		// 		standard_error(StandardError::InvalidRequest, Some(err)).into()
		// 	} else {
		// 		standard_error(StandardError::InternalError, Some(err)).into()
		// 	})
		// };
		println!("made call");
		let response = self.results_channel.lock().await.next().await;
		// if let Some(res) = &response {
			
		// }

		if let Some(res) = &response {
			println!("JSON-RPC response: {:?}", res);
			let res_json = serde_json::Value::from_str(&res).unwrap();
			
			if let Some(serde_json::Value::String(val)) = res_json.get("result") {
				return Ok(hex::decode(&val.strip_prefix("0x").unwrap_or(val)).unwrap());
			} else {
				println!("no result found in the json response");
			}
		} 
		Err(jsonrpc::Error::Rpc(standard_error(StandardError::InternalError, None)))
	}

	async fn rpc_single(
		&self,
		method: &str,
		params: Box<RawValue>,
	) -> Result<serde_json::value::Value, RpcError> {
		self.rpc(method, vec![params])
			.await
			.map(|bytes| String::from_utf8(bytes).unwrap())
			.map(|s| serde_json::value::Value::from(s))
			.map_err(|_| standard_error(StandardError::InternalError, None))
	}
}

#[cfg(test)]
mod tests {
	use crate::Backend;
	// use async_tungstenite::WebSocketStream;
	// use futures_util::sink::SinkErrInto;

	fn polkadot_backend() -> super::Backend {
		super::Backend::new(include_str!("../chainspecs/polkadot.json").to_string())
	}

	#[test]
	fn can_get_metadata() {
		    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
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

	#[test]
	fn can_get_block_hash() {
		// env_logger::init();
		let polkadot = polkadot_backend();
		std::thread::sleep(std::time::Duration::from_secs(10));
		let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![1])).unwrap();
		println!("{:?}", String::from_utf8(hash.clone()));
		assert_eq!(
			"c0096358534ec8d21d01d34b836eed476a1c343f8724fa2153dc0725ad797a90",
			hex::encode(hash)
		);

		// let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![10504599])).unwrap();
		// assert_eq!(
		// 	"e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16",
		// 	hex::encode(hash)
		// );
	}

	// #[test]
	// fn can_get_full_block() {
	// 	let hash = "c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a";
	// 	let block_bytes =
	// 		async_std::task::block_on(polkadot_backend().query_block(Some(hash))).unwrap();
	// 	assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	// }

	#[test]
	fn can_get_latest_block() {
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
		// env_logger::init();
		
		let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
		let key = hex::decode(events_key).unwrap();

		let as_of_events = async_std::task::block_on(
			polkadot_backend().query_storage(&key[..], None),
		)
		.unwrap();
		assert!(as_of_events.len() > 0);
	}
}
