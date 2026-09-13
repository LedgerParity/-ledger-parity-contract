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

Instance storage capacity, TTL/archival/restoration, current-protocol compatibility, fees, a permitted testnet deployment and a signed registration/read-back have not been validated. First registration wins globally for a hash; this is not proof of report authorship. The existing API/storage layout was retained. No wallet/signing or CLI contract submission was added.

Authorization design reference: [Stellar authorization](https://developers.stellar.org/docs/learn/fundamentals/contract-development/authorization). SDK 21.7.7 source and test utilities were inspected locally.

## Verification

Local formatting and diff checks passed. Local `cargo test` was blocked before compiling the contract because the installed Windows MSVC toolchain has no `link.exe`; the Visual Studio Build Tools installation is incomplete. Do not describe this as a passing local Rust test run. Linux CI passed at bb7c010: formatting, all seven contract tests, and the locked release Wasm build. See [contract CI run 34784668593](https://github.com/LedgerParity/-ledger-parity-contract/actions/runs/34784668593). This is native SDK testing and Wasm compilation, not a deployed network test.

Related CLI commit 2633f88 passed both Go matrix jobs: [CI run 34782675975](https://github.com/LedgerParity/ledger-parity-cli/actions/runs/34782675975).
