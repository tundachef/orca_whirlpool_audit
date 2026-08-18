# 07 — Orca Whirlpools Immutable technical audit

**Program ID:** `iwhrLHdsgrvmnwU8GF2FSmyabSMjfHwFGJAX2ufJ3ZN`  
**ProgramData:** `7j2FgCQqJKs89k8w8dFdXrkeu8TT1EPJwkz5zuRUj1uo`  
**Upgrade authority:** **none** (burned) — confirmed via `solana program show`  
**Last deployed slot:** `420510431`  
**ELF:** `fuzz/whirlpools-immutable/immutable.so` (1 538 016 bytes)  
**SHA256:** `20e890cefe5d1e3a6a0bd2a9fded3d405c511e13a5d1af3d4b59d78438f70230`  
**Config:** `8pm8erUsaMpmZ47LttHAPgnDx7xGZUvxY4q47vTCs5Nj`  
**Source lineage:** `origin/immutable/78a6ff3` ≈ mutable Osec pin `e5f089b` + ID/admin deltas  
**Date:** 2026-08-18  
**Status:** **PHASE-COMPLETE** (final program in ordered queue)

---

## 1. Identity & trust posture

| Check | Result |
|-------|--------|
| Loader | BPF Upgradeable |
| Authority | **`none`** — cannot be upgraded |
| vs mutable | Mutable `whirLb…` still upgradeable by `GwH3…` |
| Config isolation | Separate config PDA owned by immutable program |
| Config fee / collect / reward-super | all `r21Gamwd9DtyjHeGywsneoQYR39C1VDwrw7tWxHAwh6` |
| Default protocol fee rate | 1300 |

**WI-I01 (Info / positive):** Burning upgrade authority is the intended trust reduction vs mutable Whirlpools (**OW-H01** upgrade half does **not** apply here).

---

## 2. Source diff vs mutable pin `e5f089b`

Branch `immutable/78a6ff3` vs `e5f089b` under `programs/whirlpool/`:

| Change | Detail |
|--------|--------|
| `declare_id!` / Pinocchio program id | `whirLb…` → `iwhrLH…` |
| `ADMINS` (mainnet) | Dropped Eclipse admin `AqiJTdr9…`; remaining `GwH3…` only (for `initialize_config` / feature flags) |
| Version | 0.8.0 → 0.9.0 + changelog pinocchio note |
| Tests / sparse_swap fixtures | Program id string updates |

**No math, swap, tick-array, or Token-2022 logic divergence** in this diff. Therefore findings **OW-H02** (DynamicTickArrayLoader UB), Token-2022 matrix, and math-fuzz conclusions from `04_WHIRLPOOLS_AUDIT.md` **carry over** to this frozen deployment.

Temporary admin history (flagged commits):
- `895c6df` (2026-03-24): add temp admin to init immutable
- `58194bd` (2026-05-18): drop temp + Eclipse admin (aligns with burn window)

---

## 3. Findings (Immutable-specific)

| ID | Sev | Title |
|----|-----|-------|
| **WI-I01** | Info (+) | Upgrade authority burned — bytecode frozen |
| **WI-M01** | Medium (ops) | Config authorities concentrated on single key `r21Gam…` (fee + collect-protocol + reward-super). Not upgradeable, but fee/rate/badge ops still live |
| **WI-M02** | Medium (carry) | **OW-H02** residual present in lineage — unsafe `DynamicTickArrayLoader::{load,load_mut}` without len check; Critical theft still debunked for current Agave |
| **WI-M03** | Medium (carry) | TokenBadge / fee_authority trust (**OW-M03** / **OW-M02**) still applies via config key |
| **WI-L01** | Low (carry) | Pinocchio TransferHook meta divergence (**OW-L02**) if this build includes Pinocchio liq paths (lineage yes) |
| **WI-I02** | Info | `ADMINS` still lists `GwH3…` for config init / feature flags — relevant only if a *new* config were initialized; primary config already exists |

**Critical unprivileged drain unique to Immutable:** **NOT FOUND** (inherits mutable verdict).

---

## 4. Fuzz / tests

| Campaign | Result |
|----------|--------|
| Mutable math fuzz @ `e5f089b` | **1.31M iters / 0 crashes** — applies to shared math |
| Re-fuzz Immutable | **Not separately required** — source math identical; ELF is frozen redeploy of same lineage |
| Unit tests | Same suite as mutable pin (654 pass @ e5f089b) |

---

## 5. Residual risks

1. Live config key `r21Gam…` can still change fees / badges / reward super — monitor custody.
2. OW-H02 remains in frozen code — cannot patch without a new program ID.
3. Mutable Whirlpools (`whirLb…`) remains the upgradeable twin — users/TVL split should be tracked separately.

---

## 6. Phase DoD / queue close

| Item | Status |
|------|--------|
| On-chain identity + burned auth | done |
| ELF dump + hash | done |
| Source diff vs mutable pin | done |
| Carry-forward OW findings | done |
| Report | this file |

**Ordered Orca queue (#1–#7): COMPLETE.**
