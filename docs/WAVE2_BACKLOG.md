<!-- GENERATED FILE — do not hand-edit.
     Source of truth is docs/WAVE2_BACKLOG.json.
     Regenerate with: python3 scripts/render_wave_backlog_md.py -->

# Wave 2 backlog — sorocharge-core

**150 issues, 25000 points** — 80 high (200 pts each), 40 medium (150 pts each), 30 trivial (100 pts each).

By area: `ci-release` (35), `mpp` (44), `signer` (32), `x402` (39)

Every item below cites a specific file and gap found by reading this repository during this wave's survey, not a generic roadmap item. Complexity labels were assigned to hit an exact point quota per explicit maintainer instruction — they do not all correspond to real relative effort size.

## High — 200 points (80)

### fix(signer): reject non-positive transfer amounts

`area:signer`

**Description.** ChargeParams.amount is i128 with no lower-bound check anywhere in the pipeline. build_charge_entry, verify_entry, and both protocol adapters' amount parsing all accept zero or negative amounts and build/verify a structurally valid but semantically nonsensical authorization entry.

**Why.** A resource server (or a malicious counterparty supplying payment requirements) can set amount=0 and every check in verify_entry still passes, granting access for free. A negative amount produces a SEP-41 transfer call most token contracts will reject on-chain, but only after a wasted submission. No test or doc comment currently treats amount<=0 as invalid.

**Acceptance criteria.**
- [ ] build_charge_entry returns a new, specific error (not a generic one) for amount <= 0
- [ ] verify_entry rejects amount <= 0 even if it happens to match expected.amount
- [ ] A test asserts the specific error for amount == 0 and for a negative amount
- [ ] Existing callers (x402 facilitator, mpp client/server) surface the new error distinctly, not folded into InvalidPaymentPayload/InvalidChargeRequest's generic reason string

**Guidance.**
- crates/sorocharge-signer/src/entry.rs (ChargeParams), src/verify.rs (AmountMismatch check)
- Decide whether the check belongs in build_charge_entry, verify_entry, or both — CLAUDE.md's 'fail closed' standard argues for both
- Follow the existing no-catch-all error convention: one new SorochargeError variant, not a reused generic one

### fix(mpp): unsponsored settle path never caps timeBounds.maxTime when the challenge omits expires

`area:mpp`

**Description.** In MppServer::settle_pull_mode's unsponsored branch, the timeBounds.maxTime check only runs inside `if let Some(expires_at) = expires_at`. When the challenge has no expires field, the branch is skipped entirely — no cap is applied, unlike the parallel ledger-expiry check a few lines above, which correctly falls back to DEFAULT_CHALLENGE_EXPIRY_SECONDS.

**Why.** crates/sorocharge-mpp/src/server.rs:385-397. An unsponsored credential against a challenge with no `expires` can carry an arbitrarily long-lived timeBounds with no server-enforced cap, even though the code's own comment states the intended behavior (fall back to the 5-minute default) for exactly this case.

**Acceptance criteria.**
- [ ] When expires_at is None, the unsponsored branch computes max_allowed from DEFAULT_CHALLENGE_EXPIRY_SECONDS the same way the ledger-expiry check does, instead of skipping the check
- [ ] A test with no expires field and an unsponsored transaction whose maxTime exceeds the 5-minute default is rejected
- [ ] A test with no expires field and a maxTime within the default still succeeds

**Guidance.**
- crates/sorocharge-mpp/src/server.rs:340-398
- The fix should mirror the exact fallback pattern already used for seconds_until_expiry two lines above

### fix(x402): client's ledger-timeout computation carries no safety margin, unlike mpp's client

`area:x402`

**Description.** X402Client::get_with_payment sets valid_until_ledger to the full maxTimeoutSeconds allowance with no margin. MppClient::get_with_payment deliberately halves its allowance, with a comment explaining this is because slower-than-5s ledger closes can otherwise fail the facilitator's own ExpirationExceedsAllowance check — a failure mode already observed live.

**Why.** crates/sorocharge-x402/src/client.rs:177-181 vs crates/sorocharge-mpp/src/client.rs's div_ceil(2) margin. The two clients solve the identical problem inconsistently; x402's client is exposed to the exact failure mpp's already guards against.

**Acceptance criteria.**
- [ ] x402's client applies the same (or an equivalently justified) safety margin as mpp's client
- [ ] A comment explains why, referencing the same live-testnet reasoning
- [ ] A unit test demonstrates the computed valid_until_ledger is within the margin, not the full allowance

**Guidance.**
- crates/sorocharge-x402/src/client.rs:170-182
- crates/sorocharge-mpp/src/client.rs's existing comment on DEFAULT_ESTIMATED_LEDGER_SECONDS is the reference

### feat(signer): verify a Delegated entry's signatures against the account's real multisig/threshold policy

`area:signer`

**Description.** verify_entry's own doc comment admits: "A Delegated entry's account-contract-specific signing policy (how many delegates must sign, or which) is unknowable here — this only proves at least one attached signature is genuine." Add a path that queries the actual account's signer weights/thresholds (via simulation or ledger entry read) and verifies the attached signatures actually satisfy that policy, not just that one of them is cryptographically valid.

**Why.** crates/sorocharge-signer/src/verify.rs:162-167. This is a real, documented gap in the public API's own contract. It's currently unreachable via either shipped protocol (x402 forbids AddressWithDelegates outright; mpp only accepts legacy Address) but any direct caller of sorocharge-signer using Delegated credentials is exposed to it today.

**Acceptance criteria.**
- [ ] A new function (not a silent change to verify_entry's existing contract) that takes an RPC client and verifies a Delegated entry's signatures against the account's actual threshold
- [ ] A test with a 2-of-3 policy where only 1 signature is attached: rejected
- [ ] A test with a 2-of-3 policy where 2 valid signatures are attached: accepted
- [ ] Explicitly documented as unreachable via x402/mpp today, so callers understand the scope

**Guidance.**
- This requires reading the account's signers via getLedgerEntries or simulation — verify the actual RPC shape live, don't guess
- Keep this additive (a new function) rather than changing verify_entry's existing signature/behavior

### test(signer): add cargo-fuzz harness for SignedEntry/UnsignedEntry XDR decoding

`area:signer`

**Description.** No fuzz target exists anywhere in the repo despite verify_entry decoding attacker-supplied SorobanAuthorizationEntry XDR from the network. Add a cargo-fuzz target that feeds arbitrary bytes through SignedEntry::from_xdr and verify_entry, asserting only that it never panics.

**Why.** verify_entry is the exact boundary where untrusted network data meets this library — a panic here (vs. a clean error) would be a crash in whatever facilitator/server embeds this library.

**Acceptance criteria.**
- [ ] fuzz/fuzz_targets/verify_entry.rs exists and runs under cargo-fuzz
- [ ] A corpus seeded from the real golden-vector fixtures
- [ ] CI runs it for a bounded time budget (e.g. a nightly job, not blocking every PR)
- [ ] A README note on how to run it locally and extend the corpus

**Guidance.**
- crates/sorocharge-signer/src/verify.rs, src/sign.rs (SignedEntry::from_xdr)
- cargo-fuzz requires nightly Rust — document that constraint doesn't affect the library's own stable toolchain pin

### test(x402): add cargo-fuzz harness for parse_payment_transaction's TransactionEnvelope decoding

`area:x402`

**Description.** parse_payment_transaction decodes an attacker-supplied TransactionEnvelope from the PAYMENT-SIGNATURE header/VerifyRequest body. No fuzz target exists for this parse path specifically.

**Why.** crates/sorocharge-x402/src/tx.rs:120-196. This function is the exact boundary where a facilitator receives untrusted client input, structurally distinct from sorocharge-signer's own fuzz surface (different XDR root type, different invariants to check).

**Acceptance criteria.**
- [ ] fuzz/fuzz_targets/parse_payment_transaction.rs exists and runs under cargo-fuzz
- [ ] Corpus seeded from a real signed envelope produced by the test suite
- [ ] Asserts no panic across arbitrary byte input

**Guidance.**
- crates/sorocharge-x402/src/tx.rs
- Can share fuzz infrastructure/Cargo.toml setup with the signer crate's fuzz target from the sibling issue

### test(mpp): add cargo-fuzz harness for parse_payment_transaction's TransactionEnvelope decoding

`area:mpp`

**Description.** mpp has its own, separately-implemented parse_payment_transaction (not shared with x402's) that decodes an attacker-supplied TransactionEnvelope in both pull-mode settlement and push-mode's get_transaction response parsing. No fuzz target exists for it.

**Why.** crates/sorocharge-mpp/src/tx.rs. Same class of gap as x402's, but a genuinely separate implementation (the two crates don't share this parser) with its own invariants to validate under fuzzing.

**Acceptance criteria.**
- [ ] fuzz/fuzz_targets/parse_payment_transaction.rs exists for sorocharge-mpp specifically
- [ ] Corpus seeded from a real signed/sponsored and unsponsored envelope
- [ ] Asserts no panic across arbitrary byte input

**Guidance.**
- crates/sorocharge-mpp/src/tx.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### test(x402): unit-test Facilitator's safety checks against a mock RPC/HTTP layer

`area:ci-release`

**Description.** Facilitator::validate/verify/settle contains every facilitator-address-conflict check, the fee-ceiling enforcement, and the re-simulation gating — the single most safety-critical code path in the x402 crate — and has zero unit tests. stellar_rpc_client::Client is a concrete struct with no official test double, which is why this was never tested.

**Why.** crates/sorocharge-x402/src/facilitator.rs (442 lines, 0 #[test] functions). Every check in this file has only ever been exercised by manual live testnet runs during development, which aren't captured as regression tests in the repo.

**Acceptance criteria.**
- [ ] A mock HTTP server (e.g. via wiremock) stands in for the Stellar RPC endpoint stellar_rpc_client::Client talks to
- [ ] A test asserts FacilitatorAddressConflict when the facilitator's own address appears as transfer.from
- [ ] A test asserts FeeExceedsCeiling when the simulated fee exceeds config.max_transaction_fee_stroops
- [ ] A test asserts the sponsored transaction is rebuilt with the facilitator's account/sequence/fee, not the client's placeholder values

**Guidance.**
- stellar_rpc_client::Client::new takes a base_url — point it at a local mock JSON-RPC server rather than introducing a trait boundary, to avoid a breaking API change to Facilitator::new's public signature
- crates/sorocharge-x402/src/facilitator.rs

### test(mpp): unit-test MppServer's safety checks against a mock RPC/HTTP layer

`area:ci-release`

**Description.** MppServer::settle_credential/settle_pull_mode/settle_push_mode contains the server-address-conflict checks, fee-ceiling enforcement, timeBounds validation, and push/pull branching — and has zero unit tests, same gap as x402's Facilitator but a separate implementation with separate checks to cover (push mode has no x402 equivalent at all).

**Why.** crates/sorocharge-mpp/src/server.rs (493 lines, 0 #[test] functions). This file is where this wave's own core#9 fix (verify_entry's signature-binding bug) and the timeBounds gap in this same backlog were found — by reading, not by any existing test catching them.

**Acceptance criteria.**
- [ ] A mock HTTP server stands in for Stellar RPC
- [ ] A test asserts ServerAddressConflict when the server's own address appears as transfer.from or params.payer
- [ ] A test asserts PushModeForbidsFeePayer when a hash-type credential is submitted against a feePayer:true challenge
- [ ] A test asserts HashAlreadyConsumed is returned when already_consumed is true
- [ ] A test covers the timeBounds gap from the sibling fix issue, once that fix lands

**Guidance.**
- crates/sorocharge-mpp/src/server.rs
- Same mocking approach as the x402 Facilitator test issue — consider sharing the mock RPC server setup between the two

### test(x402): unit-test X402Client's full get_with_payment orchestration against a mock resource server

`area:ci-release`

**Description.** X402Client::get_with_payment — the full GET/402/decode/sign/retry flow — has zero unit tests. Only the lower-level tx.rs round-trip and manual live testnet runs exercise it.

**Why.** crates/sorocharge-x402/src/client.rs (222 lines, 0 #[test] functions).

**Acceptance criteria.**
- [ ] A mock HTTP server returns a 402 with a PAYMENT-REQUIRED header, then 200 on the signed retry
- [ ] A test asserts NoAcceptablePaymentRequirements when no accepts[] entry matches the client's network
- [ ] A test asserts the retry request carries a correctly-shaped PAYMENT-SIGNATURE header

**Guidance.**
- crates/sorocharge-x402/src/client.rs
- A fake Signer (already used in crates/sorocharge-x402/src/tx.rs's tests) covers the signing side

### test(mpp): unit-test MppClient's full get_with_payment orchestration against a mock resource server

`area:ci-release`

**Description.** MppClient::get_with_payment — challenge parsing, sponsored/unsponsored branching, the margin calculation — has zero unit tests.

**Why.** crates/sorocharge-mpp/src/client.rs (0 #[test] functions). The sponsored/unsponsored branch alone is a meaningfully different code path per call, neither one covered.

**Acceptance criteria.**
- [ ] A mock HTTP server returns a WWW-Authenticate challenge, then 200 on the signed retry
- [ ] A test exercises the sponsored (feePayer:true) path and asserts a Payload::Transaction with the all-zeros placeholder source
- [ ] A test exercises the unsponsored path and asserts the built transaction carries a real sequence number and timeBounds

**Guidance.**
- crates/sorocharge-mpp/src/client.rs
- The unsponsored path also needs a mock for get_account/simulate_transaction_envelope — same mock RPC approach as the server-side test issue

### refactor: deduplicate network_id_hash across x402::facilitator, mpp::client, and mpp::server

`area:ci-release`

**Description.** The identical sha256-of-passphrase helper fn network_id_hash is defined three separate times: crates/sorocharge-x402/src/facilitator.rs, crates/sorocharge-mpp/src/client.rs, and crates/sorocharge-mpp/src/server.rs.

**Why.** Three copies of the same five-line function is exactly the kind of drift risk CLAUDE.md's 'one audited signing engine, not two' principle exists to prevent — if one copy is ever touched and the others aren't, they silently diverge.

**Acceptance criteria.**
- [ ] A single network_id_hash (or equivalent) lives in sorocharge-signer and is exported for the protocol crates to use
- [ ] All three call sites are updated to use it
- [ ] No behavior change — existing tests still pass unmodified

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs, crates/sorocharge-mpp/src/client.rs, crates/sorocharge-mpp/src/server.rs
- sorocharge-signer already computes a network_id internally (sign.rs's compute_network_id) — this may already be exactly the function needed, just not exported

### refactor: deduplicate caip2_for_passphrase between sorocharge-x402 and sorocharge-mpp

`area:ci-release`

**Description.** crates/sorocharge-x402/src/network.rs and crates/sorocharge-mpp/src/network.rs define byte-identical PUBNET_PASSPHRASE/TESTNET_PASSPHRASE constants and an identical caip2_for_passphrase match, differing only in which crate's Error type they return.

**Why.** Same duplication risk as network_id_hash — a third network ever being supported would need to be added in two places by hand, with no compiler check that both were updated.

**Acceptance criteria.**
- [ ] The passphrase constants and the CAIP-2 mapping logic live in one place (e.g. sorocharge-signer), with each crate's thin wrapper only translating the error type
- [ ] No behavior change to either crate's public API
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/network.rs, crates/sorocharge-mpp/src/network.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### chore: add cargo-deny to CI for dependency vulnerability and license scanning

`area:ci-release`

**Description.** Every dependency in the workspace is exact-pinned (= versions) with zero automated scanning for known CVEs or license issues in the resolved dependency tree.

**Why.** Exact pins mean no dependency update ever happens automatically — combined with zero scanning, a known CVE in a transitive dependency could sit unnoticed indefinitely in a library that signs payments.

**Acceptance criteria.**
- [ ] A deny.toml with an explicit advisory/license policy, checked into the repo
- [ ] A CI job runs cargo-deny on every PR and fails on a new advisory or disallowed license
- [ ] Any current advisory exceptions are explicitly listed with a reason, not silently ignored

**Guidance.**
- .github/workflows/ci.yml, root Cargo.toml's workspace.dependencies
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### perf: add criterion benchmarks for sorocharge-signer's signing/verification hot path

`area:ci-release`

**Description.** No benchmark exists anywhere in the repo for build_charge_entry, sign_entry, or verify_entry, despite these sitting in a latency-sensitive facilitator/server request path.

**Why.** Without a baseline, a future change's performance impact (e.g. the delegated-policy verification feature elsewhere in this backlog) can't be measured against where the library stands today.

**Acceptance criteria.**
- [ ] benches/signer_benchmarks.rs using criterion, covering build_charge_entry, sign_entry, verify_entry for each CredentialKind
- [ ] cargo bench runs successfully in CI (or is documented as a manual, non-blocking step)
- [ ] A baseline result is recorded in the PR description for future comparison

**Guidance.**
- crates/sorocharge-signer/src/entry.rs, sign.rs, verify.rs
- criterion is a dev-dependency only — it must not affect the library's runtime dependency tree

### docs: write a THREAT_MODEL.md

`area:ci-release`

**Description.** No consolidated document lists what this library defends against (facilitator-address-conflict, signature-key-binding, replay via nonce, expiration-allowance bypass) versus what's explicitly out of scope (delegated-signer threshold policy today, MPP session mode, key custody).

**Why.** This wave alone found and fixed a real signature-binding vulnerability (core#9) by reading code, not by checking against a written threat model — because none exists. A written one would make the next review faster and more complete.

**Acceptance criteria.**
- [ ] THREAT_MODEL.md lists each class of attacker (malicious payer, malicious/compromised resource server, malicious facilitator-adjacent party) and what's defended against for each
- [ ] Explicitly lists current known gaps (e.g. the delegated-signer threshold-policy gap from this same backlog) rather than implying full coverage
- [ ] Linked from the README and the docs site

**Guidance.**
- crates/sorocharge-signer/src/verify.rs's own doc comments are a good starting inventory of checks already made
- sorocharge-core#9's fix and report are a concrete worked example of the kind of finding this document should anticipate

### chore: add cargo-semver-checks to CI ahead of future crates.io publishes

`area:ci-release`

**Description.** Three tagged releases exist (v0.1.0, v0.1.1, v0.1.2) and crates.io publishing is already tracked (core#7), but nothing currently checks whether a change accidentally breaks semver compatibility before a publish.

**Why.** Once published, an accidental breaking change in a patch release is far more disruptive than one caught before publish — sorocharge-php depends on this crate's exact API shape.

**Acceptance criteria.**
- [ ] A CI job (or a documented pre-publish step) runs cargo-semver-checks against the last published version
- [ ] Documented in CONTRIBUTING.md once that file exists (core#5)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- .github/workflows/ci.yml
- Depends on core#7 (crates.io publish) existing first to have a prior version to check against

### fix: audit lossy u32/i64 casts in the settlement fee and ledger-arithmetic paths

`area:ci-release`

**Description.** facilitator.rs, server.rs, and client.rs each contain multiple `u32::try_from(...).unwrap_or(u32::MAX)` and similar lossy casts converting fee/ledger-timeout calculations between i64/u64/u32. Each individually has a plausible safety argument, but none is documented, and they've never been collected and reviewed together.

**Why.** crates/sorocharge-x402/src/facilitator.rs:385, crates/sorocharge-mpp/src/server.rs:350,458, crates/sorocharge-mpp/src/client.rs: similar. A silent saturate-to-MAX on a fee calculation, under a misconfigured operator ceiling, could submit a transaction with a materially wrong fee instead of erroring.

**Acceptance criteria.**
- [ ] Every lossy numeric cast in the settlement/ledger-arithmetic paths is listed with a one-line justification (comment) or replaced with a checked conversion that errors instead of saturating
- [ ] A test exercises at least one previously-silent saturation case and confirms it now errors (or documents why saturating remains correct there)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- Search for `unwrap_or(u32::MAX)` and `unwrap_or(i64::MAX)` across crates/sorocharge-x402 and crates/sorocharge-mpp
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### fix: no retry/backoff for transient HTTP failures in X402Client

`area:x402`

**Description.** X402Client::get_with_payment makes exactly two GET requests with no retry logic. A transient 5xx or connection blip on either request fails the whole payment flow immediately.

**Why.** crates/sorocharge-x402/src/client.rs. A payer client embedded in an automated agent is exactly the kind of caller that benefits from a bounded retry on a transient failure rather than failing a payment outright.

**Acceptance criteria.**
- [ ] A documented, bounded retry policy (e.g. N attempts with backoff) for connection-level failures and 5xx responses, distinct from a deliberate 402/200 response
- [ ] A test using a mock server that fails once then succeeds confirms the retry recovers
- [ ] The existing request_timeout bound is preserved per-attempt, not multiplied unboundedly

**Guidance.**
- crates/sorocharge-x402/src/client.rs's X402Client::new already takes a timeout — the retry policy should compose with it, not replace it
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### fix: no retry/backoff for transient HTTP failures in MppClient

`area:mpp`

**Description.** Same gap as X402Client: MppClient::get_with_payment has no retry logic for transient HTTP failures, and the unsponsored path adds two more RPC calls (get_account, simulate_transaction_envelope) with no retry either.

**Why.** crates/sorocharge-mpp/src/client.rs. A separate implementation from x402's client, so this is a distinct fix even though the symptom is identical.

**Acceptance criteria.**
- [ ] A documented, bounded retry policy for connection-level/5xx failures on both the HTTP and RPC calls
- [ ] A test using a mock server/RPC endpoint that fails once then succeeds confirms recovery
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/client.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### refactor: Facilitator<'a> and MppServer<'a> borrow their signer/RPC client by reference, forcing lifetime gymnastics in typical embedding code

`area:ci-release`

**Description.** Both structs hold `&'a dyn Signer` and `&'a stellar_rpc_client::Client`. A typical axum/actix handler wants to share one Facilitator/MppServer across requests via Arc<Facilitator>, but a borrowed lifetime makes that awkward — the signer and RPC client have to outlive the facilitator for the entire process, which is usually fine but is an unusual API shape most Rust web services don't expect from a library.

**Why.** crates/sorocharge-x402/src/facilitator.rs's Facilitator<'a>, crates/sorocharge-mpp/src/server.rs's MppServer<'a>. This is exactly the kind of ergonomics issue that surfaces once someone tries to actually deploy this behind a real web framework, not during isolated testing.

**Acceptance criteria.**
- [ ] A design proposal (comment on the issue) comparing owning Arc<dyn Signer>/Arc<Client> vs the current borrowed-reference shape, with a recommendation
- [ ] If changed, existing examples (resource_server_x402.rs, resource_server_mpp.rs) are updated to the new shape
- [ ] This is a breaking API change — flagged as requiring a minor (not patch) version bump

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs, crates/sorocharge-mpp/src/server.rs
- Resolve this before crates.io publish (core#7) if changing it — a breaking change post-publish is far more disruptive

### test(signer): add property-based (proptest) round-trip tests for XDR construction

`area:signer`

**Description.** Golden vectors prove specific fixed inputs byte-match the reference SDK, but nothing systematically checks that build_charge_entry's output round-trips through XDR encode/decode correctly across the input space (arbitrary valid addresses, amounts, ledger numbers, delegate counts).

**Why.** Fuzzing (tracked separately in this backlog) finds crashes; proptest finds round-trip/invariant violations that don't crash but are still wrong — a different and complementary kind of bug.

**Acceptance criteria.**
- [ ] A proptest strategy generates valid ChargeParams + CredentialKind combinations
- [ ] A property asserts build_charge_entry's output XDR-encodes then XDR-decodes to an identical structure
- [ ] At least one property specifically exercises the Delegated variant's sort-order invariant across randomized signer sets

**Guidance.**
- crates/sorocharge-signer/src/entry.rs
- proptest is a dev-dependency only

### test(x402): add property-based (proptest) round-trip tests for transaction construction

`area:x402`

**Description.** build_payment_transaction_envelope / parse_payment_transaction have fixed-input tests but no property-based round-trip coverage across the input space.

**Why.** crates/sorocharge-x402/src/tx.rs. Same complementary-to-fuzzing rationale as the signer crate's proptest issue.

**Acceptance criteria.**
- [ ] A proptest strategy generates valid ChargeParams and signed entries
- [ ] A property asserts build-then-parse round-trips the transfer args exactly, for every CredentialKind the scheme allows
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/tx.rs's existing build_then_parse_round_trips_the_transfer_args test is the fixed-input version to generalize
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### test(mpp): add property-based (proptest) round-trip tests for transaction construction

`area:mpp`

**Description.** mpp's own, separately-implemented tx.rs has fixed-input tests but no property-based round-trip coverage, for both the sponsored and unsponsored transaction-building paths.

**Why.** crates/sorocharge-mpp/src/tx.rs. Distinct implementation from x402's, so a distinct property-test target.

**Acceptance criteria.**
- [ ] A proptest strategy covers both build_sponsored_envelope and build_unsponsored_transaction
- [ ] A property asserts each round-trips through parse_payment_transaction correctly
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/tx.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### research: confirm @stellar/mpp's channel/session-intent stability before any session-mode implementation

`area:mpp`

**Description.** A live check during this wave found conflicting signals: the official MPP Session Guide documents a one-way-channel contract and prepare_commitment flow, but a separate community source claims the channel intent was non-functional and removed in @stellar/mpp 0.7.0. CLAUDE.md's original brief already flagged this spec as 'still being drafted' and deliberately deferred it. Produce a definitive, sourced answer: is it currently implementable against a real, stable SDK version or not.

**Why.** Building real implementation issues against an unconfirmed or possibly-removed spec would be exactly the kind of disguised-investigation this program's own rules warn against. This issue exists so that decision is made deliberately, once, with evidence — not assumed either way.

**Acceptance criteria.**
- [ ] A written finding naming the exact @stellar/mpp version checked, whether the channel/session intent is present and functional in it, and a link to its actual source (not just docs)
- [ ] If stable: a scoped recommendation for what a first real implementation issue should cover
- [ ] If not stable or removed: that is an acceptable, complete outcome for this issue — no implementation issues should be opened against it until this is resolved

**Guidance.**
- https://developers.stellar.org/docs/build/agentic-payments/mpp/channel-guide is the official guide but doesn't give exact contract function signatures
- Check the actual published @stellar/mpp package version and changelog, and the one-way-channel contract's real source, not secondhand summaries

### test(signer): add golden-vector coverage for a contract-address recipient

`area:signer`

**Description.** Every existing fixture uses a plain G... account as both payer and recipient. SEP-41 transfer's `to` argument can structurally be a contract address (ScAddress::Contract) — e.g. paying into a vault contract — and this path is never exercised by any test.

**Why.** crates/sorocharge-signer/src/entry.rs, verify.rs. ScAddress::Contract only ever appears in the test suite as a deliberately-wrong value for a mismatch test, never as a legitimately-constructed recipient.

**Acceptance criteria.**
- [ ] A new golden-vector fixture (or a non-golden unit test, if generating a new reference-SDK fixture isn't practical) with recipient set to a contract address
- [ ] build_charge_entry and verify_entry both pass for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- tests/golden_vectors/generate.mjs for how existing fixtures are produced
- crates/sorocharge-signer/src/verify.rs:356 shows ScAddress::Contract already used as a test value, just for the wrong purpose

### test(signer): add golden-vector coverage for amount == 1 and amount == i128::MAX

`area:signer`

**Description.** No existing fixture exercises either numeric boundary of the i128 amount field — the smallest possible positive transfer and the largest representable value.

**Why.** Int128Parts' hi/lo split (entry.rs:179-182) is exactly the kind of bit-manipulation code where an off-by-one at a boundary would silently corrupt the amount without any existing test catching it.

**Acceptance criteria.**
- [ ] A golden-vector fixture (or equivalent reference-checked test) for amount = 1
- [ ] A golden-vector fixture for amount = i128::MAX
- [ ] Both pass through build_charge_entry and verify_entry unchanged

**Guidance.**
- crates/sorocharge-signer/src/entry.rs:179-182 (build_transfer_invocation's Int128Parts construction)
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### test(signer): add golden-vector coverage for 3+ delegate signers

`area:signer`

**Description.** The only existing Delegated-credential fixture's test asserts `signers.len() >= 2` as its sole lower bound, with no evidence any fixture exercises more than the minimum 2 — the sort-order and duplicate-detection logic is never exercised at a larger scale.

**Why.** crates/sorocharge-signer/src/entry.rs's delegated_transfer_matches_reference_sdk_byte_for_byte test and build_delegated_credentials' sort/dedup logic.

**Acceptance criteria.**
- [ ] A golden-vector fixture with 3 or more delegate signers, in deliberately unsorted input order
- [ ] The test confirms the output is correctly sorted by XDR-encoded address bytes, matching the reference SDK
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/entry.rs:99-163 (build_delegated_credentials)
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### docs(signer): audit every public item for a 'why' explanation, not just a signature restatement

`area:signer`

**Description.** CLAUDE.md §8 requires every public function's doc comment to explain why it exists and what it guarantees, not just restate its signature. Audit sorocharge-signer's public API against that bar specifically (most already meet it; confirm all do).

**Why.** crates/sorocharge-signer/src/lib.rs's public exports are the audit scope. This is a compliance check against the project's own stated rule, not a generic doc-quality pass.

**Acceptance criteria.**
- [ ] Every public function/type/trait in sorocharge-signer's exports has a doc comment stating why it exists or what it guarantees, verified against CLAUDE.md §8's literal bar
- [ ] Any found lacking is fixed in the same PR
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/lib.rs's pub use list is the exact audit scope
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### docs(x402): audit every public item for a 'why' explanation, not just a signature restatement

`area:x402`

**Description.** Same CLAUDE.md §8 compliance audit as the signer crate, scoped to sorocharge-x402's public exports.

**Why.** crates/sorocharge-x402/src/lib.rs.

**Acceptance criteria.**
- [ ] Every public item in sorocharge-x402's exports meets CLAUDE.md §8's bar
- [ ] Any found lacking is fixed in the same PR
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/lib.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### docs(mpp): audit every public item for a 'why' explanation, not just a signature restatement

`area:mpp`

**Description.** Same CLAUDE.md §8 compliance audit as the signer crate, scoped to sorocharge-mpp's public exports.

**Why.** crates/sorocharge-mpp/src/lib.rs.

**Acceptance criteria.**
- [ ] Every public item in sorocharge-mpp's exports meets CLAUDE.md §8's bar
- [ ] Any found lacking is fixed in the same PR
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/lib.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### fix(x402): cap individual wire-type field lengths (description, resource.url) independent of overall response size

`area:x402`

**Description.** Even with an overall response-body size cap (tracked separately in this backlog), nothing bounds individual string fields like ResourceInfo.description or ResourceInfo.url within that budget — a resource server could spend the entire allowed response size on one field.

**Why.** crates/sorocharge-x402/src/types.rs's ResourceInfo/PaymentRequired. A defense-in-depth complement to the overall body-size cap, not a duplicate of it.

**Acceptance criteria.**
- [ ] Explicit, documented maximum lengths for description, url, and other free-text fields, enforced on deserialize
- [ ] A test confirms an oversized field is rejected with a specific error, not silently truncated
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- Depends conceptually on the overall body-size-cap issue but is independently actionable

### fix(mpp): cap individual wire-type field lengths (description, realm) independent of overall response size

`area:mpp`

**Description.** Same gap as x402's: Challenge.description, Challenge.realm, and ChargeRequest.description have no individual length bound.

**Why.** crates/sorocharge-mpp/src/types.rs.

**Acceptance criteria.**
- [ ] Explicit, documented maximum lengths for free-text fields, enforced on deserialize
- [ ] A test confirms an oversized field is rejected with a specific error
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/types.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### fix: bound response-body size in X402Client against a malicious/compromised resource server

`area:x402`

**Description.** X402Client's reqwest::Client has a request timeout but no response-body size limit. A malicious or compromised resource server can return an unbounded 402 or 200 response body.

**Why.** crates/sorocharge-x402/src/client.rs's X402Client::new. The client chose to GET this URL, but that doesn't make an unbounded response safe to fully buffer.

**Acceptance criteria.**
- [ ] A documented maximum response size, enforced before full buffering (e.g. via a streaming read with an early abort)
- [ ] A test confirms an oversized response is rejected with a specific error rather than exhausting memory
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/client.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### fix: bound response-body size in MppClient against a malicious/compromised resource server

`area:mpp`

**Description.** Same gap as X402Client: MppClient's reqwest::Client has no response-body size limit.

**Why.** crates/sorocharge-mpp/src/client.rs's MppClient::new.

**Acceptance criteria.**
- [ ] A documented maximum response size, enforced before full buffering
- [ ] A test confirms an oversized response is rejected with a specific error
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/client.rs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### chore: re-verify every exact-pinned dependency version is still current

`area:ci-release`

**Description.** Root Cargo.toml exact-pins stellar-xdr, stellar-rpc-client, ed25519-dalek, reqwest, and others. These were verified against crates.io at the time CLAUDE.md's §0 was executed; real time has passed since, so a fresh check is overdue, per the project's own 'verify, don't assume' standard.

**Why.** Root Cargo.toml's [workspace.dependencies]. An exact-pin strategy only stays safe if someone actually re-checks it periodically — otherwise it just freezes at whatever was current on day one.

**Acceptance criteria.**
- [ ] Each exact-pinned dependency's current latest version on crates.io is checked and reported
- [ ] Any safe bump (no breaking changes, confirmed by changelog) is applied with each pin updated individually, not as one bulk bump
- [ ] Any version deliberately NOT bumped (e.g. a breaking major version) is noted with a reason

**Guidance.**
- Root Cargo.toml's [workspace.dependencies]
- This is exactly the kind of check CLAUDE.md §0 originally performed once — this issue is its periodic re-run

### chore: enforce the missing_docs lint in CI

`area:ci-release`

**Description.** CLAUDE.md §8 requires every public function to have a doc comment, but nothing in CI actually enforces this — a new public item with no doc comment would currently pass CI silently.

**Why.** .github/workflows/ci.yml has no missing_docs check, and no crate sets #![warn(missing_docs)] or deny.

**Acceptance criteria.**
- [ ] Each of the three crates' lib.rs sets #![deny(missing_docs)] (or CI runs `cargo doc` with RUSTDOCFLAGS="-D missing_docs")
- [ ] CI fails if a new public item lacks a doc comment
- [ ] Existing code already passes (confirmed by running it before merging)

**Guidance.**
- crates/*/src/lib.rs, .github/workflows/ci.yml
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### ci: smoke-test examples against a local Stellar quickstart container instead of relying on manual testnet runs

`area:ci-release`

**Description.** CI builds the examples (`cargo build --all-targets`) but never runs them — every example has only ever been exercised by manual testnet runs during development, which leave no regression coverage in the repo.

**Why.** .github/workflows/ci.yml, crates/*/examples/. A local quickstart image (stellar/quickstart) would let CI actually run verify_and_settle and the resource-server examples end to end, without depending on live testnet availability.

**Acceptance criteria.**
- [ ] A CI job runs the stellar/quickstart Docker image in standalone mode
- [ ] At least one example (verify_and_settle) runs against it successfully in CI
- [ ] The job is allowed to be slower/separate from the main fast test job, matching the existing golden-vectors job's pattern

**Guidance.**
- .github/workflows/ci.yml's existing two-job structure (build-test, golden-vectors) is the pattern to extend
- crates/sorocharge-x402/examples/verify_and_settle.rs

### docs: add a crates.io README for each of the three crates

`area:ci-release`

**Description.** None of the three crates' Cargo.toml sets a readme field, so once published (core#7) none will render a README on crates.io — only the repo-root README exists.

**Why.** crates/sorocharge-signer/Cargo.toml, crates/sorocharge-x402/Cargo.toml, crates/sorocharge-mpp/Cargo.toml all omit readme.

**Acceptance criteria.**
- [ ] Each crate has its own README.md (can be short, linking to the main docs site) referenced via Cargo.toml's readme field
- [ ] cargo publish --dry-run (or equivalent check) confirms each crate's README renders
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/*/Cargo.toml's [package] section
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### feat: instrument Facilitator and MppServer's settlement path with structured tracing spans

`area:ci-release`

**Description.** Zero usage of the tracing crate (or any logging) exists anywhere in the repo. The settlement path — the exact code an operator needs visibility into when a real payment fails in production — currently produces no structured signal at all.

**Why.** crates/sorocharge-x402/src/facilitator.rs, crates/sorocharge-mpp/src/server.rs. A failed settlement today is debuggable only by reading the returned error string after the fact, with no span/trace context.

**Acceptance criteria.**
- [ ] tracing spans wrap verify/settle (x402) and settle_credential (mpp), tagged with enough context (payer, amount, network) to correlate a failure without logging secrets
- [ ] A documented recommendation for how an embedding app wires a subscriber (this crate should not force a specific subscriber/format)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs, crates/sorocharge-mpp/src/server.rs
- tracing is additive and should be a lightweight, optional-feature dependency if it risks affecting consumers who don't want it

### test: cross-check sorocharge-signer's golden vectors against soroauth-go's own construction

`area:signer`

**Description.** CLAUDE.md states this project is 'the direct successor to soroauth-go' and explicitly asks to carry over its verification discipline. Golden vectors so far are checked against the JS reference SDK only — never cross-checked against the sibling Go library's own output for the same inputs.

**Why.** CLAUDE.md's introduction (soroauth-go reference). A second independent implementation agreeing on the same bytes is stronger evidence than one JS SDK alone, for the same reason CLAUDE.md asks for a reference-SDK diff in the first place.

**Acceptance criteria.**
- [ ] For at least the legacy and AddressV2 fixtures, soroauth-go's own entry construction is run against the same inputs and diffed byte-for-byte against sorocharge-signer's output
- [ ] Any discrepancy is reported and resolved (or explicitly explained) before this issue closes
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- The soroauth-go repository (sibling project) — read its actual construction code, don't assume it matches
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### test(signer): assert SorochargeError::InvalidAddress is returned for a strkey that fails to decode into an ScAddress

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::InvalidAddress is the specific error returned for a strkey that fails to decode into an ScAddress — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs a strkey that fails to decode into an ScAddress and asserts the result is exactly Err(SorochargeError::InvalidAddress)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(signer): assert SorochargeError::UnsupportedCredentialType is returned for SOROBAN_CREDENTIALS_SOURCE_ACCOUNT passed to verify_entry

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::UnsupportedCredentialType is the specific error returned for SOROBAN_CREDENTIALS_SOURCE_ACCOUNT passed to verify_entry — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs SOROBAN_CREDENTIALS_SOURCE_ACCOUNT passed to verify_entry and asserts the result is exactly Err(SorochargeError::UnsupportedCredentialType)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(signer): assert SorochargeError::NoMatchingCredentialNode is returned for a Signer whose address matches no node in the entry being signed

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::NoMatchingCredentialNode is the specific error returned for a Signer whose address matches no node in the entry being signed — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs a Signer whose address matches no node in the entry being signed and asserts the result is exactly Err(SorochargeError::NoMatchingCredentialNode)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(signer): assert SorochargeError::SimulationEventsMalformed is returned for a simulation event that fails to decode or lacks expected topics/data

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::SimulationEventsMalformed is the specific error returned for a simulation event that fails to decode or lacks expected topics/data — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs a simulation event that fails to decode or lacks expected topics/data and asserts the result is exactly Err(SorochargeError::SimulationEventsMalformed)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(signer): assert SorochargeError::UnexpectedBalanceChange is returned for a simulation showing a balance change other than the expected payment

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::UnexpectedBalanceChange is the specific error returned for a simulation showing a balance change other than the expected payment — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs a simulation showing a balance change other than the expected payment and asserts the result is exactly Err(SorochargeError::UnexpectedBalanceChange)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(signer): assert SorochargeError::ExpectedTransferMissing is returned for a simulation that doesn't show the expected transfer at all

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::ExpectedTransferMissing is the specific error returned for a simulation that doesn't show the expected transfer at all — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs a simulation that doesn't show the expected transfer at all and asserts the result is exactly Err(SorochargeError::ExpectedTransferMissing)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(signer): assert SorochargeError::SigningFailed is returned for a Signer implementation that returns an error, and a non-Ed25519-account signer address

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::SigningFailed is the specific error returned for a Signer implementation that returns an error, and a non-Ed25519-account signer address — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs a Signer implementation that returns an error, and a non-Ed25519-account signer address and asserts the result is exactly Err(SorochargeError::SigningFailed)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(signer): assert SorochargeError::XdrEncodingFailed is returned for a value that doesn't fit its XDR-constrained shape

`area:signer`

**Description.** No existing test in crates/sorocharge-signer currently asserts SorochargeError::XdrEncodingFailed is the specific error returned for a value that doesn't fit its XDR-constrained shape — this error path exists in the code but has no regression coverage.

**Why.** CLAUDE.md's own constraints checklist requires 'every public error variant is distinct enough that a caller can act on it differently' — that guarantee is only actually proven by a test exercising it, which this variant currently lacks.

**Acceptance criteria.**
- [ ] A test constructs a value that doesn't fit its XDR-constrained shape and asserts the result is exactly Err(SorochargeError::XdrEncodingFailed)
- [ ] The test fails (not panics) if the code is changed to return a different error for this case
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs for the variant's exact fields
- crates/sorocharge-signer/src/verify.rs or entry.rs/sign.rs/effects.rs for where it's actually returned

### test(x402): assert X402Error::HttpRequestFailed is returned for the underlying HTTP request failing (connection/TLS/timeout)

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::HttpRequestFailed is returned for the underlying HTTP request failing (connection/TLS/timeout). All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs the underlying HTTP request failing (connection/TLS/timeout) and asserts the result is exactly Err(X402Error::HttpRequestFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::UnexpectedStatus is returned for a resource responding with neither 402 nor 200

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::UnexpectedStatus is returned for a resource responding with neither 402 nor 200. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a resource responding with neither 402 nor 200 and asserts the result is exactly Err(X402Error::UnexpectedStatus) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::MissingPaymentRequiredHeader is returned for a 402 response with no PAYMENT-REQUIRED header

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::MissingPaymentRequiredHeader is returned for a 402 response with no PAYMENT-REQUIRED header. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a 402 response with no PAYMENT-REQUIRED header and asserts the result is exactly Err(X402Error::MissingPaymentRequiredHeader) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::InvalidPaymentRequired is returned for a PAYMENT-REQUIRED header that doesn't base64/JSON-decode

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::InvalidPaymentRequired is returned for a PAYMENT-REQUIRED header that doesn't base64/JSON-decode. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a PAYMENT-REQUIRED header that doesn't base64/JSON-decode and asserts the result is exactly Err(X402Error::InvalidPaymentRequired) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::NoAcceptablePaymentRequirements is returned for no accepts[] entry matching the client's scheme/network

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::NoAcceptablePaymentRequirements is returned for no accepts[] entry matching the client's scheme/network. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs no accepts[] entry matching the client's scheme/network and asserts the result is exactly Err(X402Error::NoAcceptablePaymentRequirements) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::UnsupportedNetwork is returned for a network identifier with no CAIP-2 mapping

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::UnsupportedNetwork is returned for a network identifier with no CAIP-2 mapping. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a network identifier with no CAIP-2 mapping and asserts the result is exactly Err(X402Error::UnsupportedNetwork) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::Signing is returned for sorocharge-signer failing to build or sign a charge entry

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::Signing is returned for sorocharge-signer failing to build or sign a charge entry. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs sorocharge-signer failing to build or sign a charge entry and asserts the result is exactly Err(X402Error::Signing) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::XdrEncodingFailed is returned for a value that doesn't fit its XDR-constrained shape

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::XdrEncodingFailed is returned for a value that doesn't fit its XDR-constrained shape. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a value that doesn't fit its XDR-constrained shape and asserts the result is exactly Err(X402Error::XdrEncodingFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::InvalidPaymentPayload is returned for a credential/verify-request payload that doesn't deserialize into the expected shape

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::InvalidPaymentPayload is returned for a credential/verify-request payload that doesn't deserialize into the expected shape. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a credential/verify-request payload that doesn't deserialize into the expected shape and asserts the result is exactly Err(X402Error::InvalidPaymentPayload) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::XdrDecodingFailed is returned for a submitted transaction's XDR failing to decode

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::XdrDecodingFailed is returned for a submitted transaction's XDR failing to decode. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a submitted transaction's XDR failing to decode and asserts the result is exactly Err(X402Error::XdrDecodingFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::UnexpectedTransactionShape is returned for a transaction that doesn't authorize a single bare SEP-41 transfer

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::UnexpectedTransactionShape is returned for a transaction that doesn't authorize a single bare SEP-41 transfer. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a transaction that doesn't authorize a single bare SEP-41 transfer and asserts the result is exactly Err(X402Error::UnexpectedTransactionShape) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::ForbiddenCredentialType is returned for sorobanCredentialsSourceAccount or sorobanCredentialsAddressWithDelegates on the exact scheme

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::ForbiddenCredentialType is returned for sorobanCredentialsSourceAccount or sorobanCredentialsAddressWithDelegates on the exact scheme. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs sorobanCredentialsSourceAccount or sorobanCredentialsAddressWithDelegates on the exact scheme and asserts the result is exactly Err(X402Error::ForbiddenCredentialType) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::FacilitatorAddressConflict is returned for the facilitator's own address appearing as source/from/an authorization entry

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::FacilitatorAddressConflict is returned for the facilitator's own address appearing as source/from/an authorization entry. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs the facilitator's own address appearing as source/from/an authorization entry and asserts the result is exactly Err(X402Error::FacilitatorAddressConflict) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::RpcFailed is returned for a Stellar RPC call failing

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::RpcFailed is returned for a Stellar RPC call failing. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a Stellar RPC call failing and asserts the result is exactly Err(X402Error::RpcFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::FeeExceedsCeiling is returned for a settlement fee exceeding the configured safety ceiling

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::FeeExceedsCeiling is returned for a settlement fee exceeding the configured safety ceiling. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a settlement fee exceeding the configured safety ceiling and asserts the result is exactly Err(X402Error::FeeExceedsCeiling) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(x402): assert X402Error::SettlementFailed is returned for verification succeeding but on-chain settlement failing

`area:x402`

**Description.** No existing test in crates/sorocharge-x402 asserts X402Error::SettlementFailed is returned for verification succeeding but on-chain settlement failing. All 16 of X402Error's variants currently have zero direct unit-test coverage — only live testnet runs during development have exercised any of them.

**Why.** Same CLAUDE.md error-distinctness guarantee as the signer crate's variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs verification succeeding but on-chain settlement failing and asserts the result is exactly Err(X402Error::SettlementFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/error.rs for the variant's exact fields
- crates/sorocharge-x402/src/facilitator.rs or client.rs/tx.rs for where it's returned
- Several of these variants (RpcFailed, FacilitatorAddressConflict, FeeExceedsCeiling, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::HttpRequestFailed is returned for the underlying HTTP request failing

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::HttpRequestFailed is returned for the underlying HTTP request failing. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs the underlying HTTP request failing and asserts the result is exactly Err(MppError::HttpRequestFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::UnexpectedStatus is returned for a resource responding with neither 402 nor 200

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::UnexpectedStatus is returned for a resource responding with neither 402 nor 200. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a resource responding with neither 402 nor 200 and asserts the result is exactly Err(MppError::UnexpectedStatus) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::MissingChallenge is returned for a 402 response with no WWW-Authenticate: Payment header

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::MissingChallenge is returned for a 402 response with no WWW-Authenticate: Payment header. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a 402 response with no WWW-Authenticate: Payment header and asserts the result is exactly Err(MppError::MissingChallenge) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::InvalidChallenge is returned for auth-params that don't parse or omit a required field

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::InvalidChallenge is returned for auth-params that don't parse or omit a required field. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs auth-params that don't parse or omit a required field and asserts the result is exactly Err(MppError::InvalidChallenge) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::UnsupportedMethod is returned for a challenge naming a method other than stellar

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::UnsupportedMethod is returned for a challenge naming a method other than stellar. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a challenge naming a method other than stellar and asserts the result is exactly Err(MppError::UnsupportedMethod) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::UnsupportedIntent is returned for a challenge naming an intent other than charge

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::UnsupportedIntent is returned for a challenge naming an intent other than charge. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a challenge naming an intent other than charge and asserts the result is exactly Err(MppError::UnsupportedIntent) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::InvalidChargeRequest is returned for a request field that doesn't base64url-decode to valid JSON

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::InvalidChargeRequest is returned for a request field that doesn't base64url-decode to valid JSON. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a request field that doesn't base64url-decode to valid JSON and asserts the result is exactly Err(MppError::InvalidChargeRequest) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::UnsupportedNetwork is returned for a network identifier with no CAIP-2 mapping

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::UnsupportedNetwork is returned for a network identifier with no CAIP-2 mapping. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a network identifier with no CAIP-2 mapping and asserts the result is exactly Err(MppError::UnsupportedNetwork) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(signer): add a reference-SDK-signed fixture and acceptance test for AddressV2 credentials

`area:signer`

**Description.** Only legacy_transfer_signed.json exists among the committed fixtures — verify_entry's 'accepts real reference-SDK output, not just entries this library produced itself' proof (see its test doc comment) currently only covers the Legacy credential path. address_v2_transfer.json exists but is unsigned.

**Why.** tests/golden_vectors/fixtures/ currently has no address_v2_transfer_signed.json. verify_entry claims to handle AddressV2 (CAP-71, mandatory from protocol 28 onward) but that claim is never checked against a genuine reference-SDK signature for this credential type.

**Acceptance criteria.**
- [ ] A new address_v2_transfer_signed.json fixture, signed by the pinned @stellar/stellar-sdk the same way legacy_transfer_signed.json was
- [ ] A new test mirroring accepts_a_valid_entry_signed_by_the_reference_sdk, using this fixture
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- tests/golden_vectors/generate.mjs for how legacy_transfer_signed.json was produced — the AddressV2 fixture should follow the identical process
- crates/sorocharge-signer/src/verify.rs's existing test module

### test(signer): add a reference-SDK-signed fixture and acceptance test for Delegated credentials

`area:signer`

**Description.** Same gap as the AddressV2 fixture issue, for the Delegated credential kind: delegated_transfer.json exists but is unsigned, and no signed counterpart exists.

**Why.** verify_entry's Delegated path (multi-signer, sort-order-sensitive) is the most structurally complex credential kind this library handles, and it's the one with the least verification against a genuine reference-SDK signature.

**Acceptance criteria.**
- [ ] A new delegated_transfer_signed.json fixture with at least 2 delegate signatures, signed by the pinned @stellar/stellar-sdk
- [ ] A new test mirroring accepts_a_valid_entry_signed_by_the_reference_sdk, using this fixture
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- tests/golden_vectors/generate.mjs
- crates/sorocharge-signer/src/verify.rs's existing test module

### chore: re-verify the pinned stellar-xdr/stellar-rpc-client versions still match Stellar's current live protocol version

`area:ci-release`

**Description.** CLAUDE.md §0 originally required checking stellar-xdr/stellar-rpc-client against the network's live protocol version via getLedger before any code was written. Real time has passed since; this is that check's periodic re-run, specifically for protocol-number alignment rather than just crate-version currency.

**Why.** Root Cargo.toml's exact pins (stellar-xdr = 28.0.0, stellar-rpc-client = 28.0.0) were correct at a point in time CLAUDE.md §0 doesn't name precisely — a live re-check confirms they still match testnet/pubnet's actual current protocol.

**Acceptance criteria.**
- [ ] A live getLedger call against both testnet and pubnet confirms the current protocol version
- [ ] The pinned stellar-xdr/stellar-rpc-client versions are confirmed to support that protocol version, or a bump is proposed
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- Root Cargo.toml's [workspace.dependencies]
- CLAUDE.md §0 is the exact original methodology to repeat

### chore: periodically re-validate verify_transfer_effects' assumed event shape against a live testnet transaction

`area:ci-release`

**Description.** verify_transfer_effects' own module doc states its transfer-event shape was 'verified against a real testnet transaction's event shape, not the CAP text, which the host doesn't always match exactly.' Nothing currently re-checks that this assumption still holds after a future protocol upgrade.

**Why.** crates/sorocharge-signer/src/effects.rs. An event-shape change in a future Soroban protocol version would silently break this function's core assumption, likely as a false-rejection (safe) or, worse, a false-acceptance.

**Acceptance criteria.**
- [ ] A scheduled (e.g. monthly) CI job runs a real transfer on testnet and diffs its event shape against what verify_transfer_effects expects
- [ ] A mismatch alerts rather than silently passing
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/effects.rs's module doc comment
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### docs(signer): document whether Signer implementations need to be constant-time

`area:signer`

**Description.** The Signer trait's doc comment describes its interface but says nothing about timing side-channels — relevant because this trait signs payment authorizations, a context where a timing leak in a custom Signer implementation could matter.

**Why.** crates/sorocharge-signer/src/sign.rs's Signer trait. Library consumers implementing their own Signer (e.g. an HSM-backed one) have no guidance on this from the docs today.

**Acceptance criteria.**
- [ ] The Signer trait's doc comment states whether sign_preimage needs to be constant-time and why (or explicitly states it's out of scope and the embedding application's responsibility)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/sign.rs
- ed25519-dalek's own documentation on this topic is the reference to check before writing guidance here

### docs: add a plain-language troubleshooting FAQ for verify_entry's error variants

`area:ci-release`

**Description.** sorocharge-signer alone has 18 distinct SorochargeError variants (by design, per CLAUDE.md's no-catch-all rule) — correct for the library's own discipline, but with no single plain-language reference for an integrator who just got PayerMismatch and needs to know what to actually check.

**Why.** crates/sorocharge-signer/src/error.rs's doc comments are accurate but written for a Rust API reader, not for triage.

**Acceptance criteria.**
- [ ] A docs-site page maps each error variant to "what this usually means" and "what to check first", in plain language
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs, crates/sorocharge-x402/src/error.rs, crates/sorocharge-mpp/src/error.rs
- docs/src/api/*.md already lists every variant — this issue adds the troubleshooting angle on top of the existing reference

### test(signer): add a regression test locking verify_entry's check order to its documented sequence

`area:signer`

**Description.** verify_entry's doc comment numbers its checks 1 through 8 in a specific order (expiry, allowance, shape, asset, payer, amount, recipient, signature). No test currently proves the code actually runs them in that order — a future refactor could silently reorder two checks and no test would catch it.

**Why.** crates/sorocharge-signer/src/verify.rs:134-167's documented order vs the implementation below it, which nothing currently cross-checks.

**Acceptance criteria.**
- [ ] A test constructs an entry violating two checks simultaneously (e.g. expired AND wrong amount) and asserts the error returned matches whichever check the doc comment says runs first
- [ ] Repeat for at least one more pair of checks to lock in the documented sequence
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/verify.rs:134-167 (the documented order) and :174-227 (the implementation)
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

## Medium — 150 points (40)

### chore: add a rust-toolchain.toml

`area:ci-release`

**Description.** CI pins Rust 1.98.1 explicitly in .github/workflows/ci.yml, but nothing pins it for local development — a contributor's `cargo build` uses whatever toolchain they happen to have installed.

**Why.** .github/workflows/ci.yml vs the repo root, which has no rust-toolchain.toml.

**Acceptance criteria.**
- [ ] rust-toolchain.toml at the repo root pins the same 1.98.1 version CI uses, with rustfmt and clippy as components
- [ ] A fresh `rustup show` in the repo root confirms it picks up the pin automatically
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- .github/workflows/ci.yml's existing toolchain pin is the value to mirror
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### chore: add eslint/prettier for tests/golden_vectors/generate.mjs

`area:ci-release`

**Description.** tests/golden_vectors/generate.mjs is committed JavaScript with zero linting or formatting enforcement, unlike the Rust side's cargo fmt/clippy discipline.

**Why.** tests/golden_vectors/. The script that proves sorocharge-signer's byte-for-byte correctness deserves the same basic hygiene as the code it verifies.

**Acceptance criteria.**
- [ ] eslint + prettier configs added under tests/golden_vectors/
- [ ] A CI step runs both and fails on a violation
- [ ] generate.mjs itself is reformatted to pass, with no behavior change

**Guidance.**
- tests/golden_vectors/package.json, generate.mjs
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### chore: add keywords and categories to each crate's Cargo.toml for crates.io discoverability

`area:ci-release`

**Description.** None of the three crates sets keywords or categories, which crates.io uses for search and category-browsing — once published, these crates will be essentially unsearchable by topic.

**Why.** crates/*/Cargo.toml's [package] sections, compared against crates.io's own publishing guidelines.

**Acceptance criteria.**
- [ ] Each crate's Cargo.toml sets relevant keywords (e.g. stellar, soroban, payments, x402) and at most 5 categories from crates.io's fixed category list
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/*/Cargo.toml
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### docs: add CI/license/docs badges to the README

`area:ci-release`

**Description.** README.md has zero badges — no CI status, no license, no crates.io version (once published) — despite the project having a real passing CI workflow and an Apache-2.0 LICENSE file to point to right now.

**Why.** README.md, .github/workflows/ci.yml, LICENSE. Approved repos in this ecosystem consistently lead with badges; this one currently has none.

**Acceptance criteria.**
- [ ] A CI status badge linking to the actual GitHub Actions workflow
- [ ] A license badge reflecting the real Apache-2.0 LICENSE file
- [ ] Badges render correctly on the GitHub repo page, not just in raw markdown

**Guidance.**
- README.md's top section, .github/workflows/ci.yml for the exact workflow name to badge
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### chore: add [profile.release] optimization settings to the workspace

`area:ci-release`

**Description.** Root Cargo.toml has no [profile.release] section — a facilitator/server embedding this library in production gets whatever Cargo's own release defaults are, with no deliberate choice made for a latency-sensitive signing/verification workload.

**Why.** Root Cargo.toml.

**Acceptance criteria.**
- [ ] A [profile.release] section with an explicit, justified choice (e.g. lto, codegen-units) backed by the benchmark numbers from the sibling criterion-benchmark issue
- [ ] A before/after benchmark comparison is included in the PR description
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- Root Cargo.toml
- Depends on the criterion benchmark issue existing first to measure the actual effect

### fix: SorochargeError derives Clone but X402Error/MppError don't — inconsistent error API across crates

`area:ci-release`

**Description.** crates/sorocharge-signer/src/error.rs's SorochargeError derives Clone (alongside Debug/PartialEq/Eq); X402Error and MppError do not derive it, an inconsistency in the error-type API shape across the three crates that otherwise follow an identical pattern.

**Why.** A caller wrapping SorochargeError inside X402Error::Signing or MppError::Signing can clone the inner error but not the outer one, an inconsistency with no documented reason.

**Acceptance criteria.**
- [ ] X402Error and MppError derive Clone (and PartialEq/Eq, matching SorochargeError's exact derive list) unless a specific field makes that impossible, in which case that's documented
- [ ] No existing test behavior changes
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/error.rs's derive list is the reference to match
- crates/sorocharge-x402/src/error.rs, crates/sorocharge-mpp/src/error.rs

### docs: add a runnable doctest example to build_charge_entry

`area:signer`

**Description.** build_charge_entry in crates/sorocharge-signer/src/entry.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how build_charge_entry is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] build_charge_entry's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/entry.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to ChargeParams

`area:signer`

**Description.** ChargeParams in crates/sorocharge-signer/src/entry.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how ChargeParams is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] ChargeParams's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/entry.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to CredentialKind

`area:signer`

**Description.** CredentialKind in crates/sorocharge-signer/src/entry.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how CredentialKind is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] CredentialKind's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/entry.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to UnsignedEntry

`area:signer`

**Description.** UnsignedEntry in crates/sorocharge-signer/src/entry.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how UnsignedEntry is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] UnsignedEntry's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/entry.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to sign_entry

`area:signer`

**Description.** sign_entry in crates/sorocharge-signer/src/sign.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how sign_entry is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] sign_entry's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/sign.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to Signer

`area:signer`

**Description.** Signer in crates/sorocharge-signer/src/sign.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Signer is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Signer's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/sign.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to SignedEntry

`area:signer`

**Description.** SignedEntry in crates/sorocharge-signer/src/sign.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how SignedEntry is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] SignedEntry's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/sign.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to verify_entry

`area:signer`

**Description.** verify_entry in crates/sorocharge-signer/src/verify.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how verify_entry is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] verify_entry's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/verify.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to verify_transfer_effects

`area:signer`

**Description.** verify_transfer_effects in crates/sorocharge-signer/src/effects.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how verify_transfer_effects is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] verify_transfer_effects's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/effects.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to X402Client::new

`area:x402`

**Description.** X402Client::new in crates/sorocharge-x402/src/client.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how X402Client::new is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] X402Client::new's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/client.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to X402Client::get_with_payment

`area:x402`

**Description.** X402Client::get_with_payment in crates/sorocharge-x402/src/client.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how X402Client::get_with_payment is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] X402Client::get_with_payment's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/client.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to Facilitator::new

`area:x402`

**Description.** Facilitator::new in crates/sorocharge-x402/src/facilitator.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Facilitator::new is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Facilitator::new's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to Facilitator::verify

`area:x402`

**Description.** Facilitator::verify in crates/sorocharge-x402/src/facilitator.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Facilitator::verify is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Facilitator::verify's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to Facilitator::settle

`area:x402`

**Description.** Facilitator::settle in crates/sorocharge-x402/src/facilitator.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Facilitator::settle is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Facilitator::settle's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to Facilitator::supported

`area:x402`

**Description.** Facilitator::supported in crates/sorocharge-x402/src/facilitator.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Facilitator::supported is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Facilitator::supported's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### docs: add a runnable doctest example to FacilitatorConfig

`area:x402`

**Description.** FacilitatorConfig in crates/sorocharge-x402/src/facilitator.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how FacilitatorConfig is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] FacilitatorConfig's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

### test(mpp): assert MppError::ChallengeExpired is returned for a challenge already past its expires timestamp

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::ChallengeExpired is returned for a challenge already past its expires timestamp. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a challenge already past its expires timestamp and asserts the result is exactly Err(MppError::ChallengeExpired) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::Signing is returned for sorocharge-signer failing to build or sign a charge entry

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::Signing is returned for sorocharge-signer failing to build or sign a charge entry. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs sorocharge-signer failing to build or sign a charge entry and asserts the result is exactly Err(MppError::Signing) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::XdrEncodingFailed is returned for a value that doesn't fit its XDR-constrained shape

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::XdrEncodingFailed is returned for a value that doesn't fit its XDR-constrained shape. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a value that doesn't fit its XDR-constrained shape and asserts the result is exactly Err(MppError::XdrEncodingFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::XdrDecodingFailed is returned for a submitted transaction's XDR failing to decode

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::XdrDecodingFailed is returned for a submitted transaction's XDR failing to decode. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a submitted transaction's XDR failing to decode and asserts the result is exactly Err(MppError::XdrDecodingFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::InvalidCredential is returned for a credential whose JSON doesn't match the expected shape

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::InvalidCredential is returned for a credential whose JSON doesn't match the expected shape. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a credential whose JSON doesn't match the expected shape and asserts the result is exactly Err(MppError::InvalidCredential) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::UnexpectedTransactionShape is returned for a transaction that doesn't authorize a single bare SEP-41 transfer

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::UnexpectedTransactionShape is returned for a transaction that doesn't authorize a single bare SEP-41 transfer. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a transaction that doesn't authorize a single bare SEP-41 transfer and asserts the result is exactly Err(MppError::UnexpectedTransactionShape) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::ForbiddenCredentialType is returned for a pull-mode credential type other than legacy sorobanCredentialsAddress

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::ForbiddenCredentialType is returned for a pull-mode credential type other than legacy sorobanCredentialsAddress. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a pull-mode credential type other than legacy sorobanCredentialsAddress and asserts the result is exactly Err(MppError::ForbiddenCredentialType) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::PushModeForbidsFeePayer is returned for a hash-type credential against a feePayer:true challenge

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::PushModeForbidsFeePayer is returned for a hash-type credential against a feePayer:true challenge. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a hash-type credential against a feePayer:true challenge and asserts the result is exactly Err(MppError::PushModeForbidsFeePayer) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::ServerAddressConflict is returned for the server's own address appearing where the safety checks forbid it

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::ServerAddressConflict is returned for the server's own address appearing where the safety checks forbid it. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs the server's own address appearing where the safety checks forbid it and asserts the result is exactly Err(MppError::ServerAddressConflict) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::RpcFailed is returned for a Stellar RPC call failing

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::RpcFailed is returned for a Stellar RPC call failing. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a Stellar RPC call failing and asserts the result is exactly Err(MppError::RpcFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::HashAlreadyConsumed is returned for a pushed transaction hash that's already been consumed

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::HashAlreadyConsumed is returned for a pushed transaction hash that's already been consumed. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a pushed transaction hash that's already been consumed and asserts the result is exactly Err(MppError::HashAlreadyConsumed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::VerificationFailed is returned for a credential failing one of the spec's verification checks

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::VerificationFailed is returned for a credential failing one of the spec's verification checks. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs a credential failing one of the spec's verification checks and asserts the result is exactly Err(MppError::VerificationFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### test(mpp): assert MppError::SettlementFailed is returned for verification succeeding but on-chain settlement failing

`area:mpp`

**Description.** No existing test in crates/sorocharge-mpp asserts MppError::SettlementFailed is returned for verification succeeding but on-chain settlement failing. All 21 of MppError's variants currently have zero direct unit-test coverage.

**Why.** Same CLAUDE.md error-distinctness guarantee as the other two crates' variants — unproven by any test today for this specific variant.

**Acceptance criteria.**
- [ ] A test constructs verification succeeding but on-chain settlement failing and asserts the result is exactly Err(MppError::SettlementFailed) (or the relevant field pattern)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/error.rs for the variant's exact fields
- crates/sorocharge-mpp/src/server.rs or client.rs/tx.rs for where it's returned
- Several of these (RpcFailed, ServerAddressConflict, HashAlreadyConsumed, SettlementFailed) depend on the mock-RPC testability issue elsewhere in this backlog landing first

### docs: consolidate the minimum Soroban protocol version this library assumes

`area:ci-release`

**Description.** entry.rs's CredentialKind::AddressV2 doc comment notes it's 'Mandatory from protocol 28 onward,' but no single document states the library's overall minimum assumed protocol version or what happens if it's used against an older network.

**Why.** crates/sorocharge-signer/src/entry.rs:51-54 is the only place this is mentioned, buried in one variant's doc comment rather than stated as a library-level fact.

**Acceptance criteria.**
- [ ] The README or docs site states the minimum Soroban protocol version this library assumes
- [ ] Explicitly states what happens (untested/unsupported, or a specific known failure) if used against an older network
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/entry.rs:47-59
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### docs: add a persistent-store reference example for MPP push-mode replay protection

`area:mpp`

**Description.** resource_server_mpp.rs's shipped example uses an in-memory Mutex<HashSet<String>> for consumed push-mode hashes. That's reasonable for a demo, but nothing in the example flags it as non-persistent/dev-only, and MppServer::settle_credential's docs explicitly push replay-protection ownership onto the caller.

**Why.** crates/sorocharge-mpp/examples/resource_server_mpp.rs:86-87. A real deployment that copies this example verbatim would lose its entire replay-protection state on every restart, silently.

**Acceptance criteria.**
- [ ] A comment in the example explicitly flags the in-memory store as demo-only and states what a production store needs (durability, concurrent-safe atomic check-and-mark)
- [ ] Linked from the mpp-server guide in the docs site
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/examples/resource_server_mpp.rs:80-90
- docs/src/guides/mpp-server.md already explains the caller-owns-this-store design; this issue is about the example code itself, not the explanation

### test(signer): add a statistical uniqueness sanity test for random_nonce

`area:signer`

**Description.** random_nonce's only property ever checked is that it doesn't error. No test confirms repeated calls actually produce distinct values, which is the entire property the nonce exists for (replay prevention).

**Why.** crates/sorocharge-signer/src/entry.rs:203-212.

**Acceptance criteria.**
- [ ] A test calls build_charge_entry many times with identical params and asserts no two produced nonces collide
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/entry.rs:203-212
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

### docs: add a versioning/compatibility philosophy section to the docs site

`area:ci-release`

**Description.** sorocharge-php pins sorocharge-core to an exact tagged release rather than a version range, and this repo's own dependencies are all exact-pinned — a deliberate, repeated choice that's never been written down anywhere as an explicit policy for downstream consumers to understand.

**Why.** docs/src/ has no page explaining this. A consumer deciding how to pin their own dependency on sorocharge-core has no stated guidance beyond observing the pattern in sorocharge-php's Cargo.toml.

**Acceptance criteria.**
- [ ] A docs-site page states the exact-pin philosophy, what semver guarantees (if any) apply between tagged releases, and what a consumer should pin to
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- docs/src/SUMMARY.md for where to slot the new page
- sorocharge-php's CLAUDE.md §3 (its own dependency table) is a concrete example of the pattern to document

### test(signer): add boundary tests for verify_entry's current_ledger parameter

`area:signer`

**Description.** Existing expiry tests use current_ledger values one or two below the fixture's expiration ledger. No test exercises current_ledger == 0 or current_ledger == u32::MAX, the actual type boundaries.

**Why.** crates/sorocharge-signer/src/verify.rs's existing expiry tests (rejects_an_expired_entry, accepts_a_valid_entry_signed_by_the_reference_sdk) only probe values immediately adjacent to the fixture's own expiration ledger.

**Acceptance criteria.**
- [ ] A test with current_ledger == 0 behaves correctly (entry not yet expired, assuming its expiration ledger is > 0)
- [ ] A test with current_ledger == u32::MAX correctly reports ExpiredEntry rather than any arithmetic overflow/panic
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-signer/src/verify.rs:174-197
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

## Trivial — 100 points (30)

### chore: add CODEOWNERS

`area:ci-release` `good first issue`

**Description.** No .github/CODEOWNERS file exists — nothing automatically requests review from a relevant maintainer on a PR touching a given crate.

**Why.** .github/ directory's existing contents (workflows only, no CODEOWNERS).

**Acceptance criteria.**
- [ ] .github/CODEOWNERS maps each crate directory to its maintainer(s)
- [ ] A test PR touching one crate confirms the right reviewer is auto-requested
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- .github/CODEOWNERS (new file)
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `.github/CODEOWNERS`

### chore: add a pull request template

`area:ci-release` `good first issue`

**Description.** No .github/PULL_REQUEST_TEMPLATE.md exists — every PR starts from a blank description.

**Why.** .github/ directory's existing contents.

**Acceptance criteria.**
- [ ] .github/PULL_REQUEST_TEMPLATE.md prompts for what changed, why, and how it was tested (matching CLAUDE.md §6's git-workflow discipline)
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- .github/PULL_REQUEST_TEMPLATE.md (new file)
- CLAUDE.md §6's git workflow section is the house style to reflect

**Entry point.** `.github/PULL_REQUEST_TEMPLATE.md`

### chore: add an issue template config

`area:ci-release` `good first issue`

**Description.** No .github/ISSUE_TEMPLATE directory exists — new issues get GitHub's bare default, with no structured prompt and no link to CONTRIBUTING.md (once core#5 lands) or this backlog's own issue format.

**Why.** .github/ directory's existing contents.

**Acceptance criteria.**
- [ ] .github/ISSUE_TEMPLATE/config.yml (and at least a basic bug-report template) added
- [ ] Links to CONTRIBUTING.md once it exists
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- .github/ISSUE_TEMPLATE/ (new directory)
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `.github/ISSUE_TEMPLATE/config.yml`

### chore: add an .editorconfig

`area:ci-release` `good first issue`

**Description.** No .editorconfig exists — indentation/line-ending consistency across Rust, JS (golden vectors), and Markdown files relies entirely on individual editor defaults.

**Why.** Repo root. rustfmt covers Rust formatting but not the JS/Markdown/TOML files alongside it.

**Acceptance criteria.**
- [ ] .editorconfig at the repo root covering at least indent style/size and final-newline for .rs, .js, .md, .toml
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- .editorconfig (new file)
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `.editorconfig`

### chore: add a rustfmt.toml pinning formatting style explicitly

`area:ci-release` `good first issue`

**Description.** No rustfmt.toml exists — `cargo fmt` relies entirely on rustfmt's own defaults, which can change between rustfmt versions even with the same Rust toolchain pinned.

**Why.** Repo root, compared against the explicit 1.98.1 toolchain pin in .github/workflows/ci.yml and the (proposed) rust-toolchain.toml.

**Acceptance criteria.**
- [ ] rustfmt.toml at the repo root makes the formatting style explicit (even if it just pins max_width and default stable options)
- [ ] cargo fmt --all -- --check still passes with no reformatting needed
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- Repo root, alongside the rust-toolchain.toml from the sibling issue
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `rustfmt.toml`

### docs: add a runnable doctest example to PaymentRequired

`area:x402` `good first issue`

**Description.** PaymentRequired in crates/sorocharge-x402/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how PaymentRequired is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] PaymentRequired's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-x402/src/types.rs`

### docs: add a runnable doctest example to PaymentRequirements

`area:x402` `good first issue`

**Description.** PaymentRequirements in crates/sorocharge-x402/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how PaymentRequirements is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] PaymentRequirements's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-x402/src/types.rs`

### docs: add a runnable doctest example to PaymentPayload

`area:x402` `good first issue`

**Description.** PaymentPayload in crates/sorocharge-x402/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how PaymentPayload is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] PaymentPayload's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-x402/src/types.rs`

### docs: add a runnable doctest example to VerifyRequest

`area:x402` `good first issue`

**Description.** VerifyRequest in crates/sorocharge-x402/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how VerifyRequest is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] VerifyRequest's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-x402/src/types.rs`

### docs: add a runnable doctest example to VerifyResponse

`area:x402` `good first issue`

**Description.** VerifyResponse in crates/sorocharge-x402/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how VerifyResponse is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] VerifyResponse's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-x402/src/types.rs`

### docs: add a runnable doctest example to SettlementResponse

`area:x402` `good first issue`

**Description.** SettlementResponse in crates/sorocharge-x402/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how SettlementResponse is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] SettlementResponse's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-x402/src/types.rs`

### docs: add a runnable doctest example to SupportedResponse

`area:x402` `good first issue`

**Description.** SupportedResponse in crates/sorocharge-x402/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how SupportedResponse is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] SupportedResponse's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-x402/src/types.rs`

### docs: add a runnable doctest example to MppClient::new

`area:mpp` `good first issue`

**Description.** MppClient::new in crates/sorocharge-mpp/src/client.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how MppClient::new is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] MppClient::new's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/client.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/client.rs`

### docs: add a runnable doctest example to MppClient::get_with_payment

`area:mpp` `good first issue`

**Description.** MppClient::get_with_payment in crates/sorocharge-mpp/src/client.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how MppClient::get_with_payment is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] MppClient::get_with_payment's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/client.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/client.rs`

### docs: add a runnable doctest example to MppServer::new

`area:mpp` `good first issue`

**Description.** MppServer::new in crates/sorocharge-mpp/src/server.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how MppServer::new is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] MppServer::new's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/server.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/server.rs`

### docs: add a runnable doctest example to MppServer::build_challenge_header

`area:mpp` `good first issue`

**Description.** MppServer::build_challenge_header in crates/sorocharge-mpp/src/server.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how MppServer::build_challenge_header is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] MppServer::build_challenge_header's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/server.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/server.rs`

### docs: add a runnable doctest example to MppServer::settle_credential

`area:mpp` `good first issue`

**Description.** MppServer::settle_credential in crates/sorocharge-mpp/src/server.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how MppServer::settle_credential is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] MppServer::settle_credential's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/server.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/server.rs`

### docs: add a runnable doctest example to MppServerConfig

`area:mpp` `good first issue`

**Description.** MppServerConfig in crates/sorocharge-mpp/src/server.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how MppServerConfig is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] MppServerConfig's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/server.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/server.rs`

### docs: add a runnable doctest example to ChargeRequest

`area:mpp` `good first issue`

**Description.** ChargeRequest in crates/sorocharge-mpp/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how ChargeRequest is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] ChargeRequest's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/types.rs`

### docs: add a runnable doctest example to Challenge

`area:mpp` `good first issue`

**Description.** Challenge in crates/sorocharge-mpp/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Challenge is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Challenge's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/types.rs`

### docs: add a runnable doctest example to Credential

`area:mpp` `good first issue`

**Description.** Credential in crates/sorocharge-mpp/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Credential is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Credential's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/types.rs`

### docs: add a runnable doctest example to Payload

`area:mpp` `good first issue`

**Description.** Payload in crates/sorocharge-mpp/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Payload is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Payload's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/types.rs`

### docs: add a runnable doctest example to Receipt

`area:mpp` `good first issue`

**Description.** Receipt in crates/sorocharge-mpp/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how Receipt is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] Receipt's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/types.rs`

### docs: add a runnable doctest example to MethodDetails

`area:mpp` `good first issue`

**Description.** MethodDetails in crates/sorocharge-mpp/src/types.rs has a doc comment describing it, but no `# Examples` section with a runnable code block — `cargo test --doc` currently exercises zero doctests anywhere in the workspace.

**Why.** A doc comment that only restates the signature doesn't show a caller how MethodDetails is actually used in context; a runnable example also becomes a tiny regression test for free.

**Acceptance criteria.**
- [ ] MethodDetails's doc comment gains a ```rust fenced example that compiles and runs under `cargo test --doc`
- [ ] The example uses realistic-looking values (a real-shaped address, not placeholder garbage) consistent with the rest of the crate's test fixtures
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/types.rs
- No other doctests exist yet in this workspace to copy from directly — the existing #[cfg(test)] unit tests in the same file are the best source for realistic values

**Entry point.** `crates/sorocharge-mpp/src/types.rs`

### test(x402): unit-test Facilitator::supported() directly

`area:x402` `good first issue`

**Description.** Facilitator::supported() is a pure function needing no RPC mock at all (it only reads the facilitator's own configured network/address), yet has zero direct test coverage, unlike its request/response types which do have round-trip tests.

**Why.** crates/sorocharge-x402/src/facilitator.rs:419-441. The easiest possible function in this file to test, and currently the only one with none at all.

**Acceptance criteria.**
- [ ] A test asserts supported() returns the exact scheme/network/signer address for a given FacilitatorConfig
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs:419-441
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `crates/sorocharge-x402/src/facilitator.rs`

### test(mpp): unit-test MppServer::build_challenge_header directly

`area:mpp` `good first issue`

**Description.** build_challenge_header's lower-level helpers (encode_base64url_jcs, build_www_authenticate_header) are tested in header.rs, but the MppServer method itself — the one actual callers call — has no direct test.

**Why.** crates/sorocharge-mpp/src/server.rs:109-123.

**Acceptance criteria.**
- [ ] A test calls MppServer::build_challenge_header with a real ChargeRequest and asserts the returned header value parses back to the same request via the existing header.rs parsing helpers
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/server.rs:109-123
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `crates/sorocharge-mpp/src/server.rs`

### chore: confirm the LICENSE file's copyright holder/year is accurate

`area:ci-release` `good first issue`

**Description.** LICENSE declares Apache-2.0; nothing in this audit confirmed the file's copyright notice (holder name, year) is actually correct and current rather than a boilerplate placeholder.

**Why.** LICENSE at the repo root.

**Acceptance criteria.**
- [ ] The LICENSE file's copyright notice is confirmed accurate (correct holder, correct year range) or corrected
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- LICENSE
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `LICENSE`

### chore: add a .gitattributes for consistent line endings

`area:ci-release` `good first issue`

**Description.** No .gitattributes exists — the mixed Rust/JS/Markdown/TOML file set has no enforced line-ending normalization for cross-platform contributors.

**Why.** Repo root.

**Acceptance criteria.**
- [ ] .gitattributes normalizes text file line endings (text=auto or equivalent) at the repo root
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- .gitattributes (new file)
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `.gitattributes`

### test(x402): assert FacilitatorConfig::default() matches the spec's documented defaults exactly

`area:x402` `good first issue`

**Description.** FacilitatorConfig::default() sets max_transaction_fee_stroops and inclusion_buffer_stroops from two named constants, but no test asserts the Default impl actually produces those exact documented values.

**Why.** crates/sorocharge-x402/src/facilitator.rs:49-56. A future edit to either constant without updating the other would go unnoticed without a test tying them together.

**Acceptance criteria.**
- [ ] A test asserts FacilitatorConfig::default().max_transaction_fee_stroops == 50_000 and inclusion_buffer_stroops == 100, by name matching the spec's documented values, not just by re-reading the constant
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-x402/src/facilitator.rs:29-56
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `crates/sorocharge-x402/src/facilitator.rs`

### test(mpp): assert MppServerConfig::default() matches this crate's documented conservative defaults exactly

`area:mpp` `good first issue`

**Description.** Same gap as x402's FacilitatorConfig: MppServerConfig::default()'s values are documented in a comment as this crate's own conservative choice, but no test asserts the Default impl actually produces them.

**Why.** crates/sorocharge-mpp/src/server.rs:34-55.

**Acceptance criteria.**
- [ ] A test asserts MppServerConfig::default().max_transaction_fee_stroops == 50_000 and inclusion_buffer_stroops == 100
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change
- [ ] cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt --all -- --check all still pass after the change

**Guidance.**
- crates/sorocharge-mpp/src/server.rs:34-55
- Run the existing CI checks locally before pushing — see .github/workflows/ci.yml for the exact commands

**Entry point.** `crates/sorocharge-mpp/src/server.rs`
