//! Transport layer — codec, stdio, and feature-gated network transports.

mod codec;
pub use codec::{parse_request, process_message, serialize_response};

pub mod stdio;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "ws")]
pub mod ws;

#[cfg(feature = "unix")]
pub mod unix;
