use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};
use async_std::sync::Mutex;
use async_trait::async_trait;
use futures::{
	stream::{SplitSink, SplitStream, StreamExt},
	SinkExt,
};
use futures_channel::oneshot;
use jsonrpc::{
	error::{result_to_response, standard_error, RpcError, StandardError},
	serde_json,
};
use log::{info};
use serde_json::value::RawValue;
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
	wsmeta: Arc<Mutex<WsMeta>>,
	messages: Arc<Mutex<BTreeMap<Id, oneshot::Sender<rpc::Response>>>>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Rpc for Backend
// where
// 	Tx: Sink<Message, Error = Error> + Unpin + Send,
{
	async fn rpc(&self, method: &str, params: Vec<Box<RawValue>>) -> RpcResult {
		let id = self.next_id().await;
		log::info!("RPC `{}` (ID={})", method, id);

		// Store a sender that will notify our receiver when a matching message arrives
		let (sender, recv) = oneshot::channel::<rpc::Response>();
		let messages = self.messages.clone();
		messages.lock().await.insert(id, sender);

		// send rpc request
		let msg = serde_json::to_string(&rpc::Request {
			id: id.into(),
			jsonrpc: Some("2.0"),
			method,
			params: &params,
		})
		.expect("Request is serializable");
		log::info!("RPC Request {} ...", &msg[..50]);
		let mut lock = self.stream.lock().await;
		log::info!("RPC got lock now sending {} ...", &msg[..50]);		
		let _ = lock.send(Message::Text(msg)).await;
		log::info!("RPC now waiting for response ...");
		// wait for the matching response to arrive
		let res = recv
			.await
			.map_err(|_| standard_error(StandardError::InternalError, None))?
			.result::<String>()?;
		log::info!("RPC Response: {}...", &res[..res.len().min(20)]);
		let res = hex::decode(&res[2..])
			.map_err(|_| standard_error(StandardError::InternalError, None))?;
		Ok(res)
	}

	async fn rpc_single(
		&self,
		method: &str,
		params: Box<RawValue>,
	) -> Result<serde_json::value::Value, RpcError> {
		let id = self.next_id().await;
		log::info!("RPC `{}` (ID={})", method, id);

		// Store a sender that will notify our receiver when a matching message arrives
		let (sender, recv) = oneshot::channel::<rpc::Response>();
		let messages = self.messages.clone();
		messages.lock().await.insert(id, sender);

		// send rpc request
		// example working request:
		// {"method":"chain_getBlockHash","params":["1"],"id":1,"jsonrpc":"2.0"}
		let msg = format!(
			"{{\"id\":{}, \"jsonrpc\": \"2.0\", \"method\":\"{}\", \"params\":[{}]}}",
			id, method, params
		);

		log::info!("RPC Request {} ...", &msg);
		let _ = self.stream.lock().await.send(Message::Text(msg)).await;

		// wait for the matching response to arrive
		let res = recv
			.await
			.map_err(|_| standard_error(StandardError::InternalError, None))?
			.result::<serde_json::value::Value>()
			.map_err(|_| standard_error(StandardError::InternalError, None))?;
		Ok(res)
	}
}

impl Backend {
	async fn next_id(&self) -> Id {
		self.messages.lock().await.keys().last().unwrap_or(&0) + 1
	}
}

//type WS2 = SplitSink<WsStream, ws_stream_wasm::WsMessage>;

impl Backend {
	pub async fn new_ws2(url: &str) -> core::result::Result<Self, Error> {
		log::info!("WS connecting to {}", url);
		let (wsmeta, stream) =
			WsMeta::connect(url, None).await.expect_throw("assume the connection succeeds");

		// let (tx, rx) = stream.split();

		let backend = Backend {
			stream: Arc::new(Mutex::new(stream)),
			wsmeta: Arc::new(Mutex::new(wsmeta)),
			// rx: Arc::new(Mutex::new(rx)),
			messages: Arc::new(Mutex::new(BTreeMap::new())),
		};

		info!("Connection successfully created");

		Ok(backend)
	}

	pub async fn process_incoming_messages(&self)
	// where
	// Rx: Stream<Item = core::result::Result<Message, WsError>> + Unpin + Send +
	// 'static, Rx: Stream<Item = Message> + Unpin + Send + 'static,
	{
		let rx = &mut *self.stream.lock().await;
		let messages = &self.messages;

		log::info!("checking incoming messages");
		
		{
			let mut messages = messages.lock().await;
			log::info!("checking incoming messages - waiting on {}", (*messages).len());
		}
		let res = rx.next().await;
		if let Some(msg) = res {
			log::info!("Got WS message {:?}", msg);
			if let Message::Text(msg) = msg {
				log::info!("{:#?}", msg);
				let res: rpc::Response = serde_json::from_str(&msg).unwrap_or_else(|_| {
					result_to_response(
						Err(standard_error(StandardError::ParseError, None)),
						().into(),
					)
				});
				if res.id.is_u64() {
					let id = res.id.as_u64().unwrap() as Id;
					log::info!("Answering request {}", id);
					let mut messages = messages.lock().await;
					if let Some(channel) = messages.remove(&id) {
						channel.send(res).expect("receiver waiting");
						log::debug!("Answered request id: {}", id);
					}
				}
			}
		} else {
			log::info!("Got WS error: {:?}", res);
		}
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
	use crate::ws_web::rpc;
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
			polkadot_backend().await.query_metadata(Some(&block_hash)).await
				.unwrap();
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

		let hash =  polkadot.query_block_hash(&vec![10504599]).await.unwrap();
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
		let hash = "c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a";
		let block_bytes = polkadot_backend().await.query_block(hash).await.unwrap();
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

		let as_of_events = 
			polkadot_backend().await.query_storage(&key[..], Some(&block_hash))
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
		let parachain = super::Backend::new_ws2(
			"wss://calamari-rpc.dwellir.com",
		).await
		.unwrap();

		let as_of_events =
			parachain.query_storage(&key[..], None).await.unwrap();
		assert_eq!(hex::decode("e8030000").unwrap(), as_of_events);
		// This is statemint's scale encoded parachain id (1000)
	}
}
