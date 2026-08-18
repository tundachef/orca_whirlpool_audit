# 04 — Orca Whirlpools (mutable) technical audit

**Program ID:** `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`  
**Source pin:** `e5f089bc5c49b01f5c8abb43c78457ab6c440568` (Osec-verified ↔ on-chain)  
**Checkout:** `orca-whirlpool/sources/whirlpools` @ detached `e5f089b`  
**Date:** 2026-08-18  
**Status:** **PHASE-COMPLETE**

---

## 1. Identity

| Check | Result |
|-------|--------|
| Osec verified | **true** — hash `52e75447…c5b0` @ commit `e5f089b` |
| Upgrade authority | `GwH3Hiv5…` (live; bare system key; also config `fee_authority`) |
| Config | `2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ` |
| Prior work | `reports/orca_whirlpools.md`, OR-H01 notes (`04_OR_H01_*`, `05_OR_H01_*`) |

---

## 2. Tests at pin

```
cargo test --lib  →  654 passed; 0 failed
```

Includes swap integration suites (splash + concentrated) and `fuzz_tests::test_calculate_transfer_fee_included_amount`.

---

## 3. Admin / authority surface

| Gate | Keys | Used for |
|------|------|----------|
| `ADMINS` / `is_admin_key` (mainnet) | `GwH3Hiv5…`, Eclipse `AqiJTdr9…` | **Only** `initialize_config`, `set_config_feature_flag` |
| Config `fee_authority` | currently `GwH3Hiv5…` | fee rates, fee authority transfer, many set_* |
| `collect_protocol_fees_authority` | `CRQd5wvb…` | collect protocol fees |
| `reward_emissions_super_authority` | `DXnB9N9J…` | reward super-auth |

**OW-M01 — Info/Design:** `set_fee_authority` takes `new_fee_authority: UncheckedAccount` — current fee auth can hand off to any pubkey (intentional; single-key compromise ⇒ permanent fee control until transferred again).

**OW-H01 — High (trust, non-code):** Same `GwH3…` is upgrade authority **and** fee_authority (privilege concentration).

---

## 4. Instruction inventory (high level)

Liquidity / swap: `swap`, `swap_v2`, `two_hop_swap(_v2)`, `increase/decrease_liquidity(_v2)`, `increase_liquidity_by_token_amounts_v2`, `reposition_liquidity_v2`, lock/transfer locked position, bundles, Token-2022 open/close, adaptive fee tier ixs, token badge ixs.

Pinocchio entry for hot liquidity paths under `src/pinocchio/instructions/`.

---

## 5. Findings (manual + explore @ e5f089b)

| ID | Sev | Title |
|----|-----|-------|
| **OW-H01** | **High** (trust) | `GwH3…` is upgrade authority **and** config `fee_authority` (privilege concentration) |
| **OW-H02** | **High** (code quality / future-compat) | **OR-H01 residual still present at `e5f089b`:** `DynamicTickArrayLoader::load_mut` / `load` unsafely cast to `[u8; MAX_LEN]` **without checking `data.len()`**. Call sites pass `data[8..]` (body only) while `MAX_LEN` **includes** the 8-byte discriminator — structural overclaim of 8 bytes. Prior runtime work **debunked Critical cross-account theft** under current Agave layout; remains High for UB / SIMD-0219 liveness. See `state/dynamic_tick_array.rs:119-129` and `state/tick_array.rs` load paths. |
| **OW-M01** | **Medium** (ops) | Default Cargo features omit `mainnet` → localnet `ADMINS` baked in if someone deploys without `--features mainnet` (footgun; live build OK via Osec) |
| **OW-M02** | **Medium** (ops/trust) | `set_fee_authority` / collect-protocol / reward-super: single-step rotate to `UncheckedAccount` — lockout or rug if key compromised/mis-set |
| **OW-M03** | **Medium** (trust) | TokenBadge whitelist enables TransferHook / PermanentDelegate / Pausable / freeze / MintCloseAuthority / DefaultAccountState — badge authority compromise ⇒ vault-hostile extensions |
| **OW-L01** | Low | `transfer_locked_position` uses `has_one = position` only (not PDA seeds); sole init is PDA today |
| **OW-L02** | Low | Pinocchio TransferHook CPI uses caller-supplied metas vs SPL `add_extra_accounts_for_execute_cpi` on Anchor path — residual divergence / DoS or hook-bypass risk if metas incomplete; not unbadged fund theft |
| **OW-I01** | Info | Slippage / zero-amount / exact-out partial-fill / vault binding on swap(_v2) look sound |
| **OW-I02** | Info | `increase_liquidity_by_token_amounts_v2` floors liquidity; token_max enforced; no donation path found |
| **OW-I03** | Info | Lock model: freeze-based; no decrease/close bypass while frozen |
| **OW-I04** | Info | `ADMINS` only gates `initialize_config` + `set_config_feature_flag` |
| **OW-I05** | Info | Token-2022: TransferFeeConfig always allowed; swap_v2 adjusts exact-in/out via `swap_with_transfer_fee_extension` |
| **OW-I06** | Info | NonTransferable pool mints always rejected; unknown TLV extensions rejected (`is_supported_token_mint`) |

---

## 6. Tests / fuzz

| Campaign | Result |
|----------|--------|
| `cargo test --lib` @ `e5f089b` | **654 passed, 0 failed** (incl. swap integration + transfer-fee fuzz_tests) |
| Dedicated honggfuzz `wp_math` | **1,313,356 iters / 179s / 0 crashes / 0 timeouts** — covers `compute_swap`, `get_amount_delta_{a,b}`, `get_next_sqrt_price`, `estimate_max_liquidity_from_token_amounts` (tick indices i32) |
| Log | `fuzz/whirlpools/logs/math_fuzz_180s.txt` |
| Harness | `fuzz/whirlpools/math_fuzz/` (honggfuzz 0.5.55, `whirlpool` cpi feature @ pin) |

---

## 7. Prior OR-H01 → **OW-H02**

| Claim | Verdict at `e5f089b` |
|-------|----------------------|
| Unsafe `load_mut` without len check | **Still present** (`state/dynamic_tick_array.rs:119-129`) |
| Call sites use body slice `data[8..]` | **Confirmed** — `state/tick_array.rs:123,167`, `initialize_dynamic_tick_array.rs:57` |
| `MAX_LEN` includes 8-byte discriminator | **Confirmed** (`DynamicTickArray::MAX_LEN` starts with `DISCRIMINATOR.len()`) |
| Critical neighbor-account smash / vault theft | **Remains DEBUNKED** per `04_OR_H01_RUNTIME_RESULT.md` + `05_OR_H01_THOROUGH_RESIDUAL_BATTERY.md` (Agave pad model + LiteSVM) |
| Residual severity | **High** code quality / future InvalidRealloc / SIMD-0219 readiness — **not** Critical fund theft on current evidence |

**Recommendation:** cast using actual `data.len()` or size the loader to `MAX_LEN - 8` for body slices; add an explicit `data.len() >= expected` check before the unsafe cast.

---

## 8. Token-2022 / swap_v2 deep review

### 8.1 Mint allowlist (`util/v2/token.rs::is_supported_token_mint`)

| Extension / property | Without badge | With badge |
|----------------------|---------------|------------|
| Legacy SPL Token mint | allowed | n/a |
| TransferFeeConfig | allowed | allowed |
| InterestBearing / Metadata / MetadataPointer / ScaledUiAmount | allowed | allowed |
| ConfidentialTransfer(+Fee) | allowed (non-confidential CPI only) | same |
| PermanentDelegate | **rejected** | allowed |
| TransferHook | **rejected** | allowed |
| MintCloseAuthority | **rejected** | allowed |
| DefaultAccountState | **rejected** | allowed (must be thawable if not Initialized) |
| Pausable | **rejected** | allowed |
| freeze_authority set | **rejected** | allowed |
| NonTransferable | **always rejected** | **always rejected** |
| Unknown TLV | **always rejected** | **always rejected** |
| Token-2022 native mint | **always rejected** | **always rejected** |

Pool init (`initialize_pool` v2 / adaptive fee) calls `verify_supported_token_mint` for both mints before vault creation.

### 8.2 Transfer-fee accounting on swap_v2

- Exact-in: program converts user-included amount → fee-excluded amount for AMM math, then re-inflates partial fills so vault receives the correct post-fee amount (`swap_with_transfer_fee_extension`).
- Exact-out: program inflates required output for mint transfer fee, then derives input accordingly.
- Slippage checks use fee-excluded output (exact-in) / fee-included input (exact-out).
- Vault transfers go through `transfer_checked` (+ TransferHook extras when present).

No Critical vault drain from fee-on-transfer mis-accounting found at this pin; unit `fuzz_tests` cover transfer-fee include/exclude helpers.

### 8.3 Critical unbadged Token-2022 vault theft?

**Verdict: NOT FOUND.** Dangerous extensions that can steal or freeze vault balances (PermanentDelegate, TransferHook, Pausable, freeze) require a live TokenBadge. That is an intentional trust boundary on the token-badge authority (documented as **OW-M03**), not an unprivileged user path.

### 8.4 OW-L02 residual (Pinocchio vs Anchor hooks)

- Anchor `transfer_from_*_v2` uses `spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi`.
- Pinocchio hot paths pass caller-provided remaining-account metas for hooks.
- Risk: incomplete metas → failed CPI (DoS) or divergent hook validation vs Anchor path. Does not create an unbadged PermanentDelegate-style drain.

---

## 9. Non-code carry-forward

- NC-02 GwH3 concentration (→ OW-H01)
- NC-05.1 Sec3 PDF gap vs Pinocchio / `e5f089b` surface
- Badge authority is a live trust assumption for any T22 pool with hostile extensions

---

## 10. Phase DoD checklist

| Item | Status |
|------|--------|
| Identity / Osec pin | done |
| Source map + admin surface | done |
| Manual review (swap, liquidity, locks, T22) | done |
| OR-H01 fold → OW-H02 | done |
| Unit tests @ pin | 654 pass |
| Dedicated math fuzz | 1.31M iters, 0 crashes |
| Report | this file |
| Git commit of artifacts | pending (this close-out) |

**Next in queue:** #5 Wavebreak (`waveQX2y…`) — closed-source ELF + public client math.
