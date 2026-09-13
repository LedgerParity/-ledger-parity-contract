# Read-only lifecycle monitor

Run with Node 22.12+:

```sh
npm ci --ignore-scripts
npm test
node monitor.mjs registry.testnet.json
```

The supplied registry records the synthetic 0.2.0 testnet deployment and report hash. The monitor checks the network passphrase, then reads the contract instance, Wasm code and each report entry. It emits JSON with live-until ledger, remaining ledger count and `live`, `renewal_due` or `missing_or_archived` status. It never signs or sends transactions. Missing ledger entries are not evidence that registration never happened.

Exit 0 means no maintenance finding, 3 means maintenance is needed, and 1 means invalid input or a failed RPC read. A failure cannot become a healthy empty result. At most 98 report hashes fit in one registry (two additional entries cover instance/code); split larger registries explicitly. The 259,200-ledger threshold matches half the contract's default target. On a network with a lower cap it can recommend renewal conservatively; monitoring does not override the contract's network-capped policy.

Keep an external record of the network, contract and Wasm hashes and reports you retain. Run the monitor under your own task scheduler, check nonzero exits, and submit renewal transactions when needed. No background process is installed by this tool. The operator of a deployment must own that schedule; this public testnet example is a validation artifact, not an operated service.

For a report, `restore_key_xdr` is the Bytes ScVal expected by Stellar CLI `contract restore --key-xdr`. It differs from the full ledger key in `key`. Restore the instance/code as needed too, then verify original data and renew. See [LIFECYCLE.md](../LIFECYCLE.md). Five offline tests cover XDR key identity, threshold/missing/expired states, malformed responses, network mismatch, HTTP failures and exact registered report bytes. The live monitor result is recorded separately in testnet evidence.
