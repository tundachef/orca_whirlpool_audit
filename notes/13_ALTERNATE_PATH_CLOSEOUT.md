# 13 — Alternate-path closeout (final structured pass)

**Date:** 2026-08-19  
**Question expanded from:** “Does every `rotate_*` caller serialize afterward?”  
**To:** “Can the unsafe memory view produce an observable effect **without** going through normal repair?”

**Result:** No interesting/critical alternate path found. Investigation closed.

---

## Priority 1 — Between `rotate` and `serialize` (#1 question)

### Anchor `DynamicTickArrayLoader::update_tick`

```text
bounds checks / byte_offset / deserialize     ← before rotate; can `?` (no rotate yet)
rotate_right / rotate_left                    ← infallible
update_tick_bitmap                            ← infallible; header-only write (bytes 36..52)
serialize(...)?                               ← only fallible op after rotate
```

| Between rotate and serialize? | Present? |
|-------------------------------|----------|
| CPI | **No** |
| External/program-visible read of tick payload | **No** |
| Fallible `?` before serialize | **No** (bitmap is infallible) |
| Fallible after rotate | **Only serialize `?`** |

If `serialize` returns `Err`, the **entire instruction** fails and Solana rolls back all account mutations for that ix (including the rotate). There is no window for another instruction to observe the intermediate state mid-`update_tick`.

### Pinocchio `MemoryMappedDynamicTickArray::update_tick`

```text
rotate → update_tick_bitmap → write tag 0  OR  MemoryMappedTick::update
```

**No `?` after rotate at all.** Pure in-memory field writes.

### Caller `sync_modify_liquidity_values`

May call `update_tick` twice (lower, then upper; or twice on same array). Each call is a full rotate→bitmap→serialize unit. Failure of the second call aborts the **whole instruction**, rolling back the first. Still no CPI between rotate and serialize inside a single `update_tick`.

**P1 verdict:** `REPAIRED` / no observe-before-repair window. **Closed.**

---

## Priority 1b — Does serialize overwrite the rotate-affected logical region?

| Transition | Rotate | Write after | Coverage |
|------------|--------|-------------|----------|
| U→I | `rotate_right(112)` at `byte_offset` | serialize **113** bytes at `byte_offset` | Covers the 112 pulled-in bytes at the hole |
| I→U | `rotate_left(112)` at `byte_offset` | write **1** byte tag (+ later `resize(-112)`) | Hole tag repaired; tail trimmed by shrink; packed remainder is shifted prior content |

Empirical: full production-like / pollution / shared-TA suites → **Δref = 0**.  
Static + empirical together: repair covers the logical account that remains after the transition.

---

## Priority 2 — Every direct access to the unsafe object

| Access | Location | Bounds |
|--------|----------|--------|
| `self.0[0..4]` / `[4..36]` / `[36..52]` | `initialize`, `start_tick_index`, `whirlpool`, `tick_bitmap`, `update_tick_bitmap` | **Header only** — inside any account ≥ `MIN_LEN` |
| `tick_data()` / `tick_data_mut()` → `&self.0[52..]` | open-ended to claimed MAX | Used only with `byte_offset`-derived slices |
| `ticks_data[off..off+113]` | `get_tick` / pre-rotate deserialize | P2a form; P2b Init OOB read **closed** under invariant |
| `ticks[off..].rotate_*` | `update_tick` | **P1** — only unbounded write; always followed by serialize/tag write |
| Pinocchio `ticks[off]` / `ticks[off..off+113]` | get/update | Init path only when tag≠0 ⇒ under invariant window ⊆ L |
| `get_next_init_tick_index` | bitmap only | **No tick_data access** |

No additional attacker-controlled direct index into the fake MAX region beyond `byte_offset` (bitmap-derived) was found.

**P2 verdict:** No second write primitive; reads classified. **Closed.**

---

## Priority 3 — Instruction-centric DTA table

| Instruction | Loads DTA? | Mut? | Rotates? | Reads payload? | Serializes / repairs? |
|-------------|------------|------|----------|----------------|------------------------|
| `initialize_dynamic_tick_array` | yes `load_mut` | yes | **no** | **no** (header only: start + whirlpool) | n/a — SAFE |
| `increase_liquidity` (+v2, by_token_amounts) | yes | yes | yes (U→I) | yes | yes — REPAIRED |
| `decrease_liquidity` (+v2) | yes | yes | yes (I→U) | yes | yes — REPAIRED |
| `reposition_liquidity_v2` | yes | yes | yes (both) | yes | yes — REPAIRED |
| `update_fees_and_rewards` | yes `load` | no | no | yes `get_tick` | n/a — read; P2b closed |
| `swap` / `swap_v2` / `two_hop_swap*` | yes (sparse) | yes | **no** | yes | in-place fee-growth on **already Init** only |
| `collect_*` / `close_position*` | position/vault focused | — | no DTA rotate | — | — |
| `migrate_repurpose_reward_authority_space` | Whirlpool only | — | **no DTA** | — | — |
| `idl_include` | IDL stub only | — | no | — | — |

**Swap recheck:** no `Initialized ↔ Uninitialized` transition in `swap_manager` (no `initialized: false` / deinit path). Sparse `update_tick` only flips fee/reward outside fields inside an existing 113-byte record.

**Initialize recheck:** `initialize()` writes only `start_tick_index` and `whirlpool` (body offsets 0..36). Does **not** clear or touch `tick_data`. Account created at `MIN_LEN` with zeros — header writes in-bounds.

**P3 verdict:** No missed instruction-level rotate-without-repair route. **Closed.**

---

## Priority 4 — Account lifecycle / migration

| Pattern | Found for DTA? | Notes |
|---------|----------------|-------|
| `close` DTA account | **No** dedicated close ix | TA accounts persist; positions close separately |
| `reinitialize` / `init_if_needed` on DTA | Explicitly **avoided** (`initialize_dynamic_tick_array` comment) | Manual create + discriminator |
| Migration of DTA layout | **No** DTA migration ix | Only Whirlpool reward-authority migration |
| Unexpected size via upgrade | Not via program migration path | Attacker still cannot freely set `data_len` |

**P4 verdict:** No alternate lifetime path that pairs unsafe loader with unexpected layout. **Closed.**

---

## Priority 5 — Composability / CPI into Whirlpool

Whirlpool can be CPI’d by other programs, but they still execute the **same** instruction handlers above. No alternate bytecode path. Unusual but **valid** account sizes still go through the same `update_tick` repair. Fabricated impossible lengths remain out of scope.

**P5 verdict:** No meaningful composability bypass of repair. **Closed.**

---

## Call-graph classification (final)

```text
SAFE:
  load → header read / get_next_init (bitmap) / get_tick (P2b closed)
  load_mut → initialize (header only)

REPAIRED:
  load_mut → update_tick → [rotate?] → bitmap → serialize/tag write
  (all increase/decrease/reposition paths)

INTERESTING:   (none found)
  load_mut → rotate → [observe/CPI/return] → …

CRITICAL:      (none found)
  load_mut → rotate → read/use externally → return without repair
```

---

## Endpoint statement (after alternate-path pass)

> The OOB primitive exists. All reachable production loader/instruction paths that perform `rotate_*` complete bitmap + serialize/tag write with **no CPI or external observe between rotate and repair**. Serialize/`?` failure aborts the instruction (full rollback). Read-only and swap paths do not rotate. Initialize touches only the in-bounds header. Under a LiteSVM FeatureSet reconstructed from the currently activated mainnet feature accounts, the full production-like transition yields **final account == bounded reference**.

**Therefore:** inability to produce a theft/DoS PoC is explained by the program’s state machine, not by insufficient creativity.

**Stop.** Do not reopen host fuzz, economics, or E4 without a newly identified INTERESTING/CRITICAL path or a final-state ≠ reference result under an exact mainnet/Agave execution.
