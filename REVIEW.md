# Contract review — 2026-09-13

Reviewed baseline db4d9ba. This is an implementation review, not an independent security audit.

## Findings and changes

- High: `store` accepted arbitrary owner addresses without authorization. Added `owner.require_auth()`; tests reject unauthenticated writes and assert the exact authorization invocation and arguments.
- Invalid test setup: generated clients were constructed without a registered contract address. Corrected registration and client creation throughout.
- Missing input bounds: `store` accepted arbitrary byte lengths for hashes and metadata. Enforced 32-byte hashes and a 1024-byte metadata cap; negative tests verify failed calls leave no entry.
- Duplicate integrity: tests now assert that a duplicate attempt cannot change the original owner, timestamp or metadata. Missing-record reads are checked too.
- Reproducibility: added Cargo.lock, a Linux test/Wasm build workflow, and generated-file ignores. The old host's open-ended ed25519-dalek requirement selected incompatible major 3 alongside SDK major 2; the lock resolves both to 2.2.0.
- Documentation incorrectly claimed the CLI called the contract and that registration proved reconciliation had occurred. Corrected those claims.

## Remaining deployment gates

The 0.2.0 follow-up below addresses persistent record storage and renewal logic. Current-protocol execution, fees, a testnet deployment, signed registration/read-back and network restoration remain unvalidated. First registration wins globally for a hash; this is not proof of report authorship. No wallet/signing or CLI contract submission was added.

Authorization design reference: [Stellar authorization](https://developers.stellar.org/docs/learn/fundamentals/contract-development/authorization). SDK 21.7.7 source and test utilities were inspected locally.

## Verification

Local formatting and diff checks passed. Local `cargo test` was blocked before compiling the contract because the installed Windows MSVC toolchain has no `link.exe`; the Visual Studio Build Tools installation is incomplete. Do not describe this as a passing local Rust test run. Linux CI passed at bb7c010: formatting, all seven contract tests, and the locked release Wasm build. See [contract CI run 34784668593](https://github.com/LedgerParity/-ledger-parity-contract/actions/runs/34784668593). This is native SDK testing and Wasm compilation, not a deployed network test.

Related CLI commit 2633f88 passed both Go matrix jobs: [CI run 34782675975](https://github.com/LedgerParity/ledger-parity-cli/actions/runs/34782675975).

## Persistent storage follow-up (0.2.0)

Implemented per-hash persistent entries, permissionless immutable `renew(hash)`, a 518,400-ledger target capped by network limits, and joint maintenance of contract instance/code lifetime. Reads do not renew entries. This changes the storage layout and requires a fresh deployment; there is no implicit migration from instance records. The recovery procedure and remaining deployment gates are in [LIFECYCLE.md](LIFECYCLE.md).

At 29c8a24, [Linux CI run 34785885294](https://github.com/LedgerParity/-ledger-parity-contract/actions/runs/34785885294) passed formatting, all 13 native SDK tests and the locked release Wasm build. Six lifecycle tests cover key isolation, immutable renewal, read/no-op TTL behavior, lower network limits, missing/invalid renewal, and archived access across five operations. The first run at 7276d00 passed 12 tests but exposed the SDK 21 archival host panic in the remaining test; the corrected test explicitly checks that archival failure with a fresh environment per operation. No runtime change was needed for that correction. Local execution is still limited by the missing MSVC linker; no network restoration or current-protocol Wasm execution is claimed.
