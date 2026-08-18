# 04 — Orca Whirlpools (mutable) technical audit

**Program ID:** `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`  
**Source pin:** `e5f089bc5c49b01f5c8abb43c78457ab6c440568` (Osec-verified ↔ on-chain)  
**Checkout:** `orca-whirlpool/sources/whirlpools` @ detached `e5f089b`  
**Date:** 2026-08-18  
**Status:** IN PROGRESS

---

## 1. Identity

| Check | Result |
|-------|--------|
| Osec verified | **true** — hash `52e75447d1d49774ff6938484c9011e303860995497f0687a45febb3db21c5b0` |
| Upgrade authority | `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` (live; bare system key) |
| Config | `2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ` |
| Prior work | `reports/orca_whirlpools.md`, OR-H01 runtime notes |

## 2. Scope at pin

- `programs/whirlpool` — ~171 `.rs` files at this commit  
- Includes Pinocchio liquidity path + `increase_liquidity_by_token_amounts` (#1229)  
- Prior Sec3 PDF latest **2025-08** — **coverage gap** vs this pin (NC-05.1)

## 3. Plan

1. Instruction / authority surface map at `e5f089b`  
2. Manual review: swap, liquidity, tick arrays, fees, Token-2022, adaptive fee, position lock  
3. Fold OR-H01 into residual / confirmed table  
4. Run existing `proptest` / `fuzz_tests`; add cargo-fuzz for swap/liquidity math  
5. Write findings OW-*  

## 4. Non-code carry-forward (from earlier)

- NC-02: GwH3 concentration (upgrade + fee_authority)  
- NC-05.1: audit PDF gap vs Pinocchio surface  
