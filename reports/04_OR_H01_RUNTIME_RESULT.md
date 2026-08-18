# OR-H01 Runtime Verification Result (revised)

**Finding:** Orca Whirlpool `DynamicTickArrayLoader` — cast short account body to max size then `rotate_right(112)`  
**Date:** 2026-08-13 (deep re-probe same day)  
**Runtime:** LiteSVM 0.8.2 (Agave program-runtime / rBPF)  
**Program:** `or_h01_runtime/deploy/or_h01_poc.so` (`343SfmnV2pQqDRJ7TJFm1NnXewKELe5LPvgeUV2wpsPi`)

---

## Executive correction

The earlier **host contiguous model without 10 KiB realloc padding was wrong for real Agave**.

Under standard BPF input serialization:

```
[meta A][data A][≤10240 bytes realloc pad + align][meta B][data B]
```

Whirlpool-sized cast OOB depth from a `MIN_LEN` DynamicTickArray is **~9864 bytes**.  
Distance to next account meta is **~10252 bytes**.  

**⇒ Cast + rotate cannot reach the next account.** Neighbor smash is **not** achievable with this bug class on stock Agave layout.

---

## What we re-tested (focus on what worked)

### A. Corrected host layout model

Artifact: `poc_or_h01_correct_layout`

| Metric | Value |
|--------|--------|
| OOB past A end | 9864 |
| Pad size | 10252 |
| Reaches B meta? | **false** |
| Reaches B data? | **false** |
| Pad touched by rotate? | **true** |
| B corrupted? | **false** |

Old model (no pad) still “hits B” — that was the false Critical signal.

### B. LiteSVM multi-mode deep probe

Log: `or_h01_runtime/tests/or_h01_litesvm/run_deep_probe.txt`

| Mode | Historical (`FeatureSet::default`) | All features |
|------|--------------------------------------|--------------|
| **0** Whirlpool rotate | OK, tick mutates, **victim clean** | **InvalidRealloc**, victim clean |
| **3** Production `MAX_LEN` cast+rotate | Same as 0 | **InvalidRealloc** |
| **1** Write `0xEE` at body+off | In-bounds off commits; pad offs “succeed” but **not committed** to account; **victim always clean** even at off=12000 | Same pattern for single-byte writes |
| **2** Write `data_len` (u64 before data) + paint pad | **Tick grows 148→404**, tail=`0xEE…` | **Same growth** |
| Victim corrupted in any case? | **Never** | **Never** |

### C. Mode 2 detail — real commit, but **self** only

```
mode2 inflate data_len -> 404
tick_len 148→404  new_tail=ee ee ee …
*** TICK INFLATED WITH PAD COMMIT ***
victim still all 0xAA
```

This is **manual realloc via input buffer** (same mechanism official `realloc` uses: bump `data_len`, fill pad, deserialize copies `post_len`).  
Bounded by `MAX_PERMITTED_DATA_INCREASE` (10 KiB). **Not neighbor corruption.**  
Whirlpool does **not** write `data_len` this way; it calls `resize()` then rotate.

---

## Why the original “Critical” path fails

1. **Padding wall:** OOB ≤9864 < ~10252 to next meta. Math closed.  
2. **Copy-back:** Without inflating `data_len`, deserialize only commits original `pre_len` (or official realloc length). Pad pollution discarded.  
3. **Modern features:** Large OOB store patterns (rotate over ~10 KiB virtual) → `InvalidRealloc` (“Failed to reallocate account data”).  
4. **Production order:** Liquidity paths **resize then** `load_mut`/`update_tick` — still cast to full max, but still cannot bridge the pad to a second account.

---

## Residual real effects (not fund-theft Critical)

| Effect | Severity | Notes |
|--------|----------|--------|
| Unsafe cast / UB in source | **High (code quality)** | `load_mut` ignores actual slice len |
| Pad OOB on short TA | Runtime-dependent | Historical: silent; modern: can trap |
| InvalidRealloc on short TA + rotate under modern SVM | **DoS / compatibility risk** | If a short DynamicTickArray hits rotate without enough mapped room, tx fails |
| Cross-account fund theft via this cast | **Debunked** | Layout math + LiteSVM: never hit victim |
| Manual `data_len` poke | By design (realloc-class) | Grows **self** only; not Whirlpool’s code path |

---

## Severity decision (final)

### **High** — pattern real, should fix bounds-check before cast  
### **Critical (neighbor smash / theft) — DEBUNKED** under real Agave input layout + LiteSVM e2e

**Not upgraded to Critical.** The install/runtime path that “worked” (historical success + host model smash) was re-examined: the smash required an **incorrect** host model, and every live probe left the victim account untouched.

---

## Install path (unchanged, still valid)

| Component | Status |
|-----------|--------|
| Agave 4.2 + cargo-build-sbf / platform-tools v1.54 | OK (`--no-rustup-override`) |
| SBF `.so` | OK |
| LiteSVM 0.8.2 via upstream workspace lockfile | OK |
| `solana-test-validator` | Blocked (no AVX on this host) |

### Rebuild SBF

```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"
PT=$HOME/.cache/solana/v1.54/platform-tools
export PATH="$PT/rust/bin:$PT/llvm/bin:$PATH"
export LD_LIBRARY_PATH="$PT/rust/lib:$PT/llvm/lib"
cd audit_work/or_h01_runtime/programs/or_h01_poc
cargo-build-sbf --no-rustup-override
```

---

## Artifacts

| Path | Role |
|------|------|
| `or_h01_runtime/programs/or_h01_poc` | Multi-mode probe program (0=rotate,1=OOB write,2=inflate,3=prod cast) |
| `or_h01_runtime/deploy/or_h01_poc.so` | Built SBF |
| `or_h01_runtime/tests/or_h01_litesvm/run_deep_probe.txt` | Full multi-mode LiteSVM log |
| `poc_or_h01_correct_layout` | Host model **with** 10 KiB pad |
| `poc_or_h01_agave_model` | Old model **without** pad (superseded for Critical claims) |

---

## Bottom line

We pushed the path that actually executed (historical rotate success, modern InvalidRealloc, host model).  
**Corruption of an adjacent victim account does not land** once Agave’s 10 KiB realloc padding is modeled and once LiteSVM is exercised with explicit OOB writes and `data_len` inflation.  

The only durable “commit” we produced was **self-account growth** via writing the input `data_len` field — a realloc-class primitive, not a Whirlpool neighbor-smash exploit.

**OR-H01 stays High. Critical cross-account exploit is closed on current evidence.**

---

## Opinion follow-up (Mode A read / self-corruption / full-size)

Log: `or_h01_runtime/tests/or_h01_litesvm/run_opinion_followup.txt`

### Mode A exact mutation (read gadget)

| Fill | Changed region | New bytes | Victim gradient appear? |
|------|----------------|-----------|-------------------------|
| `0x00` | none | — | no |
| `0x11` / `0xCC` / `0xDD` | offs **60..147** (88 B) | **all `0x00`** | **no** |

`rotate_right(112)` pulls virtual tail from **zero-filled realloc pad** into the committed tick-data window. Not a cross-account read; not a non-zero metadata leak here.

### Full-size vs InvalidRealloc

| Setup | Historical | all_features |
|-------|------------|--------------|
| `MAX_LEN` + mode0 body cast | OK (in-bounds) | OK |
| `MAX_LEN` + mode3 prod `[u8; MAX_LEN]` on `data[8..]` | OK (+8B effect) | **InvalidRealloc** |
| one byte short of body max | — | **InvalidRealloc** |

Production type size on `data[8..]` is always **+8 OOB** even at full account size.

### Self-corruption → oracle

Mode A on short TA only **zeros** tick payload — not a controlled forge of liquidity/fee fields. Production `update_tick` resizes then writes a real tick after rotate. Economic self-poison path remains unproven.

### Agreement with external writeup

High ceiling: agree. InvalidRealloc mitigation: agree. Mode A “read primitive”: characterized as pad zeros. CPI/Firedancer/feature-gate timing: still open residual.

---

## Residual closure (second opinion writeup)

Log: `or_h01_runtime/tests/or_h01_litesvm/run_residual_probes.txt`

### A. Padding content / infoleak — **STERILE**

Already proven in `run_opinion_followup.txt` and recap: Mode A rotate commits **only `0x00`** into offs 60..147.  
Agave serialize zero-fills the 10 KiB pad → read-into-tick is **not** an infoleak of heap/metadata/neighbor.

### B. Production `load_mut` / resize ordering — audited

| Path | Order | Hits rotate? |
|------|-------|----------------|
| `initialize_dynamic_tick_array` | `load_mut` → `initialize` (header only) | **No** |
| `increase_liquidity` (+v2) | calculate → **resize Increase** → reload → `sync`/`update_tick` | rotate only after +112 size |
| `decrease_liquidity` (+v2) | calculate → **`sync`/`update_tick` first** → then resize Decrease | **rotate_left while still large** |
| `swap` | `update_tick` on **already-initialized** ticks | typically **no** init/uninit transition → **no rotate** |
| Pinocchio liquidity | same pattern as increase/decrease | same |

**PDA constraint:** tick array seeds = `[b"tick_array", whirlpool, start_tick]`. Attacker cannot substitute a short foreign TA into a victim pool’s swap/LP ix (owner + whirlpool field + PDA checks).

**Self-poison economic path:** Mode A only **zeros** 88 B of tick payload on short arrays under historical features — not a controlled forge of liquidity/fee fields. Production then overwrites with real `TickUpdate` after rotate on init transition. **No unprivileged mispricing path proven.**

Note: **decrease** does `update_tick` (possible `rotate_left`) **before** shrinking — opposite of increase. Still uses full virtual cast; modern bulk rotate → `InvalidRealloc` risk on any short-of-MAX dynamic TA, independent of neighbor smash.

### C. `InvalidRealloc` vs single-byte / loop stores — **CONFIRMED DIFFERENCE**

| Op (short 148 B account, all_features) | Result |
|----------------------------------------|--------|
| `rotate_right` (mode0) | **ERR InvalidRealloc** |
| single store at body+9500 (mode1) | **OK** (not committed) |
| byte loop 9000..9995 (mode4) | **OK** (not committed) |
| byte loop 9800..9900 | **OK** |
| byte loop 52..200 (covers committed) | **OK**, 88× `0xEE` **committed** in tick |

So modern trap is **not** “any access past `data_len`.” Bulk rotate-style ops trap; raw single-byte / loop stores into the reserved growth region **do not** under LiteSVM `all_enabled`.

**Exploit meaning:**
- Stealthier OOB **write** that returns success: yes (pad only, discarded).
- Cross-account smash: **still no** (padding wall).
- Infoleak: still need a **read** of pad into committed bytes → rotate does that and only yields zeros.
- DoS: rotate path fails loud; loop path fails soft (success, no state) — griefing value limited.

### D/E. CPI & Firedancer

Still **untested**. Do not reopen Critical on Agave evidence alone.

### Severity after residual work

| Item | Severity |
|------|----------|
| Unsafe cast | **High** (fix) |
| Neighbor smash / theft | **Closed** |
| Pad → tick infoleak | **Closed** (zeros) |
| Byte-loop OOB without trap | **Info / Low** (no commit, no neighbor) |
| Self-corruption economic | **Unproven** (PDA + production overwrite) |
| Decrease-before-resize + modern trap | **Compat/DoS residual** if short TA hits rotate |
| CPI / Firedancer | **Open residual** |

**Critical remains unjustified on current evidence.**

---

## Feature-gate isolation: +8 production OOB / liveness claim

Log: `or_h01_runtime/tests/or_h01_litesvm/run_feature_gate.txt`

### Gate identity

| Item | Value |
|------|--------|
| Feature name | **SIMD-0219** `stricter_abi_and_runtime_constraints` |
| Feature pubkey | `CxeBn9PVeeXbmjbNwLv6U4C6svNxnC4JX6mfkvgeMocM` |
| Related | `account_data_direct_mapping` = `CR3dVN2Yoo95Y96kLSTaziWDAQT2MNEpiWh5cqVq2pNE` (Agave 4.2 set; not required for trap in LiteSVM 0.8 path) |

### Isolation experiment (LiteSVM)

| Feature set | mode3 full (10004) | mode0 short (148) |
|-------------|--------------------|-------------------|
| `FeatureSet::default()` | **OK** | **OK** |
| **only** `stricter_abi` activated | **InvalidRealloc** | **InvalidRealloc** |
| `FeatureSet::all_enabled()` | **InvalidRealloc** | **InvalidRealloc** |

⇒ Trap is **not** “all_enabled mystery.” It is **SIMD-0219 alone**.

### Mainnet activation (2026-08-13)

| Check | Result |
|-------|--------|
| `getAccountInfo(CxeBn9…)` mainnet | **AccountNotFound** |
| `getAccountInfo(CR3dVN…)` mainnet | **AccountNotFound** |
| simd.wtf / simd.watch SIMD-0219 | **Review / not mainnet-activated** |

⇒ **Whirlpool is not currently DoS’d on mainnet by this gate.**  
LiteSVM `all_enabled()` **over-approximates a future** cluster, not today’s mainnet.

### Production +8 OOB (source)

```text
DynamicTickArrayLoader([u8; MAX_LEN])  // MAX_LEN includes 8-byte disc
load_mut(&mut data[8..])               // body length = data_len - 8
```

Even when `data_len == MAX_LEN`, the cast claims **MAX_LEN** bytes starting at `data[8]`, i.e. **always +8 past account end**.  
Today (no SIMD-0219): silent write into zero pad (historical OK).  
After SIMD-0219: bulk rotate → **InvalidRealloc** even on “full” dynamic TAs that hit rotate.

### Severity (refined)

| Finding | Severity | Notes |
|---------|----------|--------|
| Unsafe cast (source) | **High** | Fix: cast with actual `data.len()` or body-sized type |
| +8 structural OOB on full TA | **High (future-compat)** | Becomes liveness when SIMD-0219 activates |
| Live mainnet liveness break today | **No** | Feature account absent |
| “Critical Liveness now” | **Not justified** | Pending gate, not active |
| Neighbor smash / theft | **Closed** | Unchanged |
| CPI / Firedancer | **Open residual** | Untested |

**Recommendation to Orca:** treat SIMD-0219 readiness as a real ship gate — change `load_mut` to not overclaim disc length (e.g. body max type = `MAX_LEN - 8`, or pass full account including disc into a correctly sized type). Not a fund-theft Critical; **is** a pre-activation fix requirement.

