# Error Variant Doc Comment Audits - Completed

## Overview
Audited four `SorochargeError` enum variants to confirm every call site genuinely matches that variant's own doc comment. All audits passed cleanly — no mismatches found.

## Audit Results

### #261: `SorochargeError::AmountMismatch`
**Doc:** "The entry's amount does not match `ChargeParams::amount` exactly."

**Return sites verified (all match):**
- `crates/sorocharge-signer/src/verify.rs:287` — `if transfer.amount != expected.amount`
- `crates/sorocharge-signer/src/entry.rs:980` — test assertion (off-by-one boundary)
- `crates/sorocharge-signer/src/verify.rs:636` — test `rejects_wrong_amount`
- `crates/sorocharge-signer/src/verify.rs:763` — test `reports_amount_before_recipient`

### #262: `SorochargeError::AmountNonPositive`
**Doc:** "The entry's amount is non-positive (zero or negative), which is semantically invalid for a SEP-41 transfer."

**Return sites verified (all match):**
- `crates/sorocharge-signer/src/entry.rs:390` — `if params.amount <= 0` in `build_charge_entry_with_nonce`
- `crates/sorocharge-signer/src/verify.rs:249` — top of `verify_entry`: `if expected.amount <= 0`
- `crates/sorocharge-signer/src/verify.rs:284` — `if transfer.amount <= 0 \|\| expected.amount <= 0`
- `crates/sorocharge-signer/src/verify.rs:653` — test: amount == 0 rejected
- `crates/sorocharge-signer/src/verify.rs:667` — test: amount < 0 rejected

### #259: `SorochargeError::PayerMismatch`
**Doc:** "The entry's authorizing address does not match `ChargeParams::payer`."

**Return sites verified (all match):**
- `crates/sorocharge-signer/src/verify.rs:277` — `if authorizing_address != expected.payer \|\| transfer.from != expected.payer`
- `crates/sorocharge-signer/src/verify.rs:622` — test `rejects_wrong_payer`

### #259: `SorochargeError::AssetMismatch`
**Doc:** "The entry's asset contract does not match `ChargeParams::asset_contract`."

**Return sites verified (all match):**
- `crates/sorocharge-signer/src/verify.rs:274` — `if transfer.asset_contract != expected.asset_contract`
- `crates/sorocharge-signer/src/verify.rs:606` — test `rejects_wrong_asset_contract`
- `crates/sorocharge-signer/src/verify.rs:739` — test `reports_asset_before_payer_amount_and_recipient`

## Summary
- **4 audits** completed
- **14 return sites** verified across all variants
- **0 mismatches** found
- **All acceptance criteria satisfied**: every call site genuinely matches its error variant's doc comment