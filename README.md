# LedgerParity report hash contract

A development Soroban contract that registers a 32-byte report hash with an authorizing address, ledger timestamp and at most 1024 metadata bytes.

`store(hash, owner, metadata)` requires the owner's authorization for the invocation and rejects duplicate hashes. `verify(hash)` checks registration, `get_info(hash)` returns the stored record (or fails if absent), and `is_owner(hash, owner)` compares the registered address.

Version 0.2.0 stores each report in its own persistent entry. `renew(hash)` lets anyone pay to extend a registered report's lifetime without changing its owner, timestamp or metadata. Successful registration and renewal also maintain the instance and Wasm code lifetime. Reads do not renew entries. See [storage lifecycle](LIFECYCLE.md) for the policy, recovery procedure and test evidence.

This records an authorized assertion of a hash. It does not prove that reconciliation ran, that the report is correct, that the registrant created it, or that the metadata is true. The timestamp is registration time. The CLI's `--verify` and `--verify-check` are local checksum operations; the CLI does not invoke this contract or submit transactions.

## Checks

Use Rust with its platform linker and the Wasm target installed:

```sh
rustup target add wasm32-unknown-unknown
cargo fmt --check
cargo test --locked
cargo build --locked --release --target wasm32-unknown-unknown
```

The artifact is `target/wasm32-unknown-unknown/release/ledger_parity_verify.wasm`. CI runs these checks on Linux. Cargo.lock fixes the resolved SDK/dependency graph; this review does not migrate the existing SDK 21 dependency to the latest protocol.

## Deployment limits

No deployment or production readiness is claimed. Version 0.2.0 requires a fresh deployment: version 0.1.0 instance records are not migrated or read by this storage layout. Per-record renewal is implemented, but scheduling, deployed restoration, current-protocol compatibility and fees still need testnet validation. Native tests and Wasm compilation do not establish those results.

Registration is first-writer-per-hash. Any address can register a publicly known hash under itself; authorization prevents impersonating another address, but does not establish report authorship. Metadata is public on deployment; never include secrets or private operator exports. Deployment and signing are separate from the read-only CLI.

See [REVIEW.md](REVIEW.md) for findings and verification evidence. MIT licensed.
