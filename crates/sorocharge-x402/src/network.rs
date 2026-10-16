//! Mapping between Stellar network passphrases and the CAIP-2 identifiers
//! (`stellar:pubnet` / `stellar:testnet`) the x402 wire format uses. The
//! `exact` scheme on Stellar supports exactly these two networks.

use crate::error::X402Error;

pub const PUBNET_PASSPHRASE: &str = "Public Global Stellar Network ; September 2015";
pub const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";

/// The CAIP-2 identifier for a network passphrase.
///
/// # Errors
///
/// Returns [`X402Error::UnsupportedNetwork`] for any passphrase other than
/// mainnet's or testnet's — the `exact` scheme on Stellar defines no other
/// network.
pub fn caip2_for_passphrase(network_passphrase: &str) -> Result<&'static str, X402Error> {
    match network_passphrase {
        PUBNET_PASSPHRASE => Ok("stellar:pubnet"),
        TESTNET_PASSPHRASE => Ok("stellar:testnet"),
        other => Err(X402Error::UnsupportedNetwork {
            network: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A passphrase with no CAIP-2 mapping must surface as exactly
    /// `UnsupportedNetwork`, carrying the offending identifier back so a
    /// caller can tell it apart from every other `X402Error` variant.
    #[test]
    fn rejects_a_passphrase_with_no_caip2_mapping() {
        let unknown = "Future SDF Network ; September 2035";

        match caip2_for_passphrase(unknown) {
            Err(X402Error::UnsupportedNetwork { network }) => {
                assert_eq!(network, unknown);
            }
            other => panic!("expected UnsupportedNetwork, got {other:?}"),
        }
    }
}
