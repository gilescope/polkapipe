#![cfg_attr(not(feature = "std"), no_std)]
/*!
Polkapipe is a fork of Sube that has few deps with multi-backend support
that can be used to access substrate based chains. It leaves encoding / decoding
to higher level crates like desub.

## Usage

Creating a client is as simple as instantiating a backend and converting it to a `Sube` instance.

```
# use polkapipe::{Sube, Error, Backend};
# #[async_std::main] async fn main() -> Result<(), Error> {
# const CHAIN_URL: &str = "ws://localhost:24680";
// Create an instance of Sube from any of the available backends
// let client: Sube<_> = ws::Backend::new_ws2(CHAIN_URL).await?.into();

# Ok(()) }
```

### Backend features

* **http** -
  Enables a surf based http backend.
* **http-web** -
  Enables surf with its web compatible backend that uses `fetch` under the hood(target `wasm32-unknown-unknown`)
* **ws** -
  Enables the websocket backend based on tungstenite
* **wss** -
  Same as `ws` and activates the TLS functionality of tungstenite

*/

#[macro_use]
extern crate alloc;
use async_trait::async_trait;
use core::{fmt, ops::Deref};
use prelude::*;
mod prelude {
	pub use alloc::{
		boxed::Box,
		string::{String, ToString},
		vec::Vec,
	};
}

pub type Result<T> = core::result::Result<T, Error>;

/// Surf based backend
#[cfg(any(feature = "http", feature = "http-web"))]
pub mod http;

/// Tungstenite based backend
#[cfg(feature = "ws")]
pub mod ws;

mod rpc;

/// Main interface for interacting with the Substrate based blockchain
#[derive(Debug)]
pub struct Sube<B> {
	backend: B,
}

impl<B: Backend> Sube<B> {
	pub fn new(backend: B) -> Self {
		Sube { backend }
	}

	// /// Get the chain metadata and cache it for future calls
	// pub async fn metadata(&self) -> Result<&Metadata> {
	//     match self.meta.get() {
	//         Some(meta) => Ok(meta),
	//         None => {
	//             let meta = self.backend.metadata().await?;
	//             // self.meta.set(meta).expect("unset");
	//             Ok(self.meta.get().unwrap())
	//         }
	//     }
	// }
}

impl<B: Backend> From<B> for Sube<B> {
	fn from(b: B) -> Self {
		Sube::new(b)
	}
}

impl<T: Backend> Deref for Sube<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.backend
	}
}

/// Generic definition of a blockchain backend
///
/// ```rust,ignore
/// #[async_trait]
/// pub trait Backend {
///     async fn query_bytes(&self, key: &StorageKey) -> Result<Vec<u8>>;
///
///     async fn submit<T>(&self, ext: T) -> Result<()>
///     where
///         T: AsRef<[u8]> + Send;
///
///     async fn metadata(&self) -> Result<Metadata>;
/// }
/// ```
#[async_trait]
pub trait Backend {
	/// Get raw storage items form the blockchain
	async fn query_storage(&self, key: &[u8], as_of: Option<&[u8]>) -> crate::Result<Vec<u8>>;

	/// Get block hash for block number
	async fn query_block_hash(&self, block_number: &[u32]) -> crate::Result<Vec<u8>>;

	/// Get block for block hash
	async fn query_block(&self, block_hash: &str) -> crate::Result<serde_json::value::Value>;

	/// Send a signed extrinsic to the blockchain
	// async fn submit<T>(&self, ext: T) -> Result<()>
	// where
	//     T: AsRef<[u8]> + Send;

	async fn query_metadata(&self, as_of: Option<&[u8]>) -> crate::Result<Vec<u8>>;
}

/// A Dummy backend for offline querying of metadata
pub struct Offline(pub Vec<u8>);

#[async_trait]
impl Backend for Offline {
	async fn query_storage(&self, _key: &[u8], _as_of: Option<&[u8]>) -> Result<Vec<u8>> {
		Err(Error::ChainUnavailable)
	}

	// /// Send a signed extrinsic to the blockchain
	// async fn submit<T>(&self, _ext: T) -> Result<()>
	// where
	//     T: AsRef<[u8]> + Send,
	// {
	//     Err(Error::ChainUnavailable)
	// }

	async fn query_block_hash(&self, _block_number: &[u32]) -> crate::Result<Vec<u8>> {
		Err(Error::ChainUnavailable)
	}

	async fn query_block(&self, _block_hash: &str) -> crate::Result<serde_json::value::Value> {
		Err(Error::ChainUnavailable)
	}

	async fn query_metadata(&self, _as_of: Option<&[u8]>) -> Result<Vec<u8>> {
		Ok(self.0.clone())
	}
}

#[derive(Clone, Debug)]
pub enum Error {
	ChainUnavailable,
	BadInput,
	BadKey,
	Node(String),
	ParseStorageItem,
	StorageKeyNotFound,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Node(e) => write!(f, "{:}", e),
			_ => write!(f, "{:?}", self),
		}
	}
}

#[cfg(feature = "ws")]
impl From<async_tungstenite::tungstenite::Error> for Error {
	fn from(_err: async_tungstenite::tungstenite::Error) -> Self {
		Error::ChainUnavailable
	}
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
