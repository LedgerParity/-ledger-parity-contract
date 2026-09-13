# ledger-parity-contract

Soroban smart contract for tamper-proof reconciliation report verification.
Part of the [LedgerParity](https://github.com/LedgerParity) ecosystem.

## What it does

Stores report hashes on-chain so operators can prove a reconciliation was
performed at a specific time with specific results. The CLI calls this contract
after generating a report.

## Functions

| Function | Description |
|---|---|
| `store(hash, owner, metadata)` | Store a report hash with owner address |
| `verify(hash)` | Check if a report hash exists on-chain |
| `get_info(hash)` | Retrieve owner, timestamp, and metadata |
| `is_owner(hash, owner)` | Check if a hash was stored by a specific owner |

## Deploy

Requires [Stellar CLI](https://developers.stellar.org/docs/smart-contracts/getting-started/setup):

```sh
# Build
soroban contract build

# Deploy to testnet
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ledger_parity_verify.wasm \
  --network testnet
```

## Usage from CLI

```sh
# Generate report and store proof
ledger-parity --config config.json --format json --out report.json --verify auto

# Verify a report against saved proof
ledger-parity --verify-check report.json.proof.json report.json
```

## Tests

```sh
soroban contract test
```

## License

MIT
