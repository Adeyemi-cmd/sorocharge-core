#![no_main]

//! Fuzz target for `sorocharge-mpp`'s untrusted-input boundary:
//! `TransactionEnvelope` decoding feeding
//! [`sorocharge_mpp::fuzzing::parse_payment_transaction`].
//!
//! MPP decodes a client-supplied `TransactionEnvelope` in both pull-mode
//! settlement (the `Payload::Transaction` arm) and push mode's
//! `getTransaction` response parsing. This crate's parser is a separate
//! implementation from `sorocharge-x402`'s — the two crates do not share it —
//! with its own invariants (for example, MPP permits only the legacy
//! `sorobanCredentialsAddress` credential, where x402 also allows
//! `AddressV2`), so it gets its own target rather than reusing x402's.
//!
//! The fuzz contract is narrow: whatever the bytes are, decoding and parsing
//! return a typed `Result` — never a panic.

use libfuzzer_sys::fuzz_target;
use sorocharge_mpp::fuzzing::parse_payment_transaction;
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
