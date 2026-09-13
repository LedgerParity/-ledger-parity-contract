# Contract roadmap

- [x] Owner authorization, immutable registration, bounded hash/metadata input, and regression tests.
- [x] Persistent per-report storage and bounded renewal of report, instance and code lifetimes.
- [x] Recovery procedure and explicit fresh-deployment boundary for the 0.2.0 storage layout.
- [x] Validate Wasm execution on Protocol 28 testnet; record signed registration, read-back, early renewal and provider fees using synthetic data.
- [x] Supply a read-only lifetime monitor and external registry of retained hashes.
- [ ] Validate actual archived restoration/read-back after entries expire; an early renewal is not that test.
- [ ] Establish a real operator's monitoring/renewal schedule and ownership.

CLI contract submission is deferred separately from the read-only reconciliation CLI and is not part of this preview.

A synthetic testnet deployment is documented in TESTNET.md. No operator-adoption or production-readiness claim. See LIFECYCLE.md and REVIEW.md for evidence limits.
