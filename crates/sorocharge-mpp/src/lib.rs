//! `sorocharge-mpp`: the client and server sides of MPP's
//! `draft-stellar-charge-00` `"stellar"`/`"charge"` payment method, built
//! on `sorocharge-signer`.
//!
//! MPP session/channel mode — a different signing primitive entirely (a raw
//! ed25519 signature over a `one-way-channel` contract's
//! `prepare_commitment` output, no `SorobanAuthorizationEntry` involved) —
//! is out of scope for this crate.

#![deny(unsafe_code)]

mod client;
mod error;
mod header;
mod network;
mod server;
mod tx;
mod types;

#[cfg(test)]
mod test_util;

pub use client::MppClient;
pub use error::MppError;
pub use server::{MppServer, MppServerConfig};
pub use types::{Challenge, ChargeRequest, Credential, MethodDetails, Payload, Receipt};

/// Internal wire-format entry points exposed on a public path solely so this
/// crate's out-of-workspace `fuzz/` cargo-fuzz target can reach them.
///
/// Gated behind the `fuzzing` feature, which no normal dependent enables, so
/// the crate's default public surface is unaffected.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing {
    pub use crate::tx::{parse_payment_transaction, TransferArgs};
}
