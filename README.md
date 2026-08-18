# Orca — Technical Audit Workspace

Deep dive of Orca mainnet programs (oldest → newest). Artifacts live in this git repo under `orca-whirlpool/`.

## Start here

1. **`reports/08_ORCA_QUEUE_CLOSEOUT.md`** — executive board + fuzz summary (**queue complete**)  
2. **`reports/ORCA_CONTRACT_INVENTORY.md`** — program IDs / authorities / timeline  
3. **`notes/03_AUDIT_QUEUE.md`** — ordered DoD status  
4. Per-program: `reports/01_*` … `reports/07_*`

## Layout

| Path | Contents |
|------|----------|
| `reports/` | Inventory, per-program audits, OR-H01 notes, close-out |
| `notes/` | Scope, git flags, non-code audit, queue, residual battery |
| `fuzz/` | Honggfuzz harnesses, dumps, logs |
| `sources/` | Full clones (gitignored): whirlpools, xorca, SDKs, SPL lineage |
| `audits-external/` | Third-party PDFs |

## Audit phases

| Phase | Focus | Status |
|-------|--------|--------|
| Inventory | Addresses, authorities, deploys | **Done** |
| Non-code technical | Trust, upgrade, verify, license, config | **Done** |
| Code + fuzz (oldest→newest) | V1 → V2 → Aquafarm → Whirlpool → Wavebreak → xORCA → Immutable | **Done** |
| Close-out + residual battery | Executive board + bounded residual verification | **Done** |

## Highest residuals (not Critical drains)

- **OW-H02** — DynamicTickArray unsafe cast (Critical theft debunked; UB residual)
- **GwH3…** privilege concentration (mutable Whirlpool / Wavebreak / xORCA)
- **WB-H02** — Wavebreak graduation reward/fee vs quote (client-implied)
- **XO-M01/M02** — xORCA cool-down bounds / residual dust inflation
