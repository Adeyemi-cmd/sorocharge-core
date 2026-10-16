#![no_main]

//! Fuzz target for the library's untrusted-XDR boundary:
//! `SorobanAuthorizationEntry` decoding feeding `SignedEntry::from_xdr` and
//! [`sorocharge_signer::verify_entry`].
//!
//! `verify_entry` is where bytes a facilitator received over the network meet
//! this library. The fuzz contract is deliberately narrow: whatever the input
//! bytes are, decoding must either fail cleanly or yield an entry that
//! `verify_entry` accepts or rejects with one of its typed errors — it must
//! never panic (which, in an embedding server, would be a crash), and it must
//! never hang.

use libfuzzer_sys::fuzz_target;
use sorocharge_signer::{verify_entry, Address, ChargeParams, SignedEntry};
use stellar_xdr::{Limits, ReadXdr, SorobanAuthorizationEntry};

/// The network the golden-vector fixtures were signed for.
const NETWORK_PASSPHRASE: &str = "Test SDF Network ; September 2015";

/// The parameters from `tests/golden_vectors/fixtures/legacy_transfer_signed.json`,
/// chosen so the committed corpus (which is seeded from the real fixtures)
/// drives `verify_entry` all the way through signature verification instead of
/// failing on the first check. Arbitrary input still takes every other path.
fn expected_params() -> Option<ChargeParams> {
    Some(ChargeParams {
        asset_contract: "CAZTGMZTGMZTGMZTGMZTGMZTGMZTGMZTGMZTGMZTGMZTGMZTGMZTGGJH"
            .parse::<Address>()
            .ok()?,
        amount: 1_000_000_000,
        payer: "GDIEVMRSOQV3JKZ2CNUL2RQV4TTNAISKW4NAC25PQUQKGMWJO6DTOAE7"
            .parse::<Address>()
            .ok()?,
        recipient: "GCQJVJPUPJTVTABP7FK7RXBNFIKKLSM5EO7JP6DECJ77SOBUKWSPB64N"
            .parse::<Address>()
            .ok()?,
        // The fixture's own expiry; `current_ledger` below sits one ledger
        // under it, so the seeded entry is neither expired nor over-long-lived.
        valid_until_ledger: 123_456,
    })
}

fuzz_target!(|data: &[u8]| {
    let Ok(entry) = SorobanAuthorizationEntry::from_xdr(data, Limits::none()) else {
        // Malformed input is expected and must stay an `Err`, not a panic.
        return;
    };

    // Wrapping an already-decoded entry is infallible by design (it performs
    // no validation), but exercise it so the whole documented path is fuzzed.
    let signed = SignedEntry::from_xdr(entry);

    let Some(expected) = expected_params() else {
        return;
    };

    // `black_box` keeps the result observable so the call can't be optimized
    // away; a returned `Result` (rather than a panic) is the property.
    let _ = std::hint::black_box(verify_entry(
        &signed,
        &expected,
        expected.valid_until_ledger - 1,
        NETWORK_PASSPHRASE,
    ));
});
