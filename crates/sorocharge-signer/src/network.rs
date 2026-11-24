use crate::SorochargeError;

const PUBNET_PASSPHRASE: &str = "Public Global Stellar Network ; September 2015";
const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";

/// Maps a Stellar network passphrase to its CAIP-2 identifier.
///
/// Exists so the x402 and MPP adapters share one mapping instead of each
/// keeping a copy that could drift. Any passphrase other than mainnet's or
/// testnet's fails closed rather than guessing a network.
///
/// # Errors
///
/// Returns [`SorochargeError::UnsupportedNetwork`] for any other passphrase.
///
/// # Examples
///
/// ```
/// use sorocharge_signer::{caip2_for_passphrase, SorochargeError};
///
/// assert_eq!(
///     caip2_for_passphrase("Test SDF Network ; September 2015"),
///     Ok("stellar:testnet"),
/// );
/// assert_eq!(
///     caip2_for_passphrase("Public Global Stellar Network ; September 2015"),
///     Ok("stellar:pubnet"),
/// );
///
/// // Anything else fails closed — including a CAIP-2 id passed where a
/// // passphrase was expected — and the error carries the input verbatim.
/// assert_eq!(
///     caip2_for_passphrase("stellar:testnet"),
///     Err(SorochargeError::UnsupportedNetwork {
///         network: "stellar:testnet".to_string(),
///     }),
/// );
/// ```
pub fn caip2_for_passphrase(network_passphrase: &str) -> Result<&'static str, SorochargeError> {
    match network_passphrase {
        PUBNET_PASSPHRASE => Ok("stellar:pubnet"),
        TESTNET_PASSPHRASE => Ok("stellar:testnet"),
        other => Err(SorochargeError::UnsupportedNetwork {
            network: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_passphrases_to_caip2() {
        assert_eq!(
            caip2_for_passphrase(PUBNET_PASSPHRASE),
            Ok("stellar:pubnet")
        );
        assert_eq!(
            caip2_for_passphrase(TESTNET_PASSPHRASE),
            Ok("stellar:testnet")
        );
    }

    /// `UnsupportedNetwork` is the only failure here, and it carries the
    /// rejected passphrase verbatim — including a CAIP-2 id mistakenly
    /// passed where a passphrase was expected.
    #[test]
    fn unknown_passphrase_is_unsupported_network_carrying_it_verbatim() {
        for bad in [
            "stellar:testnet",
            "",
            "Test SDF Future Network ; October 2022",
        ] {
            assert_eq!(
                caip2_for_passphrase(bad),
                Err(SorochargeError::UnsupportedNetwork {
                    network: bad.to_string(),
                })
            );
        }
    }
}
