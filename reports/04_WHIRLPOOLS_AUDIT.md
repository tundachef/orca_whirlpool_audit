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

## 5. Fuzz / next

- [x] `cargo test --lib` at pin  
- [ ] Dedicated cargo-fuzz/honggfuzz on `swap_math` / token amounts (beyond unit proptest)  
- [ ] Fold prior OR-H01 into confirmed/rejected table  
- [ ] Deep review Token-2022 transfer-fee + remaining accounts on `swap_v2`  

## 6. Non-code carry-forward

- NC-02 GwH3 concentration; NC-05.1 Sec3 PDF gap vs Pinocchio/`e5f089b`  
