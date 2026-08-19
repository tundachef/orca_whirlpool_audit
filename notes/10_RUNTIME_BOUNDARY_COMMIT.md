# 10 — Runtime Boundary & Commit Matrix

**Date:** 2026-08-19  
**Phase name:** Runtime boundary and transaction-observability (not “envelope as assumed useful region”)  
**Depends on:** `#09` host tree freeze  
**Harness:** `audit_work/or_h01_runtime/tests/or_h01_litesvm` (`phase_e` bin)  
**Log:** `orca-whirlpool/fuzz/logs/phase_e_litesvm.txt`

---

## Host tree — FROZEN

| Finding | Status | Action |
|---------|--------|--------|
| **P0** unsafe fixed-size mapping | Confirmed | Root cause |
| **P1** OOB modification | Confirmed | Core bug |
| P2a oversized slice formation | Confirmed | Supporting |
| **P2b** Initialized OOB read | **Closed under production layout invariant** | Reopen only if invariant breaks |
| **P3** economic corruption | **Blocked** | Reopen only if program-visible channel |
| P4 DoS | Weak | Don’t prioritize |
| **P5** ref × resize | **Closed** | Stop |
| Neighbor smash | Low | Late, runtime-only |
| **E-runtime** | **Open / decisive** | Current focus |

```text
Memory-safety primitive
        ≠
Program-visible primitive
        ≠
Persistent corruption
        ≠
Economic exploit
        ≠
Fund theft
```

**P1 framing (review-safe):** A fixed-size 10,004-byte view is constructed over an account whose body may be only 252 bytes at `rotate_*` (`N_at_rotate=260` with disc); the production-path host harness detects modifications across the resulting **9,752-byte beyond-boundary span**. That span is dirty canary bytes, not automatically corrupted application state.

---

## Observability ladder

| Level | Question |
|------:|----------|
| 0 | OOB exists? → **yes** (host) |
| 1 | Does SVM execute it? |
| 2 | Same-instruction program-visible read? |
| 3 | Cross-instruction? |
| 4 | Cross-transaction persistence? |
| 5 | Security-relevant account/program state? |
| 6 | Economic effect? |
| 7 | Attacker profit? |

Stop ascending when the level fails.

---

## Decision tree (E1 gate)

```text
Production P1 @ N_at_rotate=260
            │
            ▼
     LiteSVM / Agave
            │
     ┌──────┴──────┐
   TRAP         SUCCEEDS
     │              │
     ▼              ▼
 containment   classify boundary
                    │
             discarded vs observable
                    │
              write→read (E3.1–E3.3)
                    │
              persistence? (E4)
                    │
              failure survival? (E5/E6)
```

---

## E0 — Harness fidelity

| Item | Value |
|------|-------|
| Program | `or_h01_poc.so` mode **3** = production `data[8..]` → `[u8; MAX_LEN=10004]` + `rotate_right(112)` on tick region |
| Account length | **260** (`MIN_LEN+112`) = production `N_at_rotate` for first-tick increase |
| Body length | 252 |
| Claimed loader | 10004 |
| Beyond-boundary span | 10004 − 252 = **9752** |
| Not used as impact evidence | old short-**148** PoC / wrong resize order |

---

## E0.5 — Deployed mainnet feature profile (2026-08-19)

| Probe | Result |
|-------|--------|
| `getVersion` mainnet-beta | `solana-core` **4.2.0**, feature-set hash `565236538`, epoch **1019** |
| Agave 4.2 activations | Targeted start week of Aug 17; public tracker still showed many 4.2 gates pending as of ~Aug 17–18 |
| Feature `account_data_direct_mapping` `CR3dVN2…` | **Account null** on mainnet RPC → **not activated** |
| Feature `stricter_abi_and_runtime_constraints` (SIMD-0219) `CxeBn9P…` | **Account null** → **not activated** |
| LiteSVM `historical` / `all_features` | **Not** automatic mainnet proxies |

**`InvalidRealloc` cause (provisional):** In program-runtime serialization, `InvalidRealloc` is returned when deserialized `post_len` exceeds permitted growth/length (`serialization.rs`), i.e. often at **account commit after the instruction returns**, not necessarily an explicit `realloc` syscall. Logs show `BEFORE rotate` then failure under `all_features` — consistent with OOB stores into the input buffer’s realloc/metadata region corrupting commit bookkeeping. **Exact instruction-event attribution still needs mode5/6 rebuild + finer logs** (SBF toolchain currently blocked on edition-2024 metadata).

**Implication:** Do **not** treat LiteSVM historical success as mainnet success, nor `all_features` trap as mainnet trap, until a custom FeatureSet matching activated mainnet gates is constructed and re-run.

---

## Definitive source boundary table

| Operation | Claimed extent | Min reachable actual (prod) | OOB? | R/W | Runtime relevance |
|-----------|---------------:|----------------------------:|------|-----|-------------------|
| `DynamicTickArrayLoader` cast | 10004 | body 140 (MIN) … 9996 (full) | **yes** (≥8 always on Anchor) | object | root (P0) |
| `tick_data_mut()[off..].rotate_*` | claimed open-ended ≤9952 | **Minimum actual tick-data region at N=260: 200 B** (operation still claims through MAX view) | **yes** | **W** | **P1** |
| `get_tick` +113 slice | ≤9952 | packed L | L2 form / P2b **closed** for Init | R | supporting |
| bitmap R/W | header 16B | in-account | no | R/W | safe |
| serialize into hole | 1 or 113 | in packed region | no (repair) | W | neutralizes hole |
| Pinocchio full cast | 10004 | account_len | short only | object | Finding B |

---

## E1 — SVM access result @ N=260

### Setup

- LiteSVM historical features vs all_features  
- Account data length **260**, mode **3** (production `MAX_LEN` cast)  
- Contiguous victim account present (neighbor observation only)  
- Body pre-filled with uniform `0xCC` so an identity rotate would change **0** in-bounds bytes unless past-boundary contents are pulled in

### Expected

Unknown a priori — gate for rest of matrix.

### Observed (`fuzz/logs/phase_e_litesvm.txt`, 2026-08-19)

| Feature set | Mode | account_len | Tx result | tickΔ bytes | victimΔ | Classification |
|-------------|------|------------:|-----------|------------:|--------:|----------------|
| historical | **3** | **260** | **OK** | **112** | 0 | **SUCCESS_NO_CORRUPT** |
| all_features | **3** | **260** | **InvalidRealloc** | 0 | 0 | **TRAP** |
| historical | 0 | 260 | OK | 112 | 0 | SUCCESS_NO_CORRUPT |
| all_features | 0 | 260 | InvalidRealloc | 0 | 0 | TRAP |
| historical | 3 | 10004 | OK | **8** | 0 | SUCCESS_NO_CORRUPT |
| all_features | 3 | 10004 | InvalidRealloc | 0 | 0 | TRAP |
| historical | 3 | 148 | OK | 88 | 0 | SUCCESS_NO_CORRUPT (legacy; not N_at_rotate) |
| all_features | 3 | 148 | InvalidRealloc | 0 | 0 | TRAP |

### Interpretation

1. **E1 is split by feature set**, not absolute:
   - **Stricter / all_features:** Level 1 **TRAP** (`InvalidRealloc`) — runtime containment; no committed account mutation.
   - **Historical features:** Level 1 **SUCCEEDS**.

2. **Neighbor smash not observed** in any N=260 primary case (victimΔ=0).

3. **Post-execution account-data mutation (Level 2a — provisional):**  
   Under historical features, `tickΔ=112` at N=260 (and `tickΔ=8` on full Anchor +8) is **consistent with** past-boundary bytes being incorporated by `rotate_*`.  
   This is a **strong inference**, not yet isolated attribution: production `update_tick` also updates bitmap and serializes.  
   Correct wording: *post-execution account-data mutation observed*; **not** yet “committed ledger state” (E4) and **not** yet proven OOB-value pull-in without E3.1a arms.

4. **Next gates (order):**  
   - **E0.5** exact mainnet feature profile (not “historical ≈ mainnet”).  
   - **E3.1a–d** isolate rotate vs full `update_tick` vs bounded reference.  
   - Then E3.2 / E4 / E5 / Agave as needed.

---

## E2 — Boundary classification

Ask, not “name the region”:

| ID | Question | Status @ N=260 |
|----|----------|----------------|
| **E2A** | Runtime-enforced boundary? | **Yes** under all_features (`InvalidRealloc`). **No** under historical (access permitted). |
| **E2B** | Same-instruction contents observable in account data? | **Yes** under historical (`tickΔ` matches rotate window; uniform fill ⇒ inbound from past boundary). |
| **E2C** | Other account affected? | **Not observed** (victimΔ=0). |
| **E2D** | Persist across txs? | **Open (E4)** |

---

## E3 — Program-level read-after-write

### E3.1a–d results (host `fuzz/dta_e31` + LiteSVM gradient attribution)

| Arm | What | Result |
|-----|------|--------|
| **E3.1a** rotate-only, zero OOB vs bounded-only rotate | Attribution | **Δ=112** — past-boundary zeros enter account |
| **E3.1a** rotate-only, canary OOB vs zero OOB | Attribution | **Δ=112** — **OOB contents attributable** in first 112 tick bytes |
| **E3.1b/d** production `update_tick` (rotate+bitmap+serialize) vs bounded reference | Final logical body | **Δ=0 MATCH** — serialize **repairs** |
| LiteSVM historical mode3/len260 vs bounded-only rotate | Attribution under SVM | **Δ=112** (gradient fill) |

**Wording now justified:**

> The post-execution account data after **rotate-only** differs in a way that is **attributable** to past-boundary bytes (canary vs zero OOB). After the **full production `update_tick`** (including serialize), the final logical body **matches** the bounded reference under host semantics — i.e. Whirlpool’s own update sequence repairs/overwrites the hole before the instruction completes.

| Subtest | Status |
|---------|--------|
| E3.1a isolated rotation attribution | **Done — confirmed** |
| E3.1b full `update_tick` | **Done — repaired (host)** |
| E3.1c complete Whirlpool ix (resize+…) under exact mainnet FeatureSet | Open (needs E0.5 custom set + preferably SBF rebuild) |
| E3.2 same tx later ix | Open |
| E3.3 next tx | Open |

Host raw pointer R/W is not the bar; **attributable account-data delta** is. Persistence across txs remains E4.

---

## E4 — Cross-transaction persistence

Only if E3 interesting.

---

## E5 / E6 — Abort / rollback

Realistic failures after OOB; inspect post-abort account state.

---

## E7 — Agave vs LiteSVM

Repeat decisive cases.

---

## E8 — Conclusion (binary fork resolved under LiteSVM + mainnet FeatureSet)

### E0.5 complete (2026-08-19)

- Dumped **301 activated** Feature program accounts from mainnet-beta (`fuzz/logs/mainnet_activated_features.json`).
- Built LiteSVM `FeatureSet` by activating each at its recorded slot.
- Confirmed on that set: `stricter_abi` **false**, `account_data_direct_mapping` **false**.
- `all_enabled` remains a **non-mainnet** super-set (still traps with `InvalidRealloc`).

### Full production-like transition @ N=260 (mode 6)

```text
cast MAX_LEN on body
  → rotate_right(112)
  → bitmap bit0
  → serialize Initialized (tag=1 + 0x11×112)
  → final account bytes
```

| FeatureSet | Tx | Δ vs pre | **Δ vs bounded reference** | Victim | Fork |
|------------|-----|--------:|---------------------------:|-------:|------|
| **mainnet_activated (301)** | **OK** | 114 | **0** | 0 | **STOP** |
| historical_default | OK | 114 | **0** | 0 | STOP |
| all_enabled | InvalidRealloc | 0 (rollback) | n/a | 0 | containment (not mainnet) |

Log: `fuzz/logs/phase_e_mainnet.txt` (`phase_e_mainnet` bin).

### Preserved distinctions

- **Rotate-only** under historical/mainnet-like configs: attributable OOB effect (E3.1a canary) — runtime-visible intermediate mutation.
- **Full `update_tick`-like path**: final body **==** bounded reference — Whirlpool repair/serialize neutralizes lasting logical corruption under the tested mainnet FeatureSet.
- **`tickΔ=112` rotate-only ≠ persistent DEX-state corruption.**

```text
P1 OOB WRITE
     │
     ▼
exact mainnet FeatureSet (301 gates)
     │
     ▼
SUCCESS + full update_tick-like
     │
     ▼
final account == bounded reference
     │
     ▼
STOP — no E4 / no economics
```

### Endpoint statement (review-safe)

> **Confirmed:** unsafe fixed-size DynamicTickArray mapping enables a production-path `rotate_*` that modifies memory beyond the account-data boundary (measured; attributable under rotate-only). **Under a LiteSVM FeatureSet reconstructed from the 301 currently activated mainnet feature accounts, the full production-like transition (rotate + bitmap + serialize) at `N_at_rotate=260` succeeds and the final account body matches a bounded reference byte-for-byte.** Neighbor accounts were unaffected. **Bounty-level impact (persistent corruption, economic diversion, victim DoS) is not demonstrated.** Agave live parity remains an optional validation step (blocked here by missing AVX), not an open exploit hypothesis. Do not reopen host fuzz or economics without a final-state ≠ reference result.

**Agave parity attempt (2026-08-19):** `solana-test-validator` **4.2.0** exits immediately on this machine: `Incompatible CPU detected: missing AVX support`. Script ready at `audit_work/or_h01_runtime/tests/validator_e2e/parity_n260_mode6.js` for AVX hosts. Treat as optional validation, not an open exploit branch.

**E4 / economics:** closed/N/A for the demonstrated production-like path (final == reference).
