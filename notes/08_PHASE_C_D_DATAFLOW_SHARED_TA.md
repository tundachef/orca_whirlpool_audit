# 08 — Phase C economic data-flow + Phase D shared-TA (host)

**Date:** 2026-08-19  
**Depends on:** `07_DYNAMICTICKARRAY_REVISED.md` (P1 measured; Phase B pollution negative)  
**Harnesses:** `fuzz/dta_shared_ta/` → `fuzz/logs/dta_shared_ta.txt`

---

## Phase C — Economic data-flow map

### C.1 Initialized tick byte layout (113 bytes)

```text
offset  size  field
------  ----  -----
0       1     Borsh enum tag (1 = Initialized)
1       16    liquidity_net: i128
17      16    liquidity_gross: u128
33      16    fee_growth_outside_a: u128   (Q64.64)
49      16    fee_growth_outside_b: u128
65      48    reward_growths_outside: [u128; 3]
```

Uninitialized = **1 byte** tag `0`.

### C.2 Dependency chains (byte → field → function → instruction → money)

#### Chain 1 — Swap active liquidity (highest severity *if* corrupted)

```text
tick.liquidity_net
  → swap_manager::calculate_update
  → signed_liquidity_net = ±liquidity_net
  → add_liquidity_delta(pool.liquidity, signed)
  → curr_liquidity
  → swap step amounts / amount_out
```

**Financial variable:** swap `amount_out`, price impact, pool `liquidity`.

#### Chain 2 — LP fee accounting

```text
tick.fee_growth_outside_{a,b}
  → tick_manager::next_fee_growths_inside
  → position_manager::next_position_modify_liquidity_update
  → position.fee_owed / fee_growth_checkpoint
  → collect_fees / decrease_liquidity payouts
```

**Financial variable:** LP fee claims.

#### Chain 3 — Reward accounting

```text
tick.reward_growths_outside[*]
  → next_reward_growths_inside
  → position reward checkpoints
  → collect_reward
```

**Financial variable:** reward token claims.

#### Chain 4 — Tick cross state update

```text
tick.* (all outside growths)
  → next_tick_cross_update
  → update_tick on crossed tick (flips outside growths)
  → later fee/reward inside calculations
```

#### Chain 5 — Liquidity modify path

```text
get_tick(lower/upper)
  → calculate_modify_liquidity
  → next_tick_modify_liquidity_update (reads liquidity_net/gross, outside growths)
  → position + tick updates + possible resize/rotate (P1 trigger)
```

#### Chain 6 — Bitmap address calculation (not a tick payload field)

```text
tick_bitmap
  → byte_offset(i)
  → which physical bytes get_tick / update_tick touch
```

Corrupt bitmap ⇒ wrong-tick R/W (Phase B invariant track; **held** under tested transitions).  
Rotate OOB touches **tick_data only**, not the header bitmap — bitmap not in the OOB envelope.

### C.3 Ranking (given Phase B pollution negative)

| Rank | If these bytes were wrong… | Economic effect | Evidence they *are* wrong via P1 OOB? |
|------|----------------------------|-----------------|----------------------------------------|
| 1 | `liquidity_net` | Wrong pool liquidity on cross → bad `amount_out` | **Not in committed body** (Phase B/D) |
| 2 | `fee_growth_outside_*` | Wrong LP fees | Not in committed body |
| 3 | `reward_growths_outside` | Wrong rewards | Not in committed body |
| 4 | enum tag / layout | Mis-decode / panic / wrong size | Not in committed body |
| 5 | bitmap | Wrong offsets | Not touched by rotate OOB |
| 6 | past-boundary only (never committed) | SVM/padding only | Phase A dirty canary; impact = Phase E |

**Phase C conclusion:** The *economic sinks* are clear and severe **if** tick payloads corrupt. Under production resize+serialize+shrink, P1’s past-boundary writes have **not** been observed to deliver that corruption into committed tick fields. Impact search therefore shifts to:

1. a transition/failure mode where pollution **does** stick, or  
2. SVM mapping of the OOB envelope to something other than discardable padding, or  
3. P2b (Initialized deserialize across real boundary), or  
4. non-layout effects (DoS via planted failure — still weak).

---

## Phase D — Shared-TA attacker/victim (host)

### D.1 Model

```text
Attacker LP ──┐
              ├── same pool / same DynamicTickArray
Victim LP   ──┘
```

### D.2 Arms

| Arm | Attacker transition | Backing |
|-----|---------------------|---------|
| **BASELINE** | none | exact body |
| **CONTROL** | same logical Uninit↔Init | production `update_tick` on **MAX_LEN** (no short OOB) |
| **EXPERIMENTAL** | same logical Uninit↔Init | production `update_tick` on **exact size + canary** (Phase A path) |

### D.3 Victim observables

- `get_tick(lower)`, `get_tick(upper)` via production loader  
- `next_fee_growths_inside`  
- `add_liquidity_delta` using `±liquidity_net` (swap-cross stand-in)  
- committed account body bytes  

### D.4 Results (`fuzz/dta_shared_ta`)

| Scenario | Attacker changes vs baseline? | CONTROL ≡ EXPERIMENTAL? |
|----------|-------------------------------|-------------------------|
| empty→init tick7; victim 7..20 | yes (victim tick created) | **yes** |
| half-pop; init 15; victim 10..30 | yes | **yes** |
| half-pop; uninit victim lower=10 | yes | **yes** |
| near-full; init last; victim 0..10 | body changes; victim ticks may be unchanged | **yes** |
| full; uninit mid=44; victim 40..50 | yes | **yes** |
| half-pop; init 87; victim 2..8 disjoint | body changes | **yes** |

**Interpretation:**

- Sharing a TA is real: attacker transitions **do** change victim-visible state when they touch victim ticks (or shared layout). That is normal LP behavior.  
- The **OOB path does not add** divergence beyond the correct control transition on host.  
- First Phase D question — *“Does the attacker’s OOB operation alter victim state differently than a correct transition?”* — answer so far: **No** (host).

### D.5 Still open (persistence / atomicity / SVM)

```text
OOB write
  → memory corruption          ✅ host canary (Phase A)
  → persistent account bytes   ❌ not in logical body (B/D)
  → cross-instruction effect   open (needs runtime tx)
  → cross-transaction effect   open
  → rollback vs partial commit open (atomicity)
```

Host cannot settle commit/rollback; that is Phase E.

---

## Combined stance after C+D

```text
P1 memory-safety bug     CONFIRMED + measured
Economic sinks           MAPPED (liquidity_net ≫ fees ≫ rewards)
Committed layout corrupt NOT OBSERVED (B)
Shared-TA OOB extra FX   NOT OBSERVED (D host)
Bounty impact: not demonstrated; primary host exploit paths negative
```

**Superseded next steps:** `09_OOB_ENVELOPE_PIVOT.md` (P2b closed under invariant; Phase E E1–E5 only).

---

## One-line update for reviewers

> Past-boundary `rotate_*` modifications are confirmed on the production path, but a bounded reference differential, short-account pollution suite, and shared-TA control/experimental host tests have not shown those modifications altering committed tick payloads or victim fee/liquidity observables beyond a correct state transition.
