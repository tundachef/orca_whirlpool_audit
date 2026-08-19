# 11 — Final all-callers audit + endpoint finding

**Date:** 2026-08-19  
**Depends on:** `#10` binary fork (mainnet FeatureSet → SUCCESS + Δref=0)

---

## Objective change

**Was:** Can we turn this OOB into an exploit?  
**Now:** Can any realistic path escape Whirlpool’s repair into persistent / security-relevant state?

Normal production-like path answer: **No** (under LiteSVM FeatureSet from 301 activated mainnet features).

E4 / economics: **N/A for the demonstrated path** (nothing wrong to persist).

---

## Final all-callers audit (unsafe DynamicTickArray loaders)

### Primitives

| Primitive | File | Cast | Notes |
|-----------|------|------|-------|
| `DynamicTickArrayLoader::load/load_mut` | `state/dynamic_tick_array.rs` | `&[u8]` → `[u8; MAX_LEN]` | Length discarded |
| Anchor loaders | `state/tick_array.rs` | **`data[8..]`** | Permanent ≥8 overclaim |
| Pinocchio cast | `pinocchio/.../loader.rs` | full `data` → `MemoryMappedDynamicTickArray` | Short-account overclaim only |

### Production callers — write paths that can `rotate_*`

| Caller | Ordering | After rotate | Escapes repair? |
|--------|----------|--------------|-----------------|
| Anchor/Pinocchio **increase*** | drop → **resize(+112)** → reload → `update_tick` | bitmap + **serialize** always | **No** (serialize writes Init/Uninit into hole) |
| Anchor/Pinocchio **decrease*** | `update_tick` → drop → **resize(−112)** | bitmap + **serialize** then shrink | **No** |
| Pinocchio **reposition** (inc/dec halves) | same as above | same | **No** |

\* Includes v1/v2 / by_token_amounts variants — all funnel to `sync_modify_liquidity_values` → `update_tick`.

`update_tick` always ends with:

```text
rotate? → bitmap? → DynamicTick::serialize(...)
```

There is **no** production write path that performs `rotate_*` and returns without serialize.

### Production callers — read-only / no rotate

| Caller | Ops | OOB write? | Notes |
|--------|-----|------------|-------|
| `get_tick` / `calculate_modify_liquidity` / fees | form 113-slice + Borsh | No | P2b **closed** under invariant (Uninit reads 1) |
| Swap sparse `update_tick` | fee-growth flip on **already Init** tick | No rotate | In-bounds field writes only |
| `initialize_dynamic_tick_array` | `load_mut` + header init | No rotate | Header-only |

### Conclusion of callers audit

No remaining production caller was found with:

```text
OOB rotate → READ / return
```

without intervening serialize (or without rotate at all). The tested repair conclusion applies to **all production rotate sites**.

---

## Endpoint characterization

### Confirmed

```text
unsafe fixed-size cast
  → invalid memory extent
  → rotate beyond account boundary
  → runtime-visible intermediate effect (rotate-only; attributable)
```

### Demonstrated not to happen (tested path)

```text
OOB → full production-like update → persistent wrong tick state
```

Under LiteSVM FeatureSet reconstructed from **301 currently activated mainnet feature accounts**, mode6 (cast→rotate→bitmap→serialize) at `N_at_rotate=260`:

```text
tx OK
final body == bounded reference  (Δ = 0)
victim Δ = 0
```

### Not demonstrated

- Persistent corruption  
- Victim economic loss  
- Victim DoS  
- Cross-account smash (not observed)

### Closed side tracks

| Item | Status |
|------|--------|
| P2b | Closed under layout invariant |
| P5 | Closed (no ref×resize) |
| Bitmap fuzz | Negative |
| Shared-TA host extra effect | Negative |
| Neighbor smash | Not observed |
| E4 | **N/A** for demonstrated path |

---

## Wording constraints (review-safe)

- Say: **LiteSVM FeatureSet reconstructed from currently activated mainnet feature accounts** — not bare “under mainnet.”  
- Say: **production-like** transition (mode6) unless full Whirlpool SBF ix is executed unchanged.  
- Agave parity: validation step, not an open exploit hypothesis.  
- Do **not** reopen economics without a final-state ≠ reference result.

---

## Final status table

| Finding | Final status |
|---------|--------------|
| Unsafe fixed-size mapping | **Confirmed** |
| OOB write | **Confirmed** |
| Runtime-visible intermediate OOB effect | **Confirmed** (rotate-only; mainnet-activated FeatureSet / historical LiteSVM) |
| Final DEX-state corruption | **Not demonstrated** (production-like path Δref=0) |
| Cross-account corruption | **Not observed** |
| Persistent corruption | **Not demonstrated** |
| Economic manipulation | **Not demonstrated** |
| Victim DoS | **Not demonstrated** |
| P2b | **Closed by layout invariant** |
| P5 | **Closed** |
| Mainnet FeatureSet in LiteSVM | **Reproduced (301 gates)** |
| Real Agave parity | **Attempted; blocked on this host** (`solana-test-validator` 4.2.0: *Incompatible CPU detected: missing AVX support*). Not an open exploit hypothesis — optional validation on AVX hardware. |

---

## Recommendation

**Stop hunting for an exploit** on the normal DynamicTickArray transition.  
Optional: Agave live parity only.  
Write the finding as a **memory-safety vulnerability with no demonstrated fund-loss / victim-DoS impact**, explaining why the obvious exploit fails (serialize repair).
