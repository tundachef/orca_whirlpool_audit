# 07 — DynamicTickArray revised investigation (post-review)

**Date:** 2026-08-18  
**Responds to:** critical review of `06_DYNAMICTICKARRAY_SOURCE_PRIMITIVES.md`  
**Canary log:** `fuzz/logs/dta_canary.txt`  
**Harness:** `fuzz/dta_canary/`

---

## 0. Accepted corrections

1. **Three layers** (do not collapse them):
   - **L1:** Rust reference/object extent is invalid (length discarded).
   - **L2:** Program forms slices whose claimed length exceeds the backing account allocation.
   - **L3:** An operation **actually** reads/writes those bytes (must be measured).

2. **Agave** = impact under Solana memory model, **not** bug discovery.

3. **Neighbor smash:** demoted; say “not observed under tested Agave/LiteSVM,” not “impossible.”

4. **P2 / P4:** rewritten as hypotheses until proven (below).

5. **Anchor vs Pinocchio:** related but **distinct** length invariants.

6. **OOB table:** must use **`N_at_rotate`**, not bare `k`, because **resize order differs by path**.

---

## 1. Confirmed (source alone) — L1

| Claim | Status |
|-------|--------|
| `load`/`load_mut` cast `&[u8]` → `[u8; MAX_LEN]`, discarding `len` | **Confirmed** |
| Anchor callers pass `data[8..]` while `MAX_LEN` includes disc | **Confirmed** → permanent **≥8** extent mismatch |
| Pinocchio casts full account to `MemoryMappedDynamicTickArray` sized `MAX_LEN` | **Confirmed** → short-account overclaim only (no permanent +8 when full) |

---

## 2. Exact production order (fixes the OOB table)

### Increase (Uninit → Init) — Anchor **and** Pinocchio

```text
calculate_modify_liquidity          // read-only load
drop(tick_array refs)
update_tick_array_accounts
   └── resize(+112)                 // FIRST
reload load_mut
sync_modify_liquidity_values
   └── update_tick
         └── rotate_right(112)      // SECOND
         └── update_tick_bitmap
         └── serialize tick
```

Evidence: `increase_liquidity.rs` lines 88–113; pinocchio twin same.

### Decrease (Init → Uninit) — Anchor **and** Pinocchio

```text
calculate_modify_liquidity
sync_modify_liquidity_values
   └── update_tick
         └── rotate_left(112)       // FIRST (account still LARGE)
         └── update_tick_bitmap
         └── serialize
drop(refs)
update_tick_array_accounts
   └── resize(-112)                 // SECOND
```

Evidence: `decrease_liquidity.rs` lines 60–78; pinocchio twin same.

### Implication for “empty ↔ one tick”

| Transition | `N` (account `data_len`) **at rotate** | Real body | Claimed loader | **Measured L3 past-boundary writes** |
|------------|----------------------------------------:|----------:|---------------:|-------------------------------------:|
| Increase first tick | `MIN_LEN+112 = 260` | 252 | 10004 via body cast | **9752** (canary) |
| Decrease last tick | still `260` before shrink | 252 | 10004 | **9752** (canary) |
| Full MAX + Anchor body cast | `10004` | 9996 | 10004 | **8** (canary) |

So the scary “rotate on raw `MIN_LEN=148`” does **not** match production increase/decrease: both rotate while the account physically holds the **larger** of the before/after packed sizes.  
The table in note 06 that used bare `k` without this order was **misleading**; superseded by the table above.

---

## 3. Layer-3 experiment results (host canary, no Agave)

Harness paints a **gradient** past `account_len` (uniform `0xA5` hid canary↔canary traffic).

| Experiment | Claimed `shift_data.len()` | Dirty bytes past account | Span |
|------------|---------------------------:|-------------------------:|-----:|
| `rotate_right` after +112 | 9952 | **9752** | 260…10011 |
| `rotate_left` before −112 | 9952 | **9752** | 260…10011 |
| Full account + Anchor cast | 9952 | **8** | 10004…10011 |

**Conclusion:** Under this host execution of the same cast+rotate pattern, **`rotate_*` does perform Layer-3 writes** across essentially the entire claimed-over-real gap (9752 or 8), not merely “UB on paper.”

Wording for reports:

> The loader constructs a reference whose claimed extent exceeds the account-data slice. `tick_data_mut()[byte_offset..]` therefore has a claimed end past the allocation, and instrumentation shows `rotate_*` **actually modifies** those past-boundary bytes (9752 bytes in the first-tick increase/decrease-sized case; 8 bytes in the full Anchor body-cast case).

---

## 4. P2 rewritten (was overstated)

Source:

```rust
let mut tick_data = &ticks_data[byte_offset..byte_offset + DynamicTick::INITIALIZED_LEN];
let tick = DynamicTick::deserialize(&mut tick_data)?;
```

| Layer | What happens |
|-------|----------------|
| L2 | Slice of **113** bytes is formed against **claimed** `tick_data` len (9952) |
| L3 deserialize | Borsh enum: **Uninitialized → read 1 byte**; **Initialized → read 113** |

**Do not claim** “always physically reads 113 bytes for uninitialized ticks.”  
**Do claim** “always *forms* a 113-byte slice on the claimed extent when `byte_offset+113 ≤ 9952`.”

---

## 5. P4 rewritten (DoS not established)

`InvalidRealloc` on the attacker’s own failing tx ≠ victim DoS.

Rename:

> **P4 — Runtime rejection / execution failure (victim impact not established)**

Needs: shared/persistent state such that a **later legitimate user ix** fails.

---

## 6. Finding split: Anchor vs Pinocchio

| | Anchor `DynamicTickArrayLoader` | Pinocchio `MemoryMappedDynamicTickArray` |
|--|----------------------------------|------------------------------------------|
| Cast input | `data[8..]` (body) | full `data` (with disc) |
| Full-size account | **Still +8 overclaim** | Size matches `MAX_LEN` |
| Short account | Overclaim `MAX_LEN - body_len` | Overclaim `MAX_LEN - account_len` |
| Rotate sink | `tick_data_mut()[off..].rotate_*` | `ticks[off..].rotate_*` |
| Increase/decrease order | Same (see §2) | Same |

Treat as **Finding A** and **Finding B**, same family, prove separately.

---

## 7. Representation invariants (new section)

### Invariant 1 — object extent

```text
actual_slice_len  >=  claimed_loader_len
```

**False** (Anchor always by ≥8; both false when short).

### Invariant 2 — packed layout ↔ bitmap

```text
bitmap bit i set  ⟺  i-th packed record is 113-byte Initialized
physical offsets of later ticks = sum of sizes of earlier records
```

Must hold for `byte_offset()` to address the correct record.  
**Open investigation:** can bitmap and packed bytes diverge under partial failure / OOB / resize races? That yields **wrong-tick R/W** without neighbor smash — parallel track to P1.

---

## 8. Who controls the DynamicTickArray? (precision)

| Capability | Who |
|------------|-----|
| Create/init dynamic TA | Anyone (init ix); chooses `start_tick_index` |
| Open position / pick ticks | LP (attacker can be LP) |
| Flip tick init bit on **own** position’s arrays | LP via increase/decrease |
| Force flip on **victim’s** TA | Only if sharing that TA (same pool range) via own position referencing same arrays |
| Malformed short account as program account | Init/resizes are program-driven; attacker doesn’t freely set `data_len` without going through resize logic |

**Strongest useful scenario:** attacker LP on a pool → shares tick arrays with others → triggers rotate on that shared account.  
**Weak:** attacker only breaks their own txs.

---

## 9. Revised primitive ranking

| ID | Status | Statement |
|----|--------|-----------|
| **P0** | **Confirmed L1** | Unsafe fixed-size map discards slice length |
| **P1** | **Confirmed L1+L2; L3 measured in host canary** | `rotate_*` over falsely extended tick region; **9752** past-boundary writes at first-tick sized `N=260`; **8** on full Anchor cast |
| **P2** | **Hypothesis refined** | 113-byte slice *formation* on claimed extent; deserialize read may be 1 or 113 |
| **P3** | **Unproven / highest bounty interest** | Corrupted/past-end bytes → liquidity/fee/swap accounting |
| **P4** | **Weak** | Runtime fail ≠ victim DoS yet |
| Neighbor smash | **Deprioritized** | Not observed in tested SVM config |

---

## 10. Next experiments (ruthless order)

1. ~~Prove L3 bytes touched~~ **Done (host canary).**  
2. **Bitmap ↔ packed layout differential tests** (Invariant 2).  
3. **Data-flow:** after rotate with past-end pollution, which tick fields are consumed by swap?  
4. **Shared TA:** two positions, one attacker-triggered transition, victim swap/liquidity.  
5. **SVM only then:** what occupies the 9752-byte envelope on Agave (padding vs useful)?  
6. Victim-impact for any runtime rejection.

---

## 11. One-sentence characterization (review-safe)

> **Confirmed:** unsafe fixed-size mapping of dynamically sized DynamicTickArray account data discards the slice length (Anchor also has a structural 8-byte extent mismatch). **Measured:** under a host recreation of the production cast + `rotate_*` pattern, `rotate_right/left(112)` writes thousands of bytes past the real account end (9752 in the first-tick-sized case). **Not yet shown:** that those writes yield economic theft, persistent cross-user corruption, or victim DoS under Solana’s account commit model — that is the remaining impact question, not the existence of the memory-safety bug.
