# 01 — Orca Token Swap V1 technical audit

**Program ID:** `DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1`  
**Status:** Phase 1 in progress (identity closed; lineage mapped; fuzz running)  
**Date:** 2026-08-18

---

## 1. Identity (G1) — CLOSED

| Check | Result |
|-------|--------|
| Loader | `BPFLoader2` — **immutable** (no upgrade authority) |
| On-chain data length | 310 464 |
| Local dump | `audit_work/dumps/orca_v1.so` |
| Fresh `solana program dump` | **byte-identical** sha256 `4f316b5d1eb435105dc8166e77e9dc44569e9a30ed0d8ec8636f6a14e74b2739` |
| Build fingerprint (strings) | `solana-program-1.5.4`, rust-bpf-sysroot v0.13, builder path `/home/rawfalafel/...` |
| Curves in ELF | `ConstantProductCurve`, `ConstantPriceCurve`, `StableCurve` (+`amp`), `OffsetCurve` (+`token_b_offset`) |
| Ix log strings | `Swap`, `DepositAllTokenTypes`, `WithdrawAllTokenTypes`, `DepositSingleTokenTypeExactAmountIn`, `WithdrawSingleTokenTypeExactAmountOut`, plus `InitOffsetCurve` string |

**Verdict:** Dump is the live program. No upgrade residual.

---

## 2. Source map

| Artifact | Role |
|----------|------|
| **No Orca-published V1 program repo** | Confirmed (inventory / org scan) |
| Closest public lineage | SPL Token-Swap **`token-swap-v2.0.0`** (`SwaPpA9…` declare_id) — has Stable + Offset; solana-program ~1.5.x |
| Fetched under | `orca-whirlpool/sources/spl-token-swap-v2/token-swap/program/src/` |
| Modern SPL in `audit_work/sources/spl-lib/token-swap` (v3.0.0) | **Not byte-compatible** (no Stable in tree; Token-2022; different program id `SwapsVeCi…`) — use for CP/fee math reference + stock fuzz only |

**Audit honesty:** Findings against `token-swap-v2.0.0` source are **lineage-confirmed** where ELF strings/layout match; treat as **High confidence for math/auth patterns**, not as “Orca signed this exact commit.”

---

## 3. Instruction / curve surface (lineage)

From `token-swap-v2.0.0` `instruction.rs` + ELF:

| Tag (lineage) | Instruction |
|---------------|-------------|
| 0 | `Initialize` (fees + curve) |
| 1 | `Swap` |
| 2 | `DepositAllTokenTypes` |
| 3 | `WithdrawAllTokenTypes` |
| 4 | `DepositSingleTokenTypeExactAmountIn` |
| 5 | `WithdrawSingleTokenTypeExactAmountOut` |

| CurveType byte | Calculator |
|----------------|------------|
| 0 | ConstantProduct |
| 1 | ConstantPrice |
| 2 | **Stable** (amp) |
| 3 | Offset |

`InitOffsetCurve` appears as a debug/log string in the Orca ELF; may be an intermediate fork variant or Debug name — **residual:** confirm exact ix set via disasm if filing fork-specific bugs.

---

## 4. Manual review findings

### OV1-M01 — Permissionless pool init (unconstrained build) — **Medium**

**Evidence:**
- No embedded fee-owner pubkey / `SWAP_PROGRAM_OWNER_FEE_ADDRESS` string in ELF.
- No little-endian `u64` value `10000` anywhere in the binary — so the classic `production` fee blob `(25,10000,5,10000,0,0,20,100)` is **not** present as static data.
- Constraint *error* strings exist (enum always compiled), but without `production` + owner key, `SWAP_CONSTRAINTS` is `None` and init skips owner/fee/curve allowlist (`processor.rs` ~282–292 in v2.0.0).

**Impact:** Anyone can create pools with attacker-chosen fees/curves (honeypot / malicious fee configs / amp=0 “stable”). Classic SPL Token-Swap class risk. Live residual depends on whether UI routes only through known pools.

**Mitigation on-chain:** none (immutable). Off-chain: UI/token-list allowlists.

### OV1-M02 — `StableCurve::validate` is a no-op TODO — **Medium (lineage)**

```193:196:orca-whirlpool/sources/spl-token-swap-v2/token-swap/program/src/curve/stable.rs
    fn validate(&self) -> Result<(), SwapError> {
        // TODO are all amps valid?
        Ok(())
    }
```

**Impact (path-confirmed on lineage):** `amp = 0` passes validation at init.  
- **Swap:** `leverage = amp * N_COINS` → 0; subsequent `checked_div(leverage)` fails → swap returns calculation error (**fail closed**).  
- **Deposit/withdraw (all / single):** StableCurve’s `pool_tokens_to_trading_tokens` / `trading_tokens_to_pool_tokens` are **proportional helpers that ignore `amp`**. So an amp=0 “stable” pool can still accept/return LP proportionally while **swaps never work**.  

**Class:** grief / honeypot pool (UI risk), not direct vault theft. Reinforces OV1-M01 (permissionless init).

### OV1-M03 — Fee “minimum 1 token” when nonzero fee rounds to 0 — **Info / design**

`calculate_fee`: if `num≠0` and computed fee is 0, fee becomes **1**. Dust trades pay disproportionate fees. Known SPL behavior; not theft of vaults.

### OV1-M04 — Host fee is cut of owner trade fee — **Info**

Host account optional on swap; mis-set host does not drain vaults beyond configured owner fee share.

### OV1-M05 — Account binding (lineage) — **Positive**

Init checks: PDA authority owns vaults + pool mint authority; fee & destination **must not** be owned by authority; pool mint supply 0; no delegates/close authority on vaults; fee account mint == pool mint. Stronger than some era AMMs (pool mint binding present).

### OV1-M06 — No runtime admin / set-fees ix — **Positive**

Fees/curve fixed at init. Immutable program ⇒ no post-deploy fee rug via upgrade.

### OV1-M07 — Token-2022 — **Likely N/A for this binary**

Built against SPL Token era ~1.5.4; modern Token-2022 transfer-fee/hooks not in this ELF. Residual: weird mints may simply fail unpack.

### OV1-M08 — Production constraints (if ever enabled) ban Stable/Offset at init — **Info**

v2.0.0 `production` allowlist is ConstantProduct + ConstantPrice only. ELF still contains Stable/Offset (for swap against existing pools or non-production build). Combined with OV1-M01, unconstrained build is the worse case.

---

## 5. Fuzz campaign

| Harness | Status | Location |
|---------|--------|----------|
| Modern SPL curve/proptest (`cargo test --features fuzz --lib curve`) | **65 passed**, 0 failed | `fuzz/token-swap-v1/logs/proptest_curve.txt` |
| Orca-oriented pure math honggfuzz (CP + Stable D + fees) | **Clean sample:** 210 664 iters, **0 crashes**; 30 min campaign running | `fuzz/token-swap-v1/math_fuzz/` |
| Stock SPL `token-swap-instructions` honggfuzz | Queued | `audit_work/sources/spl-lib/token-swap/program/fuzz` |

**Invariants under test:** CP output ≤ reserve; k non-decrease when products fit u128; Stable D finite / non-explode; fee den=0 fail-closed for amount>0; fee ≤ amount when fraction would pass `Fees::validate`.

### 5.1 Crash triage (harness FPs — not on-chain bugs)

1. Fee `num > den` → harness FP; on-chain rejected at init.  
2. CP k-check used `saturating_mul` then `+` → **u128 overflow in harness** on max inputs.  
3. `amount==0 && den==0 && num!=0` → `calc_fee` returns `Some(0)` before den check; init still rejects.  

After fixes: **0 crashes** in clean 90s run (~210k iters, ~44% branch coverage).

### OV1-M09 — `calculate_fee` alone does not cap fee ≤ amount — **Info**

If account data were corrupt / packing bypassed validation, `amount * num / den` with `num > den` yields fee > amount. Normal path: validate at init + immutable fees. Not a live exploit without memory corruption.

---

## 6. Residual risks / close-out

| Residual | Status |
|----------|--------|
| Math fuzz long campaign (~30 min) | **DONE — no crashes** (corpus ~538+; clean through full timeout window) |
| Stock SPL `token-swap-instructions` honggfuzz | Running (honggfuzz 0.5.55); early sample no crashes |
| `amp=0` LP path | **Confirmed** on lineage (OV1-M02) |
| `InitOffsetCurve` string | Unresolved fork/debug name — Low residual |
| Live pool `SwapV1` layout spot-check | Open (optional) |

**V1 audit posture for queue advancement:** Identity + manual findings + clean math/proptest sample are sufficient to mark **phase-complete with residuals**; deep V2 can proceed in parallel with leftover fuzz.

---

## 7. Severity summary (so far)

| ID | Sev | Title |
|----|-----|-------|
| OV1-M01 | Medium | Unconstrained permissionless init (likely) |
| OV1-M02 | Medium | StableCurve amp validation TODO / amp=0 |
| OV1-M03 | Info | Min fee of 1 |
| OV1-M05/M06 | Positive | Binding + no set-fee + immutable loader |

No Critical fund-theft finding confirmed on V1 this pass without a live exploit path beyond malicious-pool creation (social/UI risk).
