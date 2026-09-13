# LedgerParity report hash contract

A development Soroban contract that registers a 32-byte report hash with an authorizing address, ledger timestamp and at most 1024 metadata bytes.

`store(hash, owner, metadata)` requires the owner's authorization for the invocation and rejects duplicate hashes. `verify(hash)` checks registration, `get_info(hash)` returns the stored record (or fails if absent), and `is_owner(hash, owner)` compares the registered address.

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

No deployment or production readiness is claimed. This preview retains instance storage: all entries share instance capacity and lifetime. There is no TTL extension or restoration workflow, and an unlimited registry will exceed capacity. Design persistent per-record storage and lifecycle management before sustained use; that would change storage layout and require a fresh deployment or migration.

Registration is first-writer-per-hash. Any address can register a publicly known hash under itself; authorization prevents impersonating another address, but does not establish report authorship. Metadata is public on deployment; never include secrets or private operator exports. Deployment and signing are separate from the read-only CLI.

See [REVIEW.md](REVIEW.md) for findings and verification evidence. MIT licensed.
