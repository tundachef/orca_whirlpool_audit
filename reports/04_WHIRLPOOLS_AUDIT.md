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
| Osec verified | **true** — hash `52e75447…c5b0` @ commit `e5f089b` |
| Upgrade authority | `GwH3Hiv5…` (live; bare system key; also config `fee_authority`) |
| Config | `2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ` |
| Prior work | `reports/orca_whirlpools.md`, OR-H01 notes |

## 2. Tests at pin

```
cargo test --lib  →  654 passed; 0 failed
```

Includes swap integration suites (splash + concentrated) and `fuzz_tests::test_calculate_transfer_fee_included_amount`.

## 3. Admin / authority surface

| Gate | Keys | Used for |
|------|------|----------|
| `ADMINS` / `is_admin_key` (mainnet) | `GwH3Hiv5…`, Eclipse `AqiJTdr9…` | **Only** `initialize_config`, `set_config_feature_flag` |
| Config `fee_authority` | currently `GwH3Hiv5…` | fee rates, fee authority transfer, many set_* |
| `collect_protocol_fees_authority` | `CRQd5wvb…` | collect protocol fees |
| `reward_emissions_super_authority` | `DXnB9N9J…` | reward super-auth |

**OW-M01 — Info/Design:** `set_fee_authority` takes `new_fee_authority: UncheckedAccount` — current fee auth can hand off to any pubkey (intentional; single-key compromise ⇒ permanent fee control until transferred again).

**OW-H01 — High (trust, non-code):** Same `GwH3…` is upgrade authority **and** fee_authority (privilege concentration).

## 4. Instruction inventory (high level)

Liquidity / swap: `swap`, `swap_v2`, `two_hop_swap(_v2)`, `increase/decrease_liquidity(_v2)`, `increase_liquidity_by_token_amounts_v2`, `reposition_liquidity_v2`, lock/transfer locked position, bundles, Token-2022 open/close, adaptive fee tier ixs, token badge ixs.

Pinocchio entry for hot liquidity paths under `src/pinocchio/instructions/`.

## 5. Findings (manual + explore @ e5f089b)

| ID | Sev | Title |
|----|-----|-------|
| **OW-H01** | **High** (trust) | `GwH3…` is upgrade authority **and** config `fee_authority` (privilege concentration) |
| **OW-M01** | **Medium** (ops) | Default Cargo features omit `mainnet` → localnet `ADMINS` baked in if someone deploys without `--features mainnet` (footgun; live build OK via Osec) |
| **OW-M02** | **Medium** (ops/trust) | `set_fee_authority` / collect-protocol / reward-super: single-step rotate to `UncheckedAccount` — lockout or rug if key compromised/mis-set |
| **OW-M03** | **Medium** (trust) | TokenBadge whitelist enables TransferHook / PermanentDelegate mints — badge authority compromise ⇒ vault-hostile extensions |
| **OW-L01** | Low | `transfer_locked_position` uses `has_one = position` only (not PDA seeds); sole init is PDA today |
| **OW-L02** | Low | Pinocchio TransferHook CPI uses caller metas vs SPL `add_extra_accounts_for_execute_cpi` on Anchor path |
| **OW-I01** | Info | Slippage / zero-amount / exact-out partial-fill / vault binding on swap(_v2) look sound |
| **OW-I02** | Info | `increase_liquidity_by_token_amounts_v2` floors liquidity; token_max enforced; no donation path found |
| **OW-I03** | Info | Lock model: freeze-based; no decrease/close bypass while frozen |
| **OW-I04** | Info | `ADMINS` only gates `initialize_config` + `set_config_feature_flag` |

## 6. Tests / fuzz

| Campaign | Result |
|----------|--------|
| `cargo test --lib` @ `e5f089b` | **654 passed, 0 failed** (incl. swap integration + transfer-fee fuzz_tests) |
| Dedicated cargo-fuzz on swap_math | Pending (unit proptest already present) |

## 7. Prior OR-H01

Fold from `reports/orca_whirlpools.md` / `04_OR_H01_*` — treat as **prior residual** pending re-validation against this pin (not re-opened here without PoC replay).

## 8. Non-code carry-forward

- NC-02 GwH3 concentration; NC-05.1 Sec3 PDF gap vs Pinocchio/`e5f089b`  
