# 05 — Orca Wavebreak technical audit

**Program ID:** `waveQX2yP3H1pVU8djGvEHmYg8uamQ84AuyGtpsrXTF`  
**ProgramData:** `nEuknUvGZK5UVyq3Tf18tcpNtC7dmjPMirrry66SkAs`  
**Upgrade authority:** `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` (same live key as Whirlpools)  
**Last deployed slot:** `365746180`  
**ELF:** `fuzz/wavebreak/wavebreak.so` (500 448 bytes)  
**SHA256:** `8cd60d772f1d8a97a6f879c545e2827914c1d285c0edca22148ac231f21e9742`  
**Client:** `orca_wavebreak` / `@orca-so/wavebreak` **v2.0.0** (crates.io + extract under `fuzz/wavebreak/sdk_extract/`)  
**Date:** 2026-08-18  
**Status:** **PHASE-COMPLETE** (client-math + ix-surface + fuzz; on-chain ELF closed-source)

---

## 1. Identity & residual

| Check | Result |
|-------|--------|
| Program ID matches client `WAVEBREAK_ID` | **yes** |
| Dump hash recorded | **yes** (MANIFEST) |
| Public program source | **no** — closed-source |
| Client math publicly available | **yes** — `orca_wavebreak` 2.0.0 |
| ELF ≡ client math proven | **no** — **WB-RESIDUAL** |

**WB-RESIDUAL (Info):** All math/authority findings below are *client-implied*. On-chain may add stricter init bounds, diverge in rounding, or gate paths differently. Treat equivalence as unverified until a verified build or binary diff exists.

---

## 2. Architecture (from client)

### Bonding curve
- 64-bin Bezier in sqrt-price / BPS space (`NUM_BINS=64`).
- Fixed quote per bin toward `graduation_target`; base per bin unfixed (rounding).
- Design: buy “mints less”, sell “collects more”.
- Sqrt bounds ≈ Whirlpool ticks ±300 000 (`MIN_SQRT_PRICE` / `MAX_SQRT_PRICE`).
- Builders: `flat` / `linear` / `exponential` / `sigmoid`.

### Trading (disc 8–11)
| Ix | Quote helper |
|----|--------------|
| `token_buy_exact_in` | `exact_in_buy_quote` |
| `token_buy_exact_out` | `exact_out_buy_quote` |
| `token_sell_exact_in` | `exact_in_sell_quote` |
| `token_sell_exact_out` | `exact_out_sell_quote` |

Buy: fee on quote in → curve on post-fee. Sell: curve first → fee on quote out. Caps via `max_buy_amount` / `max_sell_amount`. Optional permission bitmaps on buy/sell.

### Graduation & LP
1. `bonding_curve_graduate` — skim creator/signer/protocol rewards from quote vault.
2. `graduate_whirlpool` — seed locked Whirlpool position via CPI (large account list incl. badges, tick arrays, lock_config).
3. `graduate_manual` — alternate destination path.
4. `bonding_curve_close` / `close_quote` — mint residual/protocol base to hit intended supply.
5. LP escrow: `lp_harvest`, `lp_transfer`, `lp_takeover` (Whirlpool locked position move).

### Authority model
- `AuthorityConfig` PDA `["authority_config"]` — up to **64** `ProgramAuthority` entries (instruction bitmaps).
- `authority_config_{initialize,grant,revoke}` — grant requires current `authority` signer.
- `lp_takeover`: signer `authority` + `authority_config` (client implies bitmap-gated).
- Fee collect: any signer + matching `fee_authority` ATA + `authority_config` (exact gate in ELF).
- Permission layer: ECDSA-shaped consume (top-level / CPI) with per-signature `consumed_permission` PDA.

---

## 3. Findings

| ID | Sev | Title |
|----|-----|-------|
| **WB-RESIDUAL** | Info | Closed-source ELF; client math/guards not proven identical |
| **WB-H01** | **High** (trust) | Upgrade authority `GwH3…` + powerful admin surface (`authority_config_grant`, `mint_config_update`, `lp_takeover`, fee collect, graduation) |
| **WB-H02** | **High** (liveness / config) | `quote_graduation_amount` fails if `creator_reward + graduation_reward + protocol_fee > quote` — graduation can stick without init invariant (not visible as enforced in client alone) |
| **WB-M01** | Medium | `PriceCurveFacade::is_valid` does not require x/y monotonicity — pathological Bezier bins possible if init accepts arbitrary control points |
| **WB-M02** | Medium | Adverse rounding / zero-base dust buys are intentional; round-trips lose value; dust reconciled at close |
| **WB-M03** | Medium | `fee_from_post_fee_amount(…, 10000)` returns `u64::MAX`; 100% fee configs brick or skim all |
| **WB-M04** | Medium | `total_supply_round_up` ÷0 if `base_allocation_bps==0` or `base_protocol_fee_bps==10000` |
| **WB-M05** | Medium | `close_quote` can underflow if minted base exceeds intended residual (esp. early graduation) |
| **WB-M06** | Medium | Whirlpool seed uses `end_price`, not clearing price — arb / IL vs locked LP if early graduate |
| **WB-M07** | Medium | Permission anti-bot layer is centralized trust (signer + consume/replay); dual top-level/CPI paths |
| **WB-L01** | Low | Client `unreachable!` in max-buy/max-sell requote paths (`quote.rs`) |
| **WB-L02** | Low | Slippage helper edge cases with tiny amounts / large BPS |
| **WB-I01** | Info | Instruction metas available in crates.io client (not in thin `sdk_extract/instructions/`) |
| **WB-I02** | Info | `BondingCurve` carries buy/sell permission bitmaps, graduation methods[8], premint, retain_mint_authority |

**No Critical unprivileged drain confirmed** from client alone (would require missing auth in ELF — not demonstrated).

---

## 4. Instruction surface (30 ixs)

Permission (0–6), token buy/sell/refund (8–12), authority (16–18), mint config (24–26), graduate adapters (32–33), create launch variants (40–42), bonding curve lifecycle (48–51), LP (56–58). See `fuzz/wavebreak/sdk_extract/discriminators.txt`.

Notable account lists (crates.io generated):
- **`lp_takeover`:** `[writable,signer] authority`, `authority_config`, lp escrow pair, Whirlpool position + lock_config, token22/ata/whirlpool programs.
- **`graduate_whirlpool`:** signer + bonding curve vaults + full Whirlpool init/liquidity accounts (config, fee tier, pool, oracle, position, ticks, token badges, lock_config).
- **`bonding_curve_graduate`:** signer + creator + fee_authority + vault ATAs + `authority_config`.

---

## 5. Fuzz

| Campaign | Result |
|----------|--------|
| Harness | `fuzz/wavebreak/math_fuzz/` vs `orca_wavebreak = "2.0.0"` |
| Coverage | buy/sell exact in/out quotes, price convert, curve builders (+ monotone x assert), `graduate_quote` |
| `HFUZZ_RUN_ARGS='--run_time 120'` | **805,039 iters / 121s / 0 crashes / 0 timeouts** (117 new units) |
| Prior warm-up | ~180s corpus build (429 units); no crash artifacts |
| Log | `fuzz/wavebreak/logs/math_fuzz_120s.txt` |

Invariants asserted when `Ok`:
- Buy: `fee_amount ≤ amount_in`
- Sell: `fee_amount ≤ amount_out + fee_amount`
- Builder curves: Bezier `x` non-decreasing
- No panics / `unreachable` under clamped inputs

---

## 6. Recommendations

1. Verify on-chain init enforces: rewards+max protocol fee &lt; graduation_target; BPS ≠ 10000 / allocation ≠ 0; control-point monotonicity (or force mint-config defaults only).
2. Confirm `lp_takeover` / fee collect require bitmap grant matching disc — and that `GwH3…` is multisig/HSM in ops.
3. Revisit Whirlpool seed price for early graduation (clearing vs `end_price`).
4. Replace client `unreachable!` with `Err` if mirrored on-chain.
5. When possible: open-source or publish verified build ↔ ELF hash.

---

## 7. Phase DoD

| Item | Status |
|------|--------|
| Dump + MANIFEST identity | done |
| Client math / PDA / ix surface map | done |
| Manual findings (WB-*) | done |
| Dedicated math fuzz | **805k iters, 0 crashes** |
| Report | this file |
| Git commit | this close-out |

**Next:** #6 xORCA (`StaKE6X…`) — full public source under `sources/xorca`.
