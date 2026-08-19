# Orca — Technical Audit Workspace

Deep dive of Orca mainnet programs (oldest → newest). Artifacts live in this git repo under `orca-whirlpool/`.

## Start here

1. **`reports/08_ORCA_QUEUE_CLOSEOUT.md`** — executive board + fuzz summary (**queue complete**)  
2. **`reports/ORCA_CONTRACT_INVENTORY.md`** — program IDs / authorities / timeline  
3. **`notes/03_AUDIT_QUEUE.md`** — ordered DoD status  
4. Per-program: `reports/01_*` … `reports/07_*`  
5. **DynamicTickArray deep dive:** `notes/07_…` through `notes/13_…` (endpoint: `notes/12_ENDPOINT_FINDING.md`)

## Layout

| Path | Contents |
|------|----------|
| `reports/` | Inventory, per-program audits, OR-H01 notes, close-out |
| `notes/` | Scope, git flags, non-code audit, queue, residual battery, DTA investigation |
| `fuzz/` | Honggfuzz harnesses, DTA canaries / differentials, logs |
| `sources/` | Full clones (gitignored): whirlpools, xorca, SDKs, SPL lineage |
| `audits-external/` | Third-party PDFs |

## Audit phases

| Phase | Focus | Status |
|-------|--------|--------|
| Inventory | Addresses, authorities, deploys | **Done** |
| Non-code technical | Trust, upgrade, verify, license, config | **Done** |
| Code + fuzz (oldest→newest) | V1 → V2 → Aquafarm → Whirlpool → Wavebreak → xORCA → Immutable | **Done** |
| Close-out + residual battery | Executive board + bounded residual verification | **Done** |
| DynamicTickArray exploitability | Source → host → mainnet FeatureSet LiteSVM | **Done** |

## DynamicTickArray conclusion (OW-H02 / OR-H01)

**Confirmed:** Whirlpools `DynamicTickArrayLoader` / Pinocchio twin construct a fixed-size `MAX_LEN` view over account data, discarding the real slice length. Production `update_tick` can `rotate_left/right(112)` past the account-data boundary — a real memory-safety / OOB primitive (measured on host; attributable under rotate-only).

**Not demonstrated:** persistent wrong tick state, cross-account smash, victim DoS, or economic fund loss. Under a LiteSVM FeatureSet reconstructed from the currently activated mainnet feature accounts, the full production-like transition (rotate → bitmap → serialize) at `N_at_rotate=260` leaves a final account body **identical** to a bounded reference. Every production rotate path repairs via serialize; no alternate INTERESTING/CRITICAL path was found.

**Read:** `notes/12_ENDPOINT_FINDING.md` and `notes/13_ALTERNATE_PATH_CLOSEOUT.md`.

## Highest residuals (not Critical drains)

- **OW-H02** — DynamicTickArray unsafe cast (**memory-safety confirmed; bounty-level impact not demonstrated**)
- **GwH3…** privilege concentration (mutable Whirlpool / Wavebreak / xORCA)
- **WB-H02** — Wavebreak graduation reward/fee vs quote (client-implied)
- **XO-M01/M02** — xORCA cool-down bounds / residual dust inflation

## Contact & contributions

- **Questions, corrections, or follow-ups:** open an issue on this repository.
- **Support this research:** send SOL to  
  `DXfD1croQbKfKxEDd58mNRBuC8mJRTKJrFMxLuBB71ib`
