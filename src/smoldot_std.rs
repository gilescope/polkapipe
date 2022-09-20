use crate::prelude::*;
use async_trait::async_trait;
use core::{convert::TryInto, fmt};
use jsonrpc::{
	error::{result_to_response, standard_error, RpcError, StandardError},
	serde_json,
	serde_json::{to_string, value::to_raw_value},
};
use serde_json::value::RawValue;
// pub use surf::Url;

use core::iter;
use futures::{channel::mpsc, prelude::*};
use smoldot_light::ChainId;


use crate::rpc::{self, Rpc, RpcResult};

pub struct Backend(smoldot_light::Client::<
        smoldot_light::platform::async_std::AsyncStdTcpWebSocket,
    >, ChainId);

impl Backend {
	pub fn new<U>(chainspec: String) -> Self
	{
		//TODO put in thread local instance.
		let mut client = smoldot_light::Client::<
        smoldot_light::platform::async_std::AsyncStdTcpWebSocket,
    >::new(smoldot_light::ClientConfig {
        // The smoldot client will need to spawn tasks that run in the background. In order to do
        // so, we need to provide a "tasks spawner".
        tasks_spawner: Box::new(move |_name, task| {
            async_std::task::spawn(task);
        }),
        system_name: env!("CARGO_PKG_NAME").into(),
        system_version: env!("CARGO_PKG_VERSION").into(),
    });

	let (json_rpc_responses_tx, mut json_rpc_responses_rx) = mpsc::channel(32);

 // Ask the client to connect to a chain.
    let chain_id = client
        .add_chain(smoldot_light::AddChainConfig {
            // The most important field of the configuration is the chain specification. This is a
            // JSON document containing all the information necessary for the client to connect to said
            // chain.
            specification: &chainspec,

            // See above.
            // Note that it is possible to pass `None`, in which case the chain will not be able to
            // handle JSON-RPC requests. This can be used to save up some resources.
            json_rpc_responses: Some(json_rpc_responses_tx),

            // This field is necessary only if adding a parachain.
            potential_relay_chains: iter::empty(),

            // After a chain has been added, it is possible to extract a "database" (in the form of a
            // simple string). This database can later be passed back the next time the same chain is
            // added again.
            // A database with an invalid format is simply ignored by the client.
            // In this example, we don't use this feature, and as such we simply pass an empty string,
            // which is intentionally an invalid database content.
            database_content: "",

            // The client gives the possibility to insert an opaque "user data" alongside each chain.
            // This avoids having to create a separate `HashMap<ChainId, ...>` in parallel of the
            // client.
            // In this example, this feature isn't used. The chain simply has `()`.
            user_data: (),
        })
        .unwrap();


		Backend(client, chain_id //url.try_into().expect("Url")
	)
	}
}

#[async_trait]
impl Rpc for Backend {
	/// HTTP based JSONRpc request expecting an hex encoded result
	async fn rpc(&self, method: &str, params: Vec<Box<RawValue>>) -> RpcResult {

	self.0
        .json_rpc_request(
            r#"{"id":1,"jsonrpc":"2.0","method":"chain_subscribeNewHeads","params":[]}"#,
            self.1,
        )
        .unwrap();



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

		// log::debug!("RPC Response: {}...", &res[..res.len().min(20)]);
		// // assume the response is a hex encoded string starting with "0x"
		// let response = hex::decode(&res[2..])
		// 	.map_err(|_err| standard_error(StandardError::InternalError, None))?;
		Ok(response)
	}

	async fn rpc_single(
		&self,
		method: &str,
		params: Box<RawValue>,
	) -> Result<serde_json::value::Value, RpcError> {
		panic!()
	}
}
