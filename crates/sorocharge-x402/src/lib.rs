#![deny(unsafe_code)]

mod client;
mod error;
mod facilitator;
mod network;
mod tx;
mod types;

#[cfg(test)]
mod test_util;

pub use client::X402Client;
pub use error::X402Error;
pub use facilitator::{Facilitator, FacilitatorConfig};
pub use types::{
    PaymentPayload, PaymentRequired, PaymentRequirements, ResourceInfo, SettleRequest,
    SettlementResponse, StellarExtra, StellarPayload, SupportedKind, SupportedResponse,
    VerifyRequest, VerifyResponse, MAX_AMOUNT_LEN, MAX_DESCRIPTION_LEN, MAX_ERROR_MESSAGE_LEN,
    MAX_MIME_TYPE_LEN, MAX_NETWORK_LEN, MAX_SCHEME_LEN, MAX_STRKEY_LEN, MAX_URL_LEN,
};

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
