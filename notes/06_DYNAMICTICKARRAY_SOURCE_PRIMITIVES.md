# 06 — DynamicTickArray: source-first primitives (corrected framing)

**Date:** 2026-08-18  
**Pin:** `e5f089b` (also Immutable lineage)  
**Stance:** Agree with the correction below — **Agave is for impact analysis, not bug discovery.**

---

## 0. Framing correction (accepted)

| Wrong / loose wording | Correct wording |
|----------------------|-----------------|
| “Fake buffer allocated” | **One real memory region**; the **type lies about its extent** (`[u8; MAX_LEN]` over a shorter slice) |
| “Need Agave to establish the bug” | **Bug is established in source alone** (length discarded by unsafe cast) |
| “Neighbor smash debunked forever” | **Not observed under tested stock Agave/LiteSVM**; pad blocked cross-account smash **in that config** — not a universal proof |
| Agave investigation = finding the bug | Agave = asking whether UB becomes a **security-impacting primitive** |

Core invariant violation (source-only):

```text
&mut [u8]  { ptr, len = N }
        ↓ unsafe cast
&mut [u8; MAX_LEN]   // claims MAX_LEN, ignores N
```

---

## 1. Exact constants (computed)

| Symbol | Value |
|--------|------:|
| Discriminator | 8 |
| Body header (`start`+`whirlpool`+`bitmap`) | 52 |
| `TICK_DATA_OFFSET` (in loader / body coords) | **52** |
| `TICK_ARRAY_SIZE` | 88 |
| Uninitialized tick | 1 |
| Initialized tick | 113 (= 1+112) |
| `MIN_LEN` (full account) | **148** |
| `MAX_LEN` (full account) | **10004** |
| Min body (`data[8..]`) | **140** |
| Max body | **9996** |

**Structural overclaim even on a full account (Anchor path):**

```text
load_mut(&mut data[8..])   // len = account_len - 8 ≤ 9996
type claims MAX_LEN = 10004
⇒ minimum overclaim = 8 bytes  (always)
```

**Short (empty) account overclaim:**

```text
body len = 140
claimed  = 10004
overclaim = 9864 bytes
```

---

## 2. Two loader implementations (same class of bug)

### 2.1 Anchor / classic path — `DynamicTickArrayLoader`

```rust
pub struct DynamicTickArrayLoader([u8; DynamicTickArray::MAX_LEN]); // 10004

pub fn load_mut(data: &mut [u8]) -> &mut DynamicTickArrayLoader {
    unsafe { &mut *(data.as_mut_ptr() as *mut DynamicTickArrayLoader) }
}
```

Callers always pass **`data[8..]`** (`tick_array.rs`, `initialize_dynamic_tick_array.rs`).

### 2.2 Pinocchio path — `MemoryMappedDynamicTickArray`

```rust
#[repr(C)]
pub struct MemoryMappedDynamicTickArray {
    discriminator: [u8; 8],
    start_tick_index: BytesI32,
    whirlpool: Pubkey,
    tick_bitmap: BytesU128,
    ticks: [u8; 113 * 88], // 9944
}
// total = 10004 = MAX_LEN
```

Casts **full account pointer** (includes disc):

```rust
unsafe { &mut *(data.as_mut_ptr() as *mut MemoryMappedDynamicTickArray) }
```

Same size lie when `account.data_len() < MAX_LEN`. Full-size accounts match here; short accounts still overclaim by `MAX_LEN - actual_len`.

Both paths implement `rotate_right/left(112)` on `ticks[byte_offset..]`.

---

## 3. Method inventory — every access to the claimed region

Offsets below are **loader-local** (`self.0[...]` for Anchor loader; body coords after disc skip).

| Function | R/W | Indexing | Range op | Touches past real `N`? | Notes |
|----------|-----|----------|----------|------------------------|-------|
| `load` / `load_mut` | create ref | — | — | **Yes (by type)** | Discards `len` |
| `initialize` | W | fixed 0..4, 4..36 | — | No if body ≥ 36 | Header only |
| `start_tick_index` | R | 0..4 | — | No if body ≥ 4 | |
| `whirlpool` | R | 4..36 | — | No if body ≥ 36 | |
| `tick_bitmap` / `update_tick_bitmap` | R/W | 36..52 | — | No if body ≥ 52 | |
| `tick_data` / `tick_data_mut` | R/W | **`52..MAX_LEN`** | open end | **Yes** | End = claimed MAX, not real body end |
| `byte_offset` | R | via bitmap | — | No | Arithmetic only |
| `get_tick` | R | `tick_data[byte_offset .. +113]` | — | **If `byte_offset+113 > real_tick_region`** | Deserializes 113 bytes even for uninit (tag in first byte) |
| `get_next_init_tick_index` | R | bitmap only | — | No | |
| `update_tick` (read phase) | R | same as `get_tick` | — | same | |
| `update_tick` → **`rotate_right(112)`** | R+W | `tick_data_mut()[byte_offset..]` | **whole tail** | **HIGH** | Tail length = `claimed_tick_len - byte_offset` |
| `update_tick` → **`rotate_left(112)`** | R+W | same | **whole tail** | **HIGH** | |
| `update_tick` → serialize | W | `byte_offset .. +1 or +113` | — | Depends | After rotate |

**Unsafe sites in this module:** only the two casts in `load` / `load_mut` (plus Pinocchio tick pointer casts in `get_tick`/`update_tick`). No other `ptr::` / `from_raw_parts` in the Anchor loader file. Secondary unsafety is **inherited**: safe Rust ops on a falsely sized reference.

---

## 4. Call graph (production)

```text
load(&data[8..])                          [READ path]
├── load_tick_array()                     state/tick_array.rs
│   ├── update_fees_and_rewards
│   ├── swap / two-hop (via tick sequence)
│   └── various reads of get_tick / get_next_init_tick_index
│
load_mut(&mut data[8..])                  [WRITE path]
├── initialize_dynamic_tick_array         header init only
├── load_tick_array_mut()
│   └── TickArraysMut::load_position_tick_arrays
│         └── sync_modify_liquidity_values  manager/liquidity_manager.rs
│               └── update_tick(lower)
│               └── update_tick(upper)   ← may rotate_left/right
│
Pinocchio twin:
├── pinocchio/.../tick_array/loader.rs    cast full account → MemoryMapped*
└── pinocchio/.../manager_liquidity_manager.rs
      └── update_tick → rotate_*
```

**Trigger for rotate (attacker-relevant):**

```text
increase/decrease liquidity (v1/v2/pinocchio)
        ↓
tick flips Uninitialized ↔ Initialized
        ↓
update_tick()
        ↓
rotate_right(112)  or  rotate_left(112)
```

Resize (+/−112) is scheduled around these ops (`manager_tick_array_manager` / pinocchio port). Order matters for *how short* the account is at rotate time, but **does not remove** the type-level size lie.

---

## 5. Exact OOB envelopes (Anchor path, rotate from `byte_offset`)

Claimed tick region length always:

```text
claimed_tick_len = MAX_LEN - TICK_DATA_OFFSET = 9952
```

Real tick region for `k` initialized ticks:

```text
real_tick_len = 1*k_uninit_equiv = 113*k + 1*(88-k) = 88 + 112*k
```

| Initialized ticks `k` | Account body | Real tick bytes | Claimed tick bytes | **OOB beyond real tick end** (rotate from 0) |
|----------------------:|-------------:|----------------:|-------------------:|---------------------------------------------:|
| 0 (min) | 140 | 88 | 9952 | **9864** |
| 1 (after +112 resize) | 252 | 200 | 9952 | **9752** |
| 10 | 1260 | 1208 | 9952 | **8744** |
| 44 | 5068 | 5016 | 9952 | **4936** |
| 87 | 9884 | 9832 | 9952 | **120** |
| 88 (full body) | 9996 | 9944 | 9952 | **8** |

General:

```text
rotate_range = [TICK_DATA_OFFSET + byte_offset , MAX_LEN)
OOB_length   = (MAX_LEN - TICK_DATA_OFFSET - byte_offset)
               - max(0, real_tick_len - byte_offset)
```

For init of first empty tick at offset 0 on min account: OOB ≈ **9864** bytes of “typed” write/read via `rotate_right`.

**Full-account case still broken:** overclaim **8** bytes past body end (disc mismatch). Structural, not exotic.

---

## 6. Attacker control of `byte_offset`

```text
position tick_lower / tick_upper  (user chooses when opening LP)
        ↓
tick_index ∈ [start, start + 88*spacing)
        ↓
tick_offset = (tick_index - start) / spacing   ∈ 0..87
        ↓
byte_offset = (#init below)*113 + (#uninit below)*1
```

So the attacker (as LP) influences **where** the rotate starts inside the tick blob, but **not** the claimed end (`MAX_LEN`). They do **not** freely choose an arbitrary absolute address — only a tick slot in an array they can write via liquidity ops.

---

## 7. Ranked primitives (source characterization)

### P0 — Type-level size lie (always)

```text
Primitive: Any load(_mut) of DynamicTickArray treats body as MAX_LEN bytes
           even when actual body length is smaller (min overclaim 8).
Impact class: UB / incorrect object extent. Established without Agave.
```

### P1 — Bulk R+W via rotate (highest interest)

```text
Primitive: On Uninit→Init (or reverse), program executes
           rotate_{right,left}(112) on shift_data =
             &mut claimed_tick_region[byte_offset..]
           Length of that slice is (9952 - byte_offset) in the type's view,
           while real remaining tick bytes may be far smaller.
Trigger:   increase/decrease liquidity that flips tick initialized bit.
Control:   byte_offset via which tick flips; not arbitrary.
```

### P2 — Fixed-width OOB read in `get_tick` / update read phase

```text
Primitive: Always reads INITIALIZED_LEN (113) bytes from byte_offset
           even if tick is uninitialized (only first byte is the tag).
           If byte_offset+113 exceeds real account end → OOB read.
Note:      On a correctly sized packed layout, uninit ticks only need 1 byte
           in the packed stream; reading 113 may already span into later
           packed ticks OR past end if near the real end / inconsistent bitmap.
```

### P3 — Economic / accounting branch (needs proving, not assumed)

```text
Question: Can OOB read/write change values that later feed
          liquidity_net, fee_growth_*, swap crossing, amounts?
If yes → bounty-shaped impact without neighbor smash.
If OOB only hits padding that is discarded on commit → weaker.
```

### P4 — Runtime failure (DoS)

```text
Primitive: Under some SVM feature sets, large OOB store patterns
           → InvalidRealloc / trap → liquidity ix fails.
Impact:    Temporary inability to modify liquidity on affected dynamic TAs.
```

### Demoted for now

```text
Neighbor-account smash / vault overwrite:
  Not observed in tested Agave/LiteSVM layout (pad wall).
  Do not treat as primary research goal next.
```

---

## 8. What to do next (source-first, per your tree)

1. **Prove P1 trigger end-to-end in-process** (no Agave): unit harness with real `account_len = MIN_LEN`, call `update_tick` Uninit→Init, instrument which offsets of the **backing** slice are written (ASan/MIRI/custom canary beyond `len`).  
2. **Inventory bitmap inconsistency:** can `get_tick`’s 113-byte read cross into “unallocated” packed region when bitmap and length disagree?  
3. **Trace one flipped tick into swap math:** after a rotate that touched past-end canaries, do `liquidity_net` / fee fields used in swap differ from a correct resize-first path?  
4. **Only then** return to SVM: “what sits in the OOB envelope?” — padding vs anything security-relevant — as **impact**, not discovery.

---

## 9. One-line primitive statement

> **An LP-driven tick initialize/uninitialize can cause Whirlpool to `rotate_{left,right}(112)` on a slice whose end is defined by a falsely claimed `MAX_LEN` object rather than the account’s true length, discarding the `&mut [u8]` length invariant; minimum permanent overclaim is 8 bytes, and short dynamic arrays claim up to ~9864 bytes past the real tick region.**

That is the bug. Agave answers whether that becomes theft, freeze, DoS, or “nothing useful” — it does not create the invariant violation.
