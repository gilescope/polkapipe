// // use crate::{prelude::*, ws::serde_json::value::RawValue};
// use alloc::{collections::BTreeMap, sync::Arc};
// // use async_mutex::Mutex;
// // use async_std::task;
use async_trait::async_trait;
// // use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures_channel::oneshot;
// use futures_util::{
// 	sink::{Sink, SinkExt},
// 	stream::SplitSink,
// 	Stream, StreamExt,
// };
use alloc::boxed::Box;
// use alloc::string::ToString;
use alloc::vec::Vec;use alloc::collections::BTreeMap;
use async_std::sync::Mutex; 
use async_std::prelude::Stream;
use alloc::sync::Arc;
use jsonrpc::{
	error::{result_to_response, standard_error, RpcError, StandardError},
	serde_json,
}; use futures::stream::SplitSink;
 use futures::stream::SplitStream;
use serde_json::value::RawValue;

use crate::{
	rpc::{self, Rpc, RpcResult},
	Error,
};
// use ws_stream_wasm::WsErr as WsError;
type Message=WsMessage;
type Id = u8;
use
{
   ws_stream_wasm       :: *                        ,
//    pharos               :: *                        ,
   wasm_bindgen         :: UnwrapThrowExt           ,
//    wasm_bindgen_futures :: futures_0_3::spawn_local ,
   futures              :: stream::StreamExt        ,
};

pub struct Backend {
	tx: Mutex<SplitSink<WsStream, ws_stream_wasm::WsMessage>>,
	rx: Mutex<SplitStream<WsStream>>,
	messages: Arc<Mutex<BTreeMap<Id, oneshot::Sender<rpc::Response>>>>,
}


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
use futures::SinkExt;
 use futures::Sink;use alloc::string::String;
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

		let msg = serde_json::to_string(&rpc::Request {
				id: id.into(),
				jsonrpc: Some("2.0"),
				method,
				params: &params,
			})
			.expect("Request is serializable");
		log::debug!("RPC Request {} ...", &msg[..50]);
		let _ = self.tx.lock().await.send(Message::Text(msg)).await;

		// wait for the matching response to arrive
		let res = recv
			.await
			.map_err(|_| standard_error(StandardError::InternalError, None))?
			.result::<String>()?;
		log::debug!("RPC Response: {}...", &res[..res.len().min(20)]);
		let res = hex::decode(&res[2..])
			.map_err(|_| standard_error(StandardError::InternalError, None))?;
		Ok(res)

	// let wsio = self.tx.lock().unwrap();

	//    let (mut ws, mut wsio) = WsMeta::connect( url, None ).await

    //   .expect_throw( "assume the connection succeeds" );
		// let x = wsio.send(WsMessage::Text("hello".to_string())).await;
// 		panic!("test panic {:?}", x);
// //    let mut evts = ws.observe( ObserveConfig::default() ).expect_throw( "observe" );

//    ws.close().await;

   // Note that since WsMeta::connect resolves to an opened connection, we don't see
   // any Open events here.
   //
//    assert!( evts.next().await.unwrap_throw().is_closing() );
//    assert!( evts.next().await.unwrap_throw().is_closed () );
		
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

		log::debug!("RPC Request {} ...", &msg);
		let _ = self.tx.lock().await.send(Message::Text(msg)).await;

		// wait for the matching response to arrive
		let res = recv
			.await
			.map_err(|_| standard_error(StandardError::InternalError, None))?
			.result::<serde_json::value::Value>()
			.map_err(|_| standard_error(StandardError::InternalError, None))?;
		Ok(res)
	}
}

// impl<Tx> Backend<Tx> {
// 	async fn next_id(&self) -> Id {
// 		self.messages.lock().await.keys().last().unwrap_or(&0) + 1
// 	}
// }


type WS2 = SplitSink<WsStream, ws_stream_wasm::WsMessage>;

impl Backend {
	pub async fn new_ws2(url: &str) -> core::result::Result<Self, Error> {
		log::trace!("WS connecting to {}", url);
		// let (stream, _) = async_tungstenite::async_std::connect_async(url).await?;
	
// console_log::init_with_level(Level::Trace).expect("Failed to enable logging");
	// use soketto::handshake::{Client, ServerResponse};
	use log::{error, info, Level};
	// First, we need to establish a TCP connection.
	// let socket = tokio::net::TcpStream::connect(url).await.unwrap();
	let (_ws, wsio) = WsMeta::connect( url, None ).await

      .expect_throw( "assume the connection succeeds" );

	// wsio.send(Message::Text(r##"{"method":"chain_getBlockHash","params":["1"],"id":1,"jsonrpc":"2.0"}"##.to_string())).await.unwrap();
	let (tx, rx) = wsio.split();

   
		let backend = Backend {
			tx: Mutex::new(tx),
			rx: Mutex::new(rx),
			messages: Arc::new(Mutex::new(BTreeMap::new())),
		};

		// backend.process_incoming_message(rx).await;
 info!("Connection successfully created");
	// todo!();
		Ok(backend)
	}

	pub async fn process_incoming_messages(&self)
	where
		// Rx: Stream<Item = core::result::Result<Message, WsError>> + Unpin + Send + 'static,
			// Rx: Stream<Item = Message> + Unpin + Send + 'static,
	{
		let mut rx = &mut *self.rx.lock().await;
		let messages = self.messages.clone();

		// task::spawn(async move {
		while let Some(msg) = rx.next().await {
				// match msg {
					// Ok(msg) => {
						log::trace!("Got WS message {:?}", msg);
						if let Message::Text(msg) = msg {
							log::info!("{:#?}", msg);
							let res: rpc::Response =
								serde_json::from_str(&msg).unwrap_or_else(|_| {
									result_to_response(
										Err(standard_error(StandardError::ParseError, None)),
										().into(),
									)
								});
							if res.id.is_u64() {
								let id = res.id.as_u64().unwrap() as Id;
								log::trace!("Answering request {}", id);
								let mut messages = messages.lock().await;
								if let Some(channel) = messages.remove(&id) {
									channel.send(res).expect("receiver waiting");
									log::debug!("Answered request id: {}", id);
								}
							}
						}
		// 			},
					// Err(err) => {
					// 	log::warn!("WS Error: {}", err);
					// },
				// }
		}
		// 	log::warn!("WS connection closed");
		// });
	}
}

#[cfg(feature = "ws-web")]
#[cfg(test)]
mod tests {
	use super::*;
	use wasm_bindgen_test::*;
	wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
// 	use crate::{
// 		ws,
// 		ws::{Message, SplitSink, WS2},
// 		Backend, Error,
// 	};
// 	use async_tungstenite::WebSocketStream;
// 	use futures_util::sink::SinkErrInto;

	async fn polkadot_backend() -> super::Backend<WS2> {
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
		// async_std::task::block_on(rpc("wss://ws.postman-echo.com/raw", vec![]));
		// wasm-pack test --headless --firefox --no-default-features --features ws-web

		// async_std::task::block_on(rpc("wss://rpc.polkadot.io", vec![]));
		// async_std::task::block_on(polkadot_backend());
		// let latest_metadata = polkadot_backend().query_metadata(None).await.unwrap();
		// assert!(latest_metadata.len() > 0);
	}

// 	#[test]
// 	fn can_get_metadata() {
// 		let latest_metadata =
// 			async_std::task::block_on(polkadot_backend().query_metadata(None)).unwrap();
// 		assert!(latest_metadata.len() > 0);
// 	}

// 	#[test]
// 	fn can_get_metadata_as_of() {
// 		let block_hash =
// 			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
// 				.unwrap();
// 		let as_of_metadata =
// 			async_std::task::block_on(polkadot_backend().query_metadata(Some(&block_hash)))
// 				.unwrap();
// 		assert!(as_of_metadata.len() > 0);
// 	}

// 	#[test]
// 	fn can_get_block_hash() {
// 		env_logger::init();
// 		let polkadot = polkadot_backend();
// 		let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![1])).unwrap();
// 		assert_eq!(
// 			"c0096358534ec8d21d01d34b836eed476a1c343f8724fa2153dc0725ad797a90",
// 			hex::encode(hash)
// 		);

// 		let hash = async_std::task::block_on(polkadot.query_block_hash(&vec![10504599])).unwrap();
// 		assert_eq!(
// 			"e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16",
// 			hex::encode(hash)
// 		);
// 	}

// 	#[test]
// 	fn can_get_full_block() {
// 		let hash = "c191b96685aad1250b47d6bc2e95392e3a200eaa6dca8bccfaa51cfd6d558a6a";
// 		let block_bytes = async_std::task::block_on(polkadot_backend().query_block(hash)).unwrap();
// 		assert!(matches!(block_bytes, serde_json::value::Value::Object(_)));
// 	}

// 	#[test]
// 	fn can_get_storage_as_of() {
// 		env_logger::init();
// 		let block_hash =
// 			hex::decode("e33568bff8e6f30fee6f217a93523a6b29c31c8fe94c076d818b97b97cfd3a16")
// 				.unwrap();

// 		let events_key = "26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7";
// 		let key = hex::decode(events_key).unwrap();

// 		let as_of_events = async_std::task::block_on(
// 			polkadot_backend().query_storage(&key[..], Some(&block_hash)),
// 		)
// 		.unwrap();
// 		assert!(as_of_events.len() > 0);
// 	}

// 	#[test]
// 	fn can_get_storage_now() {
// 		env_logger::init();
// 		let key = "0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f";
// 		let key = hex::decode(key).unwrap();
// 		let parachain = async_std::task::block_on(crate::ws::Backend::new_ws2(
// 			"wss://calamari-rpc.dwellir.com",
// 		))
// 		.unwrap();

// 		let as_of_events =
// 			async_std::task::block_on(parachain.query_storage(&key[..], None)).unwrap();
// 		assert_eq!(hex::decode("e8030000").unwrap(), as_of_events);
// 		// This is statemint's scale encoded parachain id (1000)
// 	}
}

impl Backend {
	async fn next_id(&self) -> Id {
		self.messages.lock().await.keys().last().unwrap_or(&0) + 1
	}
}
