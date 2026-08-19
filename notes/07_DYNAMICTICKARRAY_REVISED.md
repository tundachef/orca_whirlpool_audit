# 07 — DynamicTickArray revised investigation (post-review #2)

**Date:** 2026-08-19  
**Responds to:** methodology review of prior `#07` (pivot from “is there an OOB?” → “what useful primitive does that OOB give?”)  
**Prior:** `06_DYNAMICTICKARRAY_SOURCE_PRIMITIVES.md`  
**Phase A canary:** `fuzz/dta_canary/` → `fuzz/logs/dta_canary.txt`  
**Phase B layout/pollution:** `fuzz/dta_layout/` → `fuzz/logs/dta_layout.txt`

---

## 0. Stance after L3

The most important boundary has been crossed:

> **A past-boundary write was measured in a controlled host harness using the production cast/rotation sequence.**

So P1 is no longer “probably OOB.” It is:

> **Confirmed source-level memory-safety violation with measured out-of-bounds memory modifications in a host reproduction.**

It is **not yet**:

> exploitable memory corruption / bounty-qualifying impact

because those past-boundary bytes have not been mapped under Solana’s actual SVM memory / commit model into persistent program-visible or economic state.

**Further raw OOB measurement has diminishing returns.** The investigation pivots to the useful-primitive question:

```text
                     ~9752-byte past-boundary modification
                              │
             ┌────────────────┼────────────────┐
             ▼                ▼                ▼
       wrong tick data   economic state    runtime state
             │                │                │
             ▼                ▼                ▼
       bitmap/layout      swap/liquidity    SVM behavior
```

---

## 1. Accepted corrections (carry forward)

1. **Three layers** (do not collapse):
   - **L1:** Rust reference/object extent is invalid (length discarded).
   - **L2:** Program forms slices whose claimed length exceeds the backing account allocation.
   - **L3:** An operation **actually** reads/writes those bytes (must be measured).

2. **Agave** = impact under Solana memory model, **not** bug discovery.

3. **Neighbor smash:** low priority. “Can touch neighboring bytes” ≠ “can steal funds.”

4. **OOB table:** use **`N_at_rotate`**, not bare `k` — resize order differs by path.

5. **Anchor vs Pinocchio:** related but **distinct** length invariants.

6. **Instrumentation language:** report **bytes modified / dirty in the canary region**, not casually “bytes written” or “bytes meaningfully corrupted” unless those distinctions are separately proven.

---

## 2. Confirmed (source alone) — L1

| Claim | Status |
|-------|--------|
| `load`/`load_mut` cast `&[u8]` → `[u8; MAX_LEN]`, discarding `len` | **Confirmed** |
| Anchor callers pass `data[8..]` while `MAX_LEN` includes disc | **Confirmed** → permanent **≥8** extent mismatch |
| Pinocchio casts full account to `MemoryMappedDynamicTickArray` sized `MAX_LEN` | **Confirmed** → short-account overclaim only (no permanent +8 when full) |

---

## 3. Exact production order (keep prominent)

### Increase (Uninit → Init) — Anchor **and** Pinocchio

```text
calculate_modify_liquidity          // read-only load
drop(tick_array refs)
update_tick_array_accounts
   └── resize(+112)                 // FIRST
reload load_mut
sync_modify_liquidity_values
   └── update_tick
         └── rotate_right(112)      // SECOND  ← N_at_rotate is post-resize
         └── update_tick_bitmap
         └── serialize tick
```

Evidence: `increase_liquidity.rs`; pinocchio twin same.

### Decrease (Init → Uninit) — Anchor **and** Pinocchio

```text
calculate_modify_liquidity
sync_modify_liquidity_values
   └── update_tick
         └── rotate_left(112)       // FIRST  ← N_at_rotate is pre-shrink
         └── update_tick_bitmap
         └── serialize
drop(refs)
update_tick_array_accounts
   └── resize(-112)                 // SECOND
```

Evidence: `decrease_liquidity.rs`; pinocchio twin same.

### Implication for “empty ↔ one tick”

The production transition does **not** rotate on raw `MIN_LEN=148`.

Both directions rotate while the account physically holds the **larger** of the before/after packed sizes:

| Transition | Expected `N_at_rotate` | Real body | Claimed loader | Canary dirty span |
|------------|-----------------------:|----------:|---------------:|------------------:|
| Increase first tick | `MIN_LEN+112 = 260` | 252 | 10004 via body cast | **9752** |
| Decrease last tick | still `260` before shrink | 252 | 10004 | **9752** |
| Full MAX + Anchor body cast | `10004` | 9996 | 10004 | **8** |

```text
real body = 252
claimed   = 10004
OOB gap   = 9752
```

### `N_at_rotate` must be instrumented, not only arithmetic

`260 = 148 + 112` is arithmetic. The value that matters is the account length **at the moment rotate begins**:

```text
before resize      data_len = ?
after resize       data_len = 260   (increase)
before load_mut    data_len = 260
before update_tick data_len = 260   ← N_at_rotate
```

Phase A harness prints these probes explicitly (see §5 / canary log).

---

## 4. Layer-3 results — precise wording

Harness paints a **gradient** past `account_len` (uniform `0xA5` hid canary↔canary traffic during `rotate_*`).

**What the instrumentation detects:** a past-boundary byte is **dirty** iff its value differs from the expected gradient paint. That measures **bytes touched/modified in the canary region**, not a semantic claim that each dirty byte is “meaningfully corrupted” application state.

| Experiment | Claimed `shift_data.len()` | Dirty bytes past account | Inclusive span |
|------------|---------------------------:|-------------------------:|---------------:|
| `rotate_right` after +112 | 9952 | **9752** | 260…10011 |
| `rotate_left` before −112 | 9952 | **9752** | 260…10011 |
| Full account + Anchor cast | 9952 | **8** | 10004…10011 |

**Review-safe wording:**

> The canary harness detected modifications spanning **9,752** bytes beyond the supplied account-data boundary in the first-tick-sized production state (`N_at_rotate = 260`). The modifications are produced by the `rotate_*` operation over the falsely extended slice. For a full-size Anchor account the same pattern detects modifications spanning **8** bytes past the boundary.

---

## 5. Harness fidelity

### Goal

```text
production code
        │
        ▼
test backing allocation + canary
```

not:

```text
production source
        │
        ▼
recreated test implementation
        │
        ▼
canary
```

### Fidelity table

| Component | Production | Harness |
|-----------|------------|---------|
| `DynamicTickArrayLoader` | same crate | **same** (`whirlpool` path dep) |
| `load` / `load_mut` | same | **same** |
| `update_tick` | same | **same** |
| `tick_data_mut` | same (private, via `update_tick`) | **same** |
| `rotate_right` / `rotate_left` | same (`slice::rotate_*` inside `update_tick`) | **same** |
| account body offset | 8 | 8 (`data[8..]` cast) |
| account length at rotate | 260 (first-tick case) | 260 |
| resize ordering | grow-then-rotate / rotate-then-shrink | same simulated order |
| tick state transition | Uninit↔Init via `TickUpdate` | same |
| backing allocation | SVM account data | **host vec + gradient canary** |

**Only intentional difference:** memory backing mechanism (host allocation + canary vs SVM account).

Phase A upgraded `fuzz/dta_canary` from a manual reimplementation of cast+rotate to calling production `DynamicTickArrayLoader::load_mut` + `TickArrayType::update_tick`. Prior reimplementation results remain historically valid as a lower-fidelity cross-check; production-path results supersede them for reviewer arguments.

### Phase A measured (production-path, 2026-08-19)

```text
probe before_resize      data_len = 148
probe after_resize       data_len = 260
probe before_load_mut    data_len = 260
probe N_at_rotate        data_len = 260
→ canary modifications spanning 9752 bytes (increase via update_tick)
→ canary modifications spanning 9752 bytes (decrease via update_tick)
→ full Anchor cast: modifications spanning 8 bytes
```

`N_at_rotate = 260` is now an **instrumented** value on the production path, not only `148+112` arithmetic.

---

## 6. P2 split — P2a / P2b

Source:

```rust
let mut tick_data = &ticks_data[byte_offset..byte_offset + DynamicTick::INITIALIZED_LEN];
let tick = DynamicTick::deserialize(&mut tick_data)?;
```

### P2a — Oversized slice construction (source-level)

```text
get_tick() / update_tick()
    ↓
byte_offset .. byte_offset+113
    ↓
slice formed against claimed ~9952-byte tick region
```

**Status:** confirmed as L2 whenever `byte_offset + 113 ≤ claimed_len`.

### P2b — Boundary-crossing deserialize (needs L3)

```text
113-byte initialized representation
    ↓
actual account boundary
    ↓
deserialize physically accesses past boundary?
```

Borsh enum behavior:

| Tag | Physical read |
|-----|---------------|
| Uninitialized | **1** byte |
| Initialized | **113** bytes |

**Do not claim** “always physically reads 113 bytes for uninitialized ticks.”

**Open experiment:** can an attacker make `byte_offset` such that the **113-byte claimed slice** itself crosses the **real** account boundary, and then force the Initialized path (tag-controlled / corrupted) so deserialize performs a past-boundary read?

For every `get_tick()` call site of interest:

```text
byte_offset + 113  >  actual_tick_data_len_in_account  ?
```

If yes → P2a is live at the boundary. Then inspect whether deserialize takes the 113-byte path.

---

## 7. Finding split: Anchor vs Pinocchio

| | Anchor `DynamicTickArrayLoader` | Pinocchio `MemoryMappedDynamicTickArray` |
|--|----------------------------------|------------------------------------------|
| Cast input | `data[8..]` (body) | full `data` (with disc) |
| Full-size account | **Still +8 overclaim** | Size matches `MAX_LEN` |
| Short account | Overclaim `MAX_LEN - body_len` | Overclaim `MAX_LEN - account_len` |
| Rotate sink | inside `update_tick` → `rotate_*` | `ticks[off..].rotate_*` |
| Increase/decrease order | Same (§3) | Same |

Treat as **Finding A** and **Finding B**, same family, prove separately.

---

## 8. Representation invariants

### Invariant 1 — object extent

```text
actual_slice_len  >=  claimed_loader_len
```

**False** (Anchor always by ≥8; both false when short).

### Invariant 2 — packed layout ↔ bitmap (expand substantially)

```text
bitmap bit i set  ⟺  i-th packed record is 113-byte Initialized
```

```text
for every tick i:
  physical_offset(i) = sum(size(tick_0) … size(tick_{i-1}))
where size(U)=1, size(I)=113
```

Therefore:

```text
bitmap  →  which ticks are initialized  →  each tick’s physical size  →  byte_offset
```

**Bitmap is security-sensitive address calculation.**  
If bitmap and packed bytes diverge, the result is **wrong-tick R/W** without any neighbor smash — a second bug class independent of the OOB primitive.

**Open investigation:** does every mutation maintain `bitmap state == physical packed state`?

---

## 9. Who controls the DynamicTickArray?

| Capability | Who |
|------------|-----|
| Create/init dynamic TA | Anyone (init ix); chooses `start_tick_index` |
| Open position / pick ticks | LP (attacker can be LP) |
| Flip tick init bit on **own** position’s arrays | LP via increase/decrease |
| Force flip on **victim’s** TA | Only if sharing that TA (same pool range) via own position referencing same arrays |
| Freely choose arbitrary `data_len` | **No** — init/resizes are program-driven |

**Strongest useful scenario:** attacker LP shares a DynamicTickArray with other LPs → triggers transition → past-boundary modification → ask whether **victim’s subsequent program-visible state** changes (not “stolen money” first).

### Malformed-account track (separate, do not abandon)

Attacker does not freely choose `data_len`, but ask:

- Can a valid historical account have an unexpected size?
- Can migration leave old layouts?
- Can CPI pass an account that passes owner/PDA checks with unexpected length?
- Can upgrade/version transitions produce such an account?

Goal: **reachable** assumption violations, not manufactured impossible accounts.

---

## 10. Revised primitive ranking

| ID | Status | Statement |
|----|--------|-----------|
| **P0** | **Confirmed L1** | Unsafe fixed-size map discards slice length |
| **P1** | **Confirmed L1+L2; L3 measured (host, production-path)** | `rotate_*` over falsely extended tick region; canary detected modifications spanning **9752** bytes past boundary at first-tick-sized `N_at_rotate=260`; **8** on full Anchor cast |
| **P2a** | **Confirmed L2** | 113-byte slice *formation* on claimed extent |
| **P2b** | **Hypothesis** | Boundary-crossing deserialize when Initialized path + offset near real end |
| **P3** | **Unproven / highest bounty interest** | Affected bytes → liquidity/fee/swap accounting divergence |
| **P4** | **Weak** | Runtime fail ≠ victim DoS yet; interesting only if attacker can plant persistent state that makes a later legitimate user ix fail |
| Neighbor smash | **Low priority** | Not observed under tested Agave/LiteSVM; even if reachable, control/commit/ownership still block “funds theft” |

---

## 11. Investigation order — Phase A → F

### Phase A — Exact primitive — **DONE**

1. ~~Production-path canary with **actual** production loader/`update_tick`~~ ✅ `fuzz/dta_canary`
2. ~~Verify **`N_at_rotate`** with explicit probes~~ ✅ logged `148→260→260`
3. ~~Measure dirty ranges for both directions~~ ✅ spanning 9752 / 8
4. Attacker-controllable `byte_offset` variants — partially covered via pollution suite idxs; expand as needed

### Phase B — Representation — **semantic + pollution suite DONE; expand residual**

5. ~~Bitmap ↔ packed-layout differential / state-transition fuzzer~~ ✅ 0 diffs
6. ~~Bounded reference vs vulnerable on MAX_LEN~~ ✅ 0 diffs
7. ~~Short-account canary pollution → committed logical body~~ ✅ 0 diffs on suite
8. Residual: multi-step pollution sequences; failure-injection mid-update; Pinocchio twin

### Phase C — Economic data flow

7. For affected byte ranges: `byte range → field → function → instruction → financial variable`  
   Examples: `liquidity_net → cross_tick → swap → amount_out`; `fee_growth_outside → update_position → LP fees`
8. Rank consequences by whether they change program-visible economics

### Phase D — Shared-state attacker model

9. Attacker LP + victim LP share one DynamicTickArray (same pool)
10. **Control:** same pool/TA/victim ops, **no** vulnerable transition
11. **Experimental:** attacker triggers transition, then victim operates
12. Compare: tick data, bitmap, liquidity, fees, rewards, swap result, account bytes
13. First question: does attacker alter anything that changes victim’s **subsequent program-visible state**?
14. Persistence ladder: transient process mem → account-backed → committed → cross-ix → cross-tx
15. Atomicity: OOB then later error — full rollback vs partial persist?

### Phase E — Runtime (targeted)

16. LiteSVM / Agave with the **precise** primitive: rotate writes ~9752 past account-data boundary  
17. Question: what is the first byte beyond the account-data boundary under SVM, and how are cross-allocation writes handled?  
18. Map systematically across MIN-like / partial / near-max / MAX  
19. Commit / rollback behavior

### Phase F — Impact classification

20. Economic manipulation  
21. Persistent corruption  
22. Victim DoS (only if planted state fails later legitimate ix)  
23. Information disclosure  
24. Cross-account corruption (demoted until envelope understood)

---

## 12. Differential oracle — Phase B results

```text
REFERENCE                         VULNERABLE
actual &[u8] bounded ops          DynamicTickArrayLoader MAX_LEN view
        │                                  │
        └──────── identical transitions ───┘
                         │
                         ▼
        compare layout / bitmap / account bytes /
               liquidity / fees / swap / errors
```

### 12.1 Semantic track (MAX_LEN backing) — **no divergence**

Sequences run (see `fuzz/logs/dta_layout.txt`):

| Sequence | Transitions | Diffs |
|----------|------------:|------:|
| asc_init_all | 88 | **0** |
| asc_init_then_clear | 176 | **0** |
| pingpong | 176 | **0** |
| alternate_scramble | 176 | **0** |
| lcg_2k ×2 | ~1900 | **0** |

After every transition: bitmap ↔ packed layout invariant held; `physical_offset(i) == loader_byte_offset(i)`; reference body ≡ production loader body prefix.

### 12.2 Pollution track (short account + canary) — **no logical-body divergence (tested cases)**

Question answered:

> Do past-boundary canary bytes rotated by `update_tick` remain inside the **committed** packed body after serialize + (logical) resize?

| Case family | Result |
|-------------|--------|
| empty → first tick (idx 0/1/7/44/87) | OK — logical body ≡ reference |
| one tick → empty (same idxs) | OK |
| half-populated interior inc/dec | OK |
| near-full / full Anchor-sized inc/dec | OK |

**Interpretation (important):** Phase A proved OOB *modifications* are real. Phase B so far shows that under production resize ordering + `serialize` into the hole + shrink-to-packed-size, those OOB bytes **did not survive into the committed logical packed state** for the tested transitions. Likely because:

- `rotate_right` pulls past-boundary into the hole, then **113-byte serialize overwrites** that hole;
- `rotate_left` shifts past-boundary toward the tail, then **resize(-112) discards** the tail past the new packed length;
- Anchor’s permanent +8 overclaim similarly lands in the trimmed/overwritten fringe in the near-full cases tested.

This is a **negative impact result**, not a denial of P1. It narrows the useful-primitive search: either find a transition where pollution *does* stick, or move to shared-TA / SVM commit / economic channels that don’t require logical-layout corruption.

---

## 13. Shared-TA experiment sketch

```text
Attacker LP ──┐
              ├── same pool / same DynamicTickArray
Victim LP   ──┘

Control:      no vulnerable transition
Experimental: attacker triggers Uninit↔Init transition (rotate)

Compare victim-visible: ticks, bitmap, liquidity, fees, rewards, swap, raw account bytes
```

Without the control arm, differences are hard to interpret.

---

## 14. Persistence & atomicity (currently missing)

Given dirty canary bytes, classify:

| Level | Question |
|-------|----------|
| A | Only transient process memory? |
| B | Account-backed memory? |
| C | Committed back to the account? |
| D | Visible to another instruction? |
| E | Visible to another transaction? |

```text
OOB write → memory corruption → persistent corruption
         → cross-instruction effect → cross-transaction effect
```

Also test:

```text
attacker ix → OOB → later error
```

Full rollback vs partial account modifications persisting changes impact classification sharply.

---

## 15. One-sentence characterization (review-safe)

> **Confirmed:** both DynamicTickArray loader implementations construct fixed-size views whose claimed extent can exceed the actual account-data allocation; the Anchor path additionally has a structural 8-byte mismatch because callers remove the discriminator before casting. **Measured:** using the production cast/rotation logic and production resize ordering in a host canary harness, `rotate_left/right(112)` modifies bytes beyond the actual account-data boundary, with modifications spanning **9,752** bytes observed in the first-tick-sized production state (`N_at_rotate = 260`) and **8** bytes for a full-size Anchor account. **Not yet established:** whether those invalid accesses produce persistent state corruption, incorrect DEX accounting, victim-impacting failure, or another bounty-qualifying consequence under Solana’s actual execution and commit model.

---

## 16. Bottom line

**Primitive:** confirmed and production-path measured (P1).  
**Layout invariant under normal transitions:** holds (Phase B semantic).  
**OOB → committed tick corruption (tested):** not observed (Phase B pollution).

The remaining work is **not** another OOB measurement round.

Phase C+D host complete — see `08_…`. **Pivot:** `09_OOB_ENVELOPE_PIVOT.md`.

Host negatives now include: layout/pollution, shared-TA, **P2b closed under invariant**, **P5 no ref×resize**, OOB→get_tick clean.

Highest expected value next (**Phase E only**):

1. E1–E5 LiteSVM/Agave envelope + atomicity (see `#09`)  
2. Do **not** reopen economics/bitmap/raw OOB fuzz unless envelope yields a program-visible channel

---

## 17. Status checklist

| Item | Status |
|------|--------|
| P1 wording: “modifications spanning N bytes” | Done (§4) |
| `N_at_rotate` instrumented | Done (Phase A probes) |
| Harness uses real production `load_mut`/`update_tick` | Done |
| Harness fidelity table | Done (§5) |
| P2a / P2b split | Done (§6) |
| Phase A→F order | Done (§11) |
| Review-safe final characterization | Done (§15) |
| Bitmap↔layout fuzzer | Done (Phase B semantic, 0 diffs) |
| Bounded reference differential | Done (Phase B) |
| Short-account pollution → logical body | Done for suite; **0 diffs** |
| Shared-TA + control (host) | **Done** — CONTROL≡EXPERIMENTAL (`08_…`, `dta_shared_ta`) |
| Economic data-flow ranking | **Done** (`08_PHASE_C_D_DATAFLOW_SHARED_TA.md`) |
| Persistence / atomicity | Pending (Phase E runtime) |
| Targeted SVM envelope | Pending (Phase E) |
