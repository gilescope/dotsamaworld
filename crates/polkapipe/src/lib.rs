#![cfg_attr(not(feature = "std"), no_std)]
/*!
Polkapipe is a fork of Sube that has few deps with multi-backend support
that can be used to access substrate based chains. It leaves encoding / decoding
to higher level crates like desub.

## Usage

Creating a client is as simple as instantiating a backend and converting it to a `Sube` instance.

```
# use polkapipe::{Error, PolkaPipe};
# #[async_std::main] async fn main() -> Result<(), Error> {
# const CHAIN_URL: &str = "ws://localhost:24680";
// Create an instance of Sube from any of the available backends
//let client = ws::PolkaPipe::<_>::new(ws::Backend::new(CHAIN_URL).await?);

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
* **ws-web**
  Enables the websocket implementation that works in the browser.
* **smoldot-std**
  Uses light client to answer rpc requests
*/

#[macro_use]
extern crate alloc;
use core::fmt;
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
#[cfg(all(feature = "ws", not(target_arch = "wasm32")))]
pub mod ws;

#[cfg(all(feature = "smoldot-std", not(target_arch = "wasm32")))]
pub mod smoldot_std;

/// Tungstenite based backend
#[cfg(all(feature = "ws-web", target_arch = "wasm32"))]
pub mod ws_web;

mod rpc;

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

#[cfg(all(feature = "ws", not(target_arch = "wasm32")))]
impl From<async_tungstenite::tungstenite::Error> for Error {
	fn from(_err: async_tungstenite::tungstenite::Error) -> Self {
		Error::ChainUnavailable
	}
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

pub use rpc::PolkaPipe;
