# 08 — Orca ordered queue close-out

**Date:** 2026-08-18  
**Scope:** Seven mainnet Orca programs (oldest → newest)  
**Policy:** Analysis-grade; no mainnet exploit broadcast  
**Workspace tip:** see `git log` in `orca-whirlpool/`

---

## 1. Coverage

| # | Program | ID | Status | Report |
|---|---------|-----|--------|--------|
| 1 | Token Swap V1 | `DjVE6…` | COMPLETE | `01_TOKEN_SWAP_V1_AUDIT.md` |
| 2 | Token Swap V2 | `9W959…` | COMPLETE | `02_TOKEN_SWAP_V2_AUDIT.md` |
| 3 | Aquafarm | `82yxj…` | PHASE-COMPLETE | `03_AQUAFARM_AUDIT.md` |
| 4 | Whirlpools (mutable) | `whirLb…` @ `e5f089b` | PHASE-COMPLETE | `04_WHIRLPOOLS_AUDIT.md` |
| 5 | Wavebreak | `waveQX…` | PHASE-COMPLETE | `05_WAVEBREAK_AUDIT.md` |
| 6 | xORCA | `StaKE6…` | PHASE-COMPLETE | `06_XORCA_AUDIT.md` |
| 7 | Whirlpools Immutable | `iwhrLH…` | PHASE-COMPLETE | `07_WHIRLPOOLS_IMMUTABLE_AUDIT.md` |

---

## 2. Fuzz summary

| Target | Iters | Crashes | Notes |
|--------|------:|--------:|-------|
| V1 math (final clean) | ~210k+ / long runs | 0 (after FP fix) | also SPL ix fuzz ~398k |
| Aquafarm emissions | short campaigns | 0 | dump + SDK math |
| Whirlpools `wp_math` | **1,313,356** | **0** | swap/token math @ e5f089b |
| Wavebreak `wb_math` | **805,039** | **0** | `orca_wavebreak` 2.0.0 client |
| xORCA `xo_math` | **764,737** | **0** | after saturating-add FP fix |
| Immutable | n/a separate | — | math ≡ mutable pin |

---

## 3. Executive findings board (Orca-only)

### High

| ID | Program | Title | User funds? |
|----|---------|-------|-------------|
| **OW-H01** | Whirlpools mutable | `GwH3…` upgrade **and** fee_authority | Privilege / rug surface |
| **OW-H02** | Whirlpools (+ Immutable lineage) | `DynamicTickArrayLoader` unsafe cast w/o len check | **Not** Critical theft on current Agave evidence; UB / future-compat |
| **WB-H01** | Wavebreak | Same `GwH3…` upgrade + admin/LP takeover surface | Privilege |
| **WB-H02** | Wavebreak | Graduation rewards+fee can exceed quote (client) | Liveness / stuck graduate if init unbound |
| **XO-H01** | xORCA | `GwH3…` upgrade authority | Privilege |
| **OV2-I01** / Aquafarm auth | V2 / Aquafarm | Live legacy auth `23zF9…` | Privilege |

### Medium (selected)

| ID | Program | Title |
|----|---------|-------|
| **OW-M03** | Whirlpools | TokenBadge enables PermanentDelegate / TransferHook / Pausable |
| **WB-M01–M07** | Wavebreak | Weak curve validation, dust rounding, 100% fee, close underflow, end_price seed, permissions |
| **XO-M01–M04** | xORCA | Cool-down may be 0 / unbounded; residual dust inflation; `non_escrowed==0` 1:1 under-mint; single-step auth rotate |
| **WI-M01** | Immutable | Config authorities concentrated on `r21Gam…` (program frozen) |

### Critical unprivileged drains in this Orca pass

**None confirmed.** OR-H01 Critical vault smash remains **DEBUNKED**. Token-2022 unbadged vault theft **NOT FOUND**. Wavebreak/xORCA Critical paths require missing on-chain auth or admin compromise (not demonstrated).

---

## 4. Trust map (authorities)

| Key | Controls |
|-----|----------|
| `GwH3Hiv5…` | Upgrade: mutable Whirlpools, Wavebreak, xORCA; also Whirlpools fee_authority / ADMINS |
| `23zF9Azp…` | Upgrade: Token Swap V2, Aquafarm |
| `94kZD71s…` | xORCA initialize only |
| `r21Gamwd…` | Immutable Whirlpools config fee/collect/reward-super |
| *(none)* | V1 Loader2; Whirlpools Immutable upgrade auth **burned** |

---

## 5. Residual verification (this close-out addendum)

See **`notes/04_RESIDUAL_BATTERY.md`**.

| Residual | Result |
|----------|--------|
| Wavebreak graduation insolvency | Client **Err** when rewards &gt; quote; residual fuzz **557k / 0 crashes** |
| xORCA dust after donation | **Confirmed** zero-share / `InsufficientStakeAmount` for stake ≪ donation/100 |
| xORCA `non_escrowed==0` | **Confirmed** 1:1 under-mint vs virtual fair formula |
| OW-H02 | Unchanged High UB; Critical theft still debunked |

**Still no Critical unprivileged drain confirmed.**

---

## 6. Deliverables index

| Path | Role |
|------|------|
| `notes/03_AUDIT_QUEUE.md` | Queue status |
| `reports/01`–`07_*` | Per-program audits |
| `reports/04_OR_H01_*`, `05_OR_H01_*` | DynamicTickArray residual battery |
| `fuzz/*` | Harnesses + logs + dumps |
| `audits-external/` | Third-party PDFs (Whirlpool + xORCA Sec3) |
