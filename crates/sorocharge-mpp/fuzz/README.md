# Fuzzing `sorocharge-mpp`

This directory holds the [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)
harness for `sorocharge-mpp`'s untrusted-input boundary: arbitrary bytes are
decoded as a `TransactionEnvelope` and fed through
`parse_payment_transaction`. The contract under test is only that this never
panics — malformed input must come back as a typed `MppError`.

This parser is a separate implementation from `sorocharge-x402`'s (the two
crates do not share it), with its own invariants, so it has its own target
rather than reusing x402's.

See `crates/sorocharge-signer/fuzz/README.md` for the shared cargo-fuzz setup,
toolchain notes, and general corpus guidance. The specifics for this target:

## Run it locally

```sh
rustup toolchain install nightly
cargo install cargo-fuzz

cd crates/sorocharge-mpp/fuzz
cargo fuzz run parse_payment_transaction
cargo fuzz run parse_payment_transaction -- -max_total_time=60
```

Run from inside `fuzz/` so its `rust-toolchain.toml` (nightly) is honored even
when your default toolchain is stable.

## Corpus

`fuzz/corpus/parse_payment_transaction/` is seeded with two real envelopes
produced by this crate's own test suite:

- `signed_sponsored_envelope` — the `feePayer: true` pull-mode shape
  (`build_sponsored_envelope`).
- `signed_unsponsored_envelope` — the `feePayer: false` shape
  (`build_unsponsored_transaction`).

Both are committed as raw XDR bytes. Add more by building an envelope in a
test and writing `envelope.to_xdr(Limits::none())` under
`fuzz/corpus/parse_payment_transaction/`.

The target reaches the parser through the `fuzzing`-gated
`sorocharge_mpp::fuzzing::parse_payment_transaction` re-export. The parser
lives in the crate-private `tx` module, so that re-export is the only way
out-of-crate code (such as this harness) can reach it.
