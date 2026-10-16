# Fuzzing `sorocharge-x402`

This directory holds the [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)
harness for `sorocharge-x402`'s untrusted-input boundary: arbitrary bytes are
decoded as a `TransactionEnvelope` and fed through
`parse_payment_transaction`. The contract under test is only that this never
panics — malformed input must come back as a typed `X402Error`.

See `crates/sorocharge-signer/fuzz/README.md` for the shared cargo-fuzz setup,
toolchain notes, and general corpus guidance. The specifics for this target:

## Run it locally

```sh
rustup toolchain install nightly
cargo install cargo-fuzz

cd crates/sorocharge-x402/fuzz
cargo fuzz run parse_payment_transaction
cargo fuzz run parse_payment_transaction -- -max_total_time=60
```

Run from inside `fuzz/` so its `rust-toolchain.toml` (nightly) is honored even
when your default toolchain is stable.

## Corpus

`fuzz/corpus/parse_payment_transaction/signed_legacy_envelope` is a real
signed envelope produced by this crate's own test suite (the payer flow in
`src/tx.rs`), committed as raw XDR bytes. Add more by building a signed
envelope in a test and writing `envelope.to_xdr(Limits::none())` to a new file
under `fuzz/corpus/parse_payment_transaction/`.

The target reaches the parser through the `fuzzing`-gated
`sorocharge_x402::fuzzing::parse_payment_transaction` re-export. The parser
lives in the crate-private `tx` module, so that re-export is the only way
out-of-crate code (such as this harness) can reach it.
