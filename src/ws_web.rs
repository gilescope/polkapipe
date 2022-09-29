use alloc::{boxed::Box, sync::Arc};
use async_std::sync::Mutex;
use async_trait::async_trait;
use core::time::Duration;
use futures::{stream::StreamExt, SinkExt};
use jsonrpc::{
	error::{result_to_response, standard_error, StandardError},
	serde_json,
};
use log::info;
use wasm_bindgen::UnwrapThrowExt;
use ws_stream_wasm::*;

use crate::{
	rpc::{self, Rpc, RpcResult},
	Error,
};

type Message = WsMessage;
type Id = u8;

#[derive(Clone)]
pub struct Backend {
	stream: Arc<Mutex<WsStream>>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Rpc for Backend {
	async fn rpc(&self, method: &str, params: &str) -> RpcResult {
		let id = 1; //TODO: we can do better
		log::trace!("RPC normal `{}`", method);

		// send rpc request
		let msg = format!(
			"{{\"id\":{}, \"jsonrpc\": \"2.0\", \"method\":\"{}\", \"params\":{}}}",
			id, method, params
		);
		log::trace!("RPC Request {} ...", &msg[..msg.len().min(150)]);
		{
			let mut lock = self.stream.lock().await;
			log::trace!("RPC got lock now sending {} ...", &msg[..50]);
			let _ = lock.send(Message::Text(msg)).await;
		}
		log::trace!("RPC now waiting for response ...");

		loop {
			let res = self.stream.lock().await.next().await;
			//TODO might be picked up by someone else...
			if let Some(msg) = res {
				log::trace!("Got WS message {:?}", msg);
				if let Message::Text(msg) = msg {
					log::trace!("{:#?}", msg);
					let res: rpc::Response = serde_json::from_str(&msg).unwrap_or_else(|_| {
						result_to_response(
							Err(standard_error(StandardError::ParseError, None)),
							().into(),
						)
					});
					if res.id.is_u64() {
						let res_id = res.id.as_u64().unwrap() as Id;
						log::trace!("Answering request {}", res_id);
						if res_id == id {
							return Ok(res.result.unwrap())
						} else {
							todo!("At the moment a socket is only used in order");
						}
					}
				}
			} else {
				log::error!("Got WS error: {:?}", res);
				// If you're getting errors, slow down.
				async_std::task::sleep(Duration::from_secs(2)).await
			}
		}
	}
}

impl Backend {
	pub async fn new_ws2(url: &str) -> core::result::Result<Self, Error> {
		log::info!("WS connecting to {}", url);
		let (_wsmeta, stream) =
			WsMeta::connect(url, None).await.expect_throw("assume the connection succeeds");

		let backend = Backend {
			stream: Arc::new(Mutex::new(stream)),
			// wsmeta: Arc::new(Mutex::new(wsmeta))
		};

		info!("Connection successfully created");

		Ok(backend)
	}

	pub async fn process_incoming_messages(&self) {
		// Not needed as done inline at the moment.
	}
}

#[cfg(feature = "ws-web")]
#[cfg(test)]
mod tests {
	use super::*;
	use wasm_bindgen_test::*;
	wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
	use crate::Backend;

	#[cfg(target_arch = "wasm32")]
	pub fn set_panic_hook() {
		// When the `console_error_panic_hook` feature is enabled, we can call the
		// `set_panic_hook` function at least once during initialization, and then
		// we will get better error messages if our code ever panics.
		//
		// For more details see
		// https://github.com/rustwasm/console_error_panic_hook#readme
		#[cfg(feature = "console_error_panic_hook")]
		console_error_panic_hook::set_once();
	}

	async fn polkadot_backend() -> super::Backend {
		crate::ws_web::Backend::new_ws2("wss://rpc.polkadot.io").await.unwrap()
	}

	#[test]
	#[wasm_bindgen_test]
	fn can_get_metadata() {
		async_std::task::block_on(can_get_metadata_test());
	}

	async fn can_get_metadata_test() {
		set_panic_hook();
		// wasm-pack test --headless --firefox --no-default-features --features ws-web

		let latest_metadata = polkadot_backend().await.query_metadata(None).await.unwrap();
		assert!(latest_metadata.len() > 0);
	}

	#[test]
	#[wasm_bindgen_test]
	fn can_get_metadata_as_of() {
		async_std::task::block_on(can_get_metadata_as_of_test());
	}

	async fn can_get_metadata_as_of_test() {
		let block_hash =
			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
				.unwrap();
		let as_of_metadata =
			polkadot_backend().await.query_metadata(Some(&block_hash)).await.unwrap();
		assert!(as_of_metadata.len() > 0);
	}

	#[test]
	#[wasm_bindgen_test]
	fn can_get_block_hash() {
		async_std::task::block_on(can_get_block_hash_test());
	}

	async fn can_get_block_hash_test() {
		env_logger::init();
		let polkadot = polkadot_backend().await;
		let hash = polkadot.query_block_hash(&vec![1]).await.unwrap();
		assert_eq!(
			"c0096358534ec8d21d01d34b836eed476a1c343f8724fa2153dc0725ad797a90",
			hex::encode(hash)
		);

		let hash = polkadot.query_block_hash(&vec![10504599]).await.unwrap();
		assert_eq!(
			"e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16",
			hex::encode(hash)
		);
	}

	#[test]
	#[wasm_bindgen_test]
	fn can_get_full_block() {
		async_std::task::block_on(can_get_full_block_test());
	}

	async fn can_get_full_block_test() {
		let hash = Some("c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a");
		let block_bytes = polkadot_backend().await.query_block(hash).await.unwrap();
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	#[test]
	#[wasm_bindgen_test]
	fn can_get_latest_full_block() {
		async_std::task::block_on(can_get_latest_full_block_test());
	}

	async fn can_get_latest_full_block_test() {
		let block_bytes = polkadot_backend().await.query_block(None).await.unwrap();
		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
	}

	#[test]
	#[wasm_bindgen_test]
	fn can_get_storage_as_of() {
		async_std::task::block_on(can_get_storage_as_of_test());
	}

	async fn can_get_storage_as_of_test() {
		//env_logger::init();
		let block_hash =
			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
				.unwrap();

		let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
		let key = hex::decode(events_key).unwrap();

		let as_of_events = polkadot_backend()
			.await
			.query_storage(&key[..], Some(&block_hash))
			.await
			.unwrap();
		assert!(as_of_events.len() > 0);
	}

	#[test]
	#[wasm_bindgen_test]
	fn can_get_storage_now() {
		async_std::task::block_on(can_get_storage_now_test());
	}

	async fn can_get_storage_now_test() {
		// env_logger::init();
		let key = "0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";
		let key = hex::decode(key).unwrap();
		let parachain = super::Backend::new_ws2("wss://calamari-rpc.dwellir.com").await.unwrap();

		let as_of_events = parachain.query_storage(&key[..], None).await.unwrap();
		assert_eq!(hex::decode("e8030000").unwrap(), as_of_events);
		// This is statemint's scale encoded parachain id (1000)
	}
}
