# 09 — Pivot: OOB envelope, reads, lifetime (post-#08 critique)

**Date:** 2026-08-19  
**Responds to:** critique of `#08` — original exploit tree weakened; underlying bug remains strong  
**Prior:** `07_…`, `08_PHASE_C_D_DATAFLOW_SHARED_TA.md`

---

## 0. Host exploit tree — FROZEN

Further host fuzz / P2b / P5 / economic tracing is **stopped**.  
Active work: **`10_RUNTIME_BOUNDARY_COMMIT.md`** (runtime boundary & transaction-observability).

P2b = **closed under production layout invariant**.  
P5 = **closed**.  
Central question: what does Solana’s runtime do beyond the account-data boundary?

---

## 0b. Revised status (honest)

| Area | Judgment |
|------|----------|
| Memory-safety bug | **Confirmed** |
| Actual OOB modification | **Confirmed in host, production code path** |
| 9,752-byte figure | **Credible** |
| Bitmap/packed-layout corruption | **Negative under tested transitions** — reopen only on new mutation/failure path |
| OOB → committed tick corruption | **Negative under tested transitions** |
| Shared-TA OOB adds effect | **Negative on host logical observables** (`CONTROL ≡ EXPERIMENTAL`) |
| Economic consequence | **Mapped, not demonstrated** — branch **blocked** until middle link reopens |
| DoS | **Not demonstrated** |
| Agave/SVM relevance | **Critical unknown** |
| Keep fuzzing raw OOB / economics | **No** |
| Investigate runtime + OOB **reads** + lifetimes | **Yes** |

**Strongest statement today:**

> There is a real unsafe memory operation, reachable through a normal production DynamicTickArray transition, that modifies memory beyond the account-data boundary. All tested logical-state, packed-layout, and shared-TickArray **host** experiments show that normal resize/serialize mechanics neutralize that corruption before it becomes committed DEX state. **Bounty impact: not demonstrated. Primary exploit paths tested so far are negative.** This is *not* “OOB affects nothing” — only “no additional **logical-state** effect under the host backing model.”

```text
CONTROL ≡ EXPERIMENTAL
  ⇒ no extra logical effect on host
  ≠ no runtime / SVM / transient effect
```

---

## 1. The OOB envelope (central concept)

```text
real account data
├──────────────────────┤
                      account boundary
                       ↓
                       ├───────────────────────────────┐
                       │       OOB envelope (~9752 B)  │
                       └───────────────────────────────┘
```

**Already answered:** can we write there? → **Yes** (Phase A).

**Now answer:** what can cause program execution to **read** from the envelope?

| Outcome | Meaning |
|---------|---------|
| **A** write → envelope → discarded | Probably harmless |
| **B** write → envelope → later read → program-visible | Interesting |
| **C** write → envelope → SVM metadata/object | Potentially serious |
| **D** write → envelope → neighboring account | Potentially serious; still low priority until envelope mapped |

Phase E exists to classify **A/B/C/D**.

---

## 2. Fork — only three serious paths left

```text
A  P2b / OOB READ          (source + harness)
B  SVM envelope semantics  (LiteSVM → Agave; E1–E5)
C  P5 reference × resize   (source audit; parallel)
```

Priority: **A → B**, with **C** in parallel at source level.

**Stopped / demoted:**

- More raw OOB write measurement  
- Economic-chain tracing until middle link reopens  
- Bitmap fuzz campaigns (negative; reopen only if inconsistent state found)  
- Forcing impossible `data_len` accounts  

---

## 3. Primitive ranking (updated)

| ID | Status | Notes |
|----|--------|-------|
| **P0** | Confirmed L1 | Length-discarding cast |
| **P1** | Confirmed L1+L2+L3 host | OOB **write** via `rotate_*` |
| **P2a** | Confirmed L2 | 113-byte slice formation on claimed extent |
| **P2b** | **Elevated — primary remaining source primitive** | Boundary-crossing **deserialize read** |
| **P3** | Blocked | Needs committed/read corruption first |
| **P4** | Weak | Victim DoS not shown |
| **P5** | **New — source audit** | Unsafe ref surviving resize |
| Neighbor | Low | After envelope map only |

---

## 4. Seven experiments (and nothing else)

1. Enumerate every `load` / `load_mut` / pinocchio cast caller  
2. Attack **P2b** (`byte_offset+113 > real` + Initialized tag)  
3. OOB **read-after-write** harness  
4. Audit **resize × reference lifetime** (P5)  
5. Exact production ix under **LiteSVM** (E1–E5)  
6. Same under **Agave**  
7. Forced-error **atomicity**

---

## 5. Phase E questions (mandatory)

| ID | Question |
|----|----------|
| **E1** | Does SVM permit the access? (tx success/fail) |
| **E2** | What physical region is the envelope? (pad / alloc / guard / other) |
| **E3** | Are writes observable in-tx? (write then read) |
| **E4** | Are writes persistent across txs? |
| **E5** | OOB then realistic failure → full rollback? |

### Reachable-failure matrix (for E5)

| Failure point | OOB already? | Resize already? | Persist? |
|---------------|-------------:|----------------:|---------:|
| before rotate | No | maybe | n/a |
| after rotate / before serialize | Yes | inc: yes / dec: not yet shrink | **?** |
| after serialize | Yes | same | **?** |
| after shrink | Yes | yes | **?** |
| later CPI / ix error | Yes | yes | **?** |

Only realistic fallible points (token transfer, rent, math errors) — not artificial panics.

---

## 6. Experiment 1 — Loader caller inventory

**Done.** Exhaustive production table (Anchor + Pinocchio) lives in the audit trail; summary:

| Caller class | R/W | Exceed real? | Attacker? | Interesting? |
|--------------|-----|--------------|-----------|--------------|
| `update_tick` → `rotate_*` | W | **yes** | yes | **P1 confirmed** |
| `get_tick` | R | P2a slice yes; P2b Init read **no** under invariant | yes | P2a only |
| `calculate_modify_liquidity` / fees | R | via `get_tick` | yes | same as get_tick |
| swap sparse `update_tick` | W | no rotate (cross keeps Init) | yes | in-bounds field writes |
| `initialize_dynamic_tick_array` | W | MIN_LEN cast | yes | header only |
| bitmap R/W | R/W | in header | yes | not in OOB envelope |

**No production path keeps a Ref/RefMut across resize** (see §9 P5).

---

## 7. Experiment 2 — P2b (`fuzz/dta_p2b`, log `fuzz/logs/dta_p2b.txt`)

**Verdict: CLOSED under invariant-holding reachable layouts.**

```text
states checked: 97
P2a Uninitialized slice crosses: 4084
P2b Initialized 113-byte OOB READ: 0
```

**Proof sketch:** if tick `i` is Initialized, `byte_offset(i)+113 = Σ_{j≤i} size(j) ≤ L`. Equality on last `I`.  
Trailing-`U` ticks can form `off+113 > L` (P2a), but Borsh reads **1** byte.

Anchor +8 on full account: last `I` window ends exactly at real `L=9944`; does **not** enter the +8 zone. The +8 is hit by open-ended `rotate_*`, not by fixed +113 reads.

**Reopen P2b only if** bitmap↔layout or `N↔k` breaks (separate bug class).

---

## 8. Experiment 3 — OOB read-after-write (`fuzz/dta_oob_rw`)

| Test | Result |
|------|--------|
| write OOB → raw read same allocation | **YES** — 9752 dirty; envelope bytes changed (host memory channel) |
| write OOB → production `get_tick` fields | **NO** influence under invariant |
| Uninit `get_tick` with crossing slice | forms past end; reads 1 in-bounds tag |
| Forged Init tag + crossing (invariant broken) | deserialize **can** consume past-boundary bytes — not a reachable production state |

**Envelope classification on host:**

- **A** for committed DEX / program-visible tick state  
- **B** only as raw process memory (not yet shown to feed production reads under invariant)  
- **C/D** unknown → Phase E  

---

## 9. Experiment 4 — P5 reference lifetime across resize

**Verdict: No surviving DynamicTickArray Ref/RefMut across resize** on any production path.

| Path | Order |
|------|-------|
| Increase | load → get_tick → **drop** → resize(+112) → **reload** → update_tick/rotate |
| Decrease | load → get_tick → update_tick/rotate → **drop** → resize(−112) → no reload |

Caveat (unchanged): rotate still runs under the size lie on a short/large account; P5 is specifically about **dangling refs after realloc**, which is **not** present.

---

## 10. Mental model after experiments 1–4

```text
                 CONFIRMED
                    │
                    ▼
           unsafe MAX_LEN view
                    │
                    ▼
              OOB WRITE ~9752
                    │
       ┌────────────┼─────────────┐
       ▼            ▼             ▼
  tick state    SVM memory     OOB READ
       │            │             │
       X            ?             X (P2b closed
  host negative              under invariant;
  shared-TA X                raw channel only)
```

Two question marks remain: **SVM envelope (C?)** and **commit/atomicity (E5)**.

---

## 11. Phase E — next (mandatory; no more host OOB fuzz)

Reuse / extend `audit_work/or_h01_runtime` LiteSVM harness and Agave model.

### Prior OR-H01 LiteSVM evidence (partial; re-frame under E1–E5)

From `or_h01_litesvm/run_output.txt` / thorough battery (synthetic PoC, not full Whirlpool ix):

| Observation | Implication |
|-------------|-------------|
| historical features: rotate **succeeds**, neighbor victim **untouched** | E1≈permit; E2≈padding absorbs (not automatic neighbor smash) |
| stricter / all_features: **`InvalidRealloc`** | modern bounds check can **trap** OOB |
| rare `rev_order` hist case: victim_diff_bytes=104 | layout-order sensitive; do **not** generalize to production account order |
| PoC used short **148** without production resize-first | must re-run at **`N_at_rotate=260`** and full Whirlpool path |

So: SVM relevance is real, but **not yet** a bounty story. Phase E must answer E1–E5 on the **production-sized** transition, then atomicity.

| ID | Experiment | Status |
|----|------------|--------|
| E1 | Production-like rotate — success or trap? | Partial (old PoC); **redo at N=260** |
| E2 | Envelope = pad / other / guard? | Partial (pad hypothesis); systematic sizes TBD |
| E3 | In-tx write→read envelope | **Not done** under SVM |
| E4 | Cross-tx persistence | **Not done** |
| E5 | OOB → realistic failure → rollback | **Not done** (mandatory) |

If all E1–E5 are negative (trap or discarded padding, full rollback, no program-visible read), the defensible conclusion is:

> **Real memory-safety bug; no demonstrated bounty-level impact under the deployed runtime.**

Do **not** force a theft narrative.

---

## 12. What we are explicitly *not* doing next

- Another 2k tick-state fuzz campaign  
- Economic-chain tracing while middle link stays closed  
- Artificial `data_len` not reachable via init/resize/increase/decrease  
- Neighbor-smash as primary track before envelope map
