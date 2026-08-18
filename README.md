# Orca Whirlpool — Technical Audit Workspace

Root folder for the Orca deep-dive (oldest → newest programs). Prior findings copied under `reports/`. Full git histories of public Orca sources under `sources/`.

## Layout

| Path | Contents |
|------|----------|
| `reports/` | Prior inventory + Whirlpool/OR-H01 notes moved from `audit_work/findings/` |
| `sources/` | Full clones: `whirlpools`, `xorca`, `typescript-sdk`, `aquafarm-sdk` |
| `audits-external/` | Third-party PDF audits shipped in Orca repos |
| `notes/` | Commit flags, open-source scope map, **non-code technical audit** |

## Start here

1. `reports/ORCA_CONTRACT_INVENTORY.md` — all mainnet program IDs / authorities / timeline  
2. `notes/00_OPEN_SOURCE_SCOPE.md` — what we are reviewing (and what is closed-source)  
3. `notes/01_GIT_HISTORY_FLAGGED.md` — whirlpools history from first commit; flagged commits  
4. `notes/02_NON_CODE_TECHNICAL_AUDIT.md` — authorities, verify, license, config, ops (**this phase**)

## Audit phases

| Phase | Focus | Status |
|-------|--------|--------|
| Inventory | Addresses, authorities, deploys | Done |
| Non-code technical | Trust, upgrade, verify, license, config, docs claims | **In progress** |
| Code (oldest first) | V1 → V2 → Aquafarm → Whirlpool → Wavebreak → xORCA → Immutable | Pending |
