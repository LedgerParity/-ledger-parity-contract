# Synthetic testnet validation

Contract 0.2.0 was deployed on Stellar testnet on 2026-09-13 using the verified CI Wasm from source ddb3eff. The runtime is the same as lifecycle-tested 29c8a24. Stellar CLI 28.0.0 was downloaded from its official GitHub release and SHA-256 checked before use. RPC reported Protocol 28 and the exact testnet network passphrase.

Contract: `CBSEWOCE3V7SZDYKAOIJDXI54KWPFIHU7RMRTB56PZBV5Y3N47BGNPV7`

Wasm SHA-256: `26c92f94678817fb97dc8e4f9e7d9858d4f39d28c1bae20420201804d1a483cd`

Report SHA-256: `712d1ce47ff80339a5ec37cee27d08945730ad1c5975e06b4233dee91445cbce`

The expected report is the deterministic offline CLI demo, not an operator export. The testnet owner is `GB72LF6W45GRQV7ODHG5D55PJ3OCVYXCNTUX7ZBQZ2VRRWQZT6Y5PMTL`; it was created for this run and funded with testnet Friendbot funds. No production account or real-value funds were used. Private configuration remains outside the repository and is not distributed.

## Observed results

| Action | Transaction | Horizon fee_charged (stroops) |
|---|---|---:|
| Wasm upload | `6d7e9b8eebf8c4a889fec7adb678be4f8fcff7c914adb14103296ad61233e13e` | 2456523 |
| Deployment | `641fb3fdc59e8da6256126151f14b28f88345d3adefc3430d506ba395bb814b3` | 19295 |
| Authorized registration | `5874ed5bb30349e16c9db930d8853b59b817fce9cb3622adb4b62ed6e10f3d1e` | 8143529 |
| Renewal invocation | `ad4dd635079ca9eb5cfdc217a5ba95a334e1257f781e9579919ed4d5004222ee` | 3627 |

All four transactions were successful. Fee values are the provider's `fee_charged` field, recorded without conversion or extrapolation to future fees. Read-back returned the expected owner, metadata bytes for `synthetic-demo`, and ledger timestamp 1789338322. The renewed entry remained identical. The monitor at ledger 4663055 observed the instance, code and report live through ledger 5181347, with 518292 ledgers remaining.

The renewal was deliberately sent as a transaction, but it was above the renewal threshold and therefore did not demonstrate an on-network TTL bump. Threshold extension and archived-access rejection are covered by native SDK tests. Real archived-state restoration was not executed: these fresh persistent entries have not expired, and the contract cannot shorten their lifetime. A successful early renewal or restoration no-op must not be presented as proof of archived restoration.

## Reproduction and remaining operations

The public `tools/registry.testnet.json` can be checked with the read-only monitor. `evidence/testnet.json` records the network, transactions and result; `evidence/report.json` is the synthetic source whose exact bytes were hashed. Testnet resets can remove this deployment, so later absence must not be treated as historical disproof.

For a future independent run, build the locked Wasm or download the checksum-verified CI artifact; generate a dedicated testnet identity with Stellar CLI; fund it through Friendbot; deploy with `--network testnet`; invoke `store` using the exact report SHA-256 and your public owner; read `get_info`; and submit `renew --send=yes` only when a transaction is intended. Keep key/config paths outside evidence. Record resulting transaction IDs and provider fees rather than copying the values above.

Production operation still needs an operator-owned monitoring schedule, archived restoration/read-back after actual expiry, and real operator reconciliation validation. The reconciliation CLI does not sign or invoke this contract.
