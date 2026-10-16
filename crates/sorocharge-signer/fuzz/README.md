# Fuzzing `sorocharge-signer`

This directory holds the [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)
harness for `sorocharge-signer`'s untrusted-XDR boundary: arbitrary bytes are
decoded as a `SorobanAuthorizationEntry` and fed through
`SignedEntry::from_xdr` and `verify_entry`. The contract under test is only
that this never panics — a malformed entry must come back as a typed
`SorochargeError`, never as a crash in the process that embeds the library.

## Toolchain

`cargo-fuzz` needs **nightly** Rust (for the sanitizer/libFuzzer runtime).
That requirement is scoped to this directory by `fuzz/rust-toolchain.toml`; it
does **not** change the library's own stable toolchain pin (`rust-version` in
`../Cargo.toml`), and `fuzz/` is detached from the root workspace so
`cargo build/test --workspace` never tries to compile it.

## Run it locally

```sh
rustup toolchain install nightly        # once
cargo install cargo-fuzz                # once

# From inside this `fuzz/` directory, so `fuzz/rust-toolchain.toml`
# (nightly) is honored even when your default toolchain is stable:
cd crates/sorocharge-signer/fuzz
cargo fuzz run verify_entry             # runs until it finds a crash (Ctrl-C to stop)
cargo fuzz run verify_entry -- -max_total_time=60   # bounded 60-second run
```

A crash is written to `fuzz/artifacts/verify_entry/` (i.e.
`artifacts/verify_entry/` from inside this directory); reproduce it with:

```sh
cargo fuzz run verify_entry artifacts/verify_entry/<crash-file>
```

## Corpus

`fuzz/corpus/verify_entry/` is seeded with the real golden-vector fixtures
from `tests/golden_vectors/fixtures/` — each file is the raw XDR bytes of the
fixture's `unsigned_entry_xdr_base64` / `signed_entry_xdr_base64`, so the
fuzzer starts from entries the reference SDK actually produced (including the
one valid, signed entry that reaches full signature verification).

To extend the corpus:

1. Add or edit a fixture under `tests/golden_vectors/fixtures/` (see
   `tests/golden_vectors/generate.mjs`).
2. Re-decode its base64 field into `fuzz/corpus/verify_entry/`:

   ```sh
   python3 - <<'PY'
   import base64, json, pathlib
   f = json.loads(pathlib.Path("tests/golden_vectors/fixtures/<name>.json").read_text())
   raw = base64.b64decode(f["signed_entry_xdr_base64"])  # or unsigned_entry_xdr_base64
   pathlib.Path("crates/sorocharge-signer/fuzz/corpus/verify_entry/<name>").write_bytes(raw)
   PY
   ```

3. Commit the new corpus file. Any raw bytes that decode are also welcome as
   corpus entries — the committed fixtures are just a high-quality starting
   point.
