#![no_main]

//! Fuzz target for `sorocharge-x402`'s untrusted-input boundary:
//! `TransactionEnvelope` decoding feeding
//! [`sorocharge_x402::fuzzing::parse_payment_transaction`].
//!
//! A facilitator receives the client's payment as a base64
//! `TransactionEnvelope` in the `PAYMENT-SIGNATURE` header (or `/verify` /
//! `/settle` body). `parse_payment_transaction` is the first code to touch
//! that decoded XDR, so the fuzz contract is: whatever the bytes are,
//! decoding and parsing return a typed `Result` — never a panic.
//!
//! This is a genuinely separate surface from `sorocharge-signer`'s
//! `verify_entry` target: a different XDR root type
//! (`TransactionEnvelope`, not `SorobanAuthorizationEntry`) with different
//! structural invariants.

use libfuzzer_sys::fuzz_target;
use sorocharge_x402::fuzzing::parse_payment_transaction;
use stellar_xdr::{Limits, ReadXdr, TransactionEnvelope};

fuzz_target!(|data: &[u8]| {
    let Ok(envelope) = TransactionEnvelope::from_xdr(data, Limits::none()) else {
        // Malformed input is expected and must stay an `Err`, not a panic.
        return;
    };

    // `black_box` keeps the result observable so the call can't be optimized
    // away; a returned `Result` (rather than a panic) is the property.
    let _ = std::hint::black_box(parse_payment_transaction(&envelope));
});
