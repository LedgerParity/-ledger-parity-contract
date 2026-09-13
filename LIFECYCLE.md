# Storage lifecycle (0.2.0)

Each report is a persistent entry keyed by the raw 32 hash bytes (Soroban `Bytes` / XDR `ScVal::Bytes`), with the unchanged `ReportInfo` value. Entries no longer accumulate in the contract instance. The first authorized registration remains immutable; no delete or overwrite operation exists.

This is a fresh-deployment layout, not an in-place upgrade of 0.1.0. The old instance key space is distinct and is not consulted. Existing deployments would need a separately designed, verified migration. Retain any old contract ID alongside its reports.

## Renewal policy

Successful `store` and `renew` request a TTL of 518,400 ledgers, capped by the runtime's maximum TTL. The threshold is half that effective target, using integer division. The host extends entries when their remaining TTL reaches its threshold; entries above it are unchanged. At a five-second ledger cadence the target is approximately 30 days, but ledger counts and network caps are authoritative.

The selected report, instance and Wasm code are evaluated independently. Renewing one report does not renew another report. Existing longer lifetimes are never shortened. `verify`, `get_info` and `is_owner` remain reads and do not renew anything.

`renew(hash)` requires no owner authorization: its caller can only pay transaction/rent costs to preserve existing data. It cannot change attribution or metadata. It returns false for an absent hash, rejects malformed hash lengths, and does not create entries. A true result means an existing record was processed; an early renewal can be a TTL no-op.

## Operator procedure

Maintain an off-chain list of contract ID, network, Wasm hash and report hashes; the contract does not enumerate records. Monitor each record's ledger lifetime as well as instance/code lifetimes, and submit renewals below the threshold with enough margin for transaction retries. Simulation alone does not commit TTL changes. There is no scheduler in this repository.

If data has archived, do not treat a failed read as proof that the report never existed or attempt to replace its registration. Restore the required entry at transaction level, then read the original owner/timestamp/metadata and renew it. If the instance or code archived too, those entries also need restoration before invocation.

For an explicit report-entry restore, use the deployed contract ID and persistent durability with the **base64 XDR encoding of the raw hash's Bytes ScVal** as `--key-xdr`. Do not pass the hash as a Symbol, UTF-8 hex bytes, or a complete ledger-key XDR. The [official CLI restore guide](https://developers.stellar.org/docs/tools/cli/cookbook/restore-contract-storage) describes the transaction options. The fee-paying source must be configured separately; no credentials belong in report metadata.

Modern networks may restore entries through simulation-prepared transaction restore lists. SDK 21 native tests model archived access as an error; they do not exercise that newer network mechanism. Follow the [state archival documentation](https://developers.stellar.org/docs/learn/fundamentals/contract-development/storage/state-archival) for the target protocol. The [SDK TTL test guide](https://developers.stellar.org/docs/build/guides/archival/test-ttl-extension) explains the ledger-based test approach.

## Acceptance and remaining work

Six lifecycle tests cover separate storage/initial TTLs, immutable permissionless renewal and key isolation, read/early-renewal behavior, a lower network TTL cap, invalid/missing renewals, and archived-record reads/overwrite rejection. The original seven authorization and registration tests remain.

All 13 tests and the release Wasm build passed at 29c8a24 in [Linux CI run 34785885294](https://github.com/LedgerParity/-ledger-parity-contract/actions/runs/34785885294). Archived-access tests assert the SDK 21 host panic's archival diagnostic for each operation; the host fails before a missing-record result or overwrite can succeed.

Still required before deployment claims: current-protocol Wasm execution, signed testnet registration/read-back, real fee measurements, instance/code and record restoration/read-back, and an operator-run monitoring process. Contract renewal logic and a recovery runbook are implemented; network restoration has not been executed. Local Rust tests remain unavailable because the Windows MSVC linker is missing; Linux CI is the execution gate.
