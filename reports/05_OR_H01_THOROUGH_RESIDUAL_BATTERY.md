# OR-H01 Thorough Residual Battery

**Date:** 2026-08-13  
**Purpose:** Run every residual test proposed in the final “high confidence / not 100%” writeup, document results, assumptions, and compromises so a Critical cannot be missed by incomplete testing.

**Logs:**
- `or_h01_runtime/tests/or_h01_litesvm/run_thorough_battery.txt` (normal)
- `or_h01_runtime/tests/or_h01_litesvm/run_thorough_battery_dm.txt` (`OR_H01_FORCE_DM=1`)

---

## Assumptions (explicit)

1. **LiteSVM 0.8.2 + Agave program-runtime ~3.0.10** is a faithful model of stock Agave BPF serialization for the feature sets we enable.
2. **CPI** uses the same `serialize_parameters` family as direct invoke for the callee (Agave code path).
3. **`account_data_direct_mapping` without `stricter_abi`** does not change the non-strict branch of `write_account` (still contiguous copy + 10 KiB pad). Forcing DM alone is only meaningful together with SIMD-0219.
4. **Mainnet today** does not have SIMD-0219 / DM feature accounts (`getAccountInfo` null for `CxeBn9…`, `CR3dVN…`).
5. **Program under test** is the minimal PoC (`or_h01_poc`), not the full Whirlpool binary — it **mirrors** the cast/rotate pattern, not full swap math.
6. **Victim “corruption”** means post-tx account bytes differ from pre-tx fill. We use a gradient fill so changes are unambiguous.

---

## Compromises (what we could not run)

| Item | Status | Impact on certainty |
|------|--------|---------------------|
| **`solana-test-validator`** | **Blocked** — host CPU has SSE only, no AVX; prebuilt Agave validator aborts | Cannot multi-process full validator e2e |
| **Firedancer SVM harness** | **Not installed** (`fdctl` / images absent) | Cannot falsify pad model on FD |
| **True OS page-level direct mapping** | LiteSVM models VM regions, not host `mmap` page adjacency | Cannot prove/disprove “4 KiB page gap” host layout theory |
| **Full Whirlpool binary e2e** | PoC only; production path order audited in **source** | Economic self-poison not e2e against real swap CU path |
| **Mainnet live feature list beyond RPC null checks** | Feature accounts null; no multi-RPC / explorer cross-check of every related gate | Small residual on “is 0219 really off everywhere” |

---

## Tests executed

### A. Direct invoke matrix

| Case | hist | stricter (0219) | all_enabled |
|------|------|-----------------|-------------|
| mode0 short148 + victim165 | OK, tickΔ, **victim clean** | InvalidRealloc | InvalidRealloc |
| mode0 short148 + victim1024 | OK, tickΔ, **victim clean** | InvalidRealloc | InvalidRealloc |
| mode0 full + victim165 | OK, **victim clean** | OK, clean | OK, clean |
| mode3 full (+8 cast) + victim165 | OK, tickΔ (+8 path), **victim clean** | InvalidRealloc | InvalidRealloc |
| mode0 short **rev_order** | OK, see §False positive | InvalidRealloc | InvalidRealloc |

### B. CPI matrix (caller `or_h01_cpi_caller` → `invoke` → `or_h01_poc`)

Same feature × size × mode grid. **Every normal-order CPI case: `victim_corrupted=false`.**

| CPI hist short | OK, tickΔ, victim clean |
| CPI hist full mode0 | OK, clean |
| CPI hist mode3 full | OK, tickΔ, clean |
| CPI stricter short | InvalidRealloc |
| CPI stricter mode3 full | InvalidRealloc |
| CPI all short | InvalidRealloc |

**Conclusion:** CPI re-serialization **does not** reopen neighbor smash under LiteSVM for any feature set we tried.

### C. Force direct-mapping flag (`OR_H01_FORCE_DM=1`)

Patched LiteSVM after `InvokeContext::new`:

```rust
if env OR_H01_FORCE_DM { invoke_context.account_data_direct_mapping = true; }
```

**Results identical** to non-DM run (including CPI). Consistent with code: without `stricter_abi`, `write_account` ignores DM and uses contiguous + pad path; with `stricter_abi`, OOB bulk ops trap before any “page bridge.”

### D. Production path source audit (Whirlpool)

| Path | `load_mut` / rotate | Resize order |
|------|---------------------|--------------|
| `initialize_dynamic_tick_array` | `load_mut` → header only | No rotate |
| `increase_liquidity` (+v2, pinocchio) | After **Increase** resize | rotate only after +112 |
| `decrease_liquidity` (+v2, pinocchio) | **Before** Decrease resize | rotate_left while still large |
| `swap` / `swap_manager` | `update_tick` on usually **initialized** ticks | No init/uninit → **no rotate** |
| Pinocchio liquidity | Same pattern as anchor paths | Same |
| Oracle / adaptive fee `load_mut` | Different account type | N/A to TA rotate |

No admin/cleanup path found that calls `DynamicTickArrayLoader::update_tick` without going through the above. Residual: “unknown code” is never zero, but **all grep hits for rotate on DynamicTick** are in `update_tick` / tests.

### E. Offsets 60–147 economic meaning (MIN_LEN layout)

```
Account MIN_LEN=148:
  [0..8)   discriminator
  [8..12)  start_tick_index
  [12..44) whirlpool pubkey
  [44..60) tick_bitmap
  [60..148) 88 × DynamicTick::Uninitialized (1 byte each)
```

Mode A zeros **exactly** the 88 uninit tick marker slots. On a **real** freshly initialized DynamicTickArray those bytes are already uninit markers (not full `DynamicTickData` liquidity/fee fields). Full initialized tick fields (`liquidity_net`, `fee_growth_outside_*`, …) only exist after growth past MIN_LEN.

**Implication:** Zeroing 60–147 on a true MIN array is **not** zeroing `fee_growth_outside` / `liquidity_net` of an initialized tick; it zeros empty slots. Controlled price oracle via Mode A alone remains **unproven / unlikely**.

### F. Firedancer / AVX validator

| Tool | Result |
|------|--------|
| AVX | **Absent** on this host |
| `solana-test-validator` | Aborts: incompatible CPU |
| Firedancer | **Not installed** |

**Compromise:** Cannot close FD residual on this machine.

---

## False positive: `rev_order` “victim corrupted”

```
direct/hist/mode0/short/rev_order  victim_corrupted=true  diff=104
```

**Cause:** Harness put **victim as accounts[0]**. PoC always mutates **accounts[0]** as the tick. So the program **intentionally** rotated the gradient account — not OOB into a second account.

Evidence: `tick_changed=false` (true tick was accounts[1], never written); head of victim still gradient prefix; ~104 bytes changed at tick-region offsets (consistent with rotating the 165 B account as if it were the TA).

**Under normal order `[tick, victim]`:** **zero** cases of `victim_corrupted=true` across direct + CPI + DM force.

---

## Results summary

| Hypothesis | Result |
|------------|--------|
| Neighbor smash direct invoke | **Not observed** (normal order) |
| Neighbor smash **CPI** | **Not observed** |
| DM force changes outcome | **No** |
| SIMD-0219 traps short/mode3 full | **Confirmed** (direct + CPI) |
| mode0 full-size (in-bounds body cast) under 0219 | **OK** |
| Firedancer | **Untested (unavailable)** |
| Economic self-poison via zeroing MIN body | **Layout argues no** (uninit slots only) |

---

## Residual risk after this battery (honest)

| Risk | After battery | Notes |
|------|---------------|-------|
| Agave direct + CPI fund-theft | **Very low** | CPI matrix clean |
| True host-page DM layout | **Low–medium residual** | LiteSVM ≠ kernel page map; mainnet DM feature acct null |
| Firedancer | **Medium residual** | Untested |
| SIMD-0219 liveness when activated | **High future-compat** | Still real; not live Critical today |
| Compound second bug | **Always residual** | Outside single-finding scope |

---

## Verdict (unchanged, better grounded)

| | |
|--|--|
| **Fund-theft Critical on stock Agave (direct + CPI, LiteSVM)** | **Not supported** by thorough battery |
| **Report severity** | **High** (unsafe cast + future 0219 +8 structural OOB) |
| **Separate note** | OR-H01b: +8 cast / SIMD-0219 readiness |
| **What still could flip Critical** | Firedancer e2e smash, or true mainnet DM page layout proof, or second validation bug |

---

## Artifacts

| Path | Role |
|------|------|
| `programs/or_h01_poc` | Multi-mode PoC (0 rotate, 1 write, 2 inflate, 3 prod cast, 4 loop) |
| `programs/or_h01_cpi_caller` | CPI wrapper |
| `deploy/*.so` | Built SBF |
| `run_thorough_battery*.txt` | Full matrices |

**Bottom line for the client:** Every **runnable** residual test proposed (CPI, feature isolation, DM flag force, reverse-order clarification, path audit, MIN layout economics) was executed. Compromises are listed. **No true-order victim corruption** in CPI or direct invoke. I still will not claim 100% against Firedancer/host-page theory — those require hardware/software we do not have here.
