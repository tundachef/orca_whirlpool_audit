# 04 — Residual verification battery (post-queue)

**Date:** 2026-08-18  
**Goal:** Bounded re-check of highest residuals after ordered queue completion.  
**Policy:** Confirm / refine severity; no mainnet exploit txs.

---

## R1 — Wavebreak graduation / close (WB-H02, WB-M05/M06)

**Method:** Extended `fuzz/wavebreak/math_fuzz` with:
- `GraduationInsolvency` — asserts `quote_graduation_amount` returns **Err** when `creator_reward + graduation_reward > quote`
- `CloseQuoteStress` — `close_quote` with alloc=0 / extreme BPS (must not panic)

**Result:** `fuzz/wavebreak/logs/residual_90s.txt` — **557,098 iters / 91s / 0 crashes / 0 timeouts**.

**Verdict:** Client math **fails closed** (Err) on insolvent graduation rewards — severity remains **High liveness/config** if on-chain init can set rewards without the same bound (ELF still closed-source → **WB-RESIDUAL**).

---

## R2 — xORCA inflation / cool-down (XO-M01/M02/M03)

**Method:** Deterministic probe `fuzz/xorca/residual_probe` (log: `fuzz/xorca/logs/residual_probe.txt`).

| Case | Result |
|------|--------|
| Donor ≫ victim after tiny first stake | **4/5 → ZERO shares** (on-chain: `InsufficientStakeAmount`) |
| 1 ORCA victim after ~500M ORCA donation | ZERO shares |
| Equal 1e6 pool + 1e12 donor + 1e6 victim | shares=1 (extreme dilution) |
| `non_escrowed==0`, supply≫0 | 1:1 special case **under-mints** vs virtual-offset fair formula by up to ~`supply/100` |
| Cool-down bounds | Only `< 0` rejected; **0 allowed**; existing pendings not rewritten |

**Verdict:**
- **XO-M02 CONFIRMED** residual of virtual offset 100 (griefing / dust, not silent vault theft of existing holders via this path alone — stake fails instead of minting worthless shares when shares==0).
- **XO-M03 CONFIRMED** user-hostile under-mint when vault fully escrowed.
- **XO-M01 CONFIRMED** by code: cool-down may be 0.

Sec3 H-01 remains “Resolved” for the classic first-depositor inflation steal; residual is **economic griefing / dilution**, not the original Critical-class steal.

---

## R3 — OW-H02 DynamicTickArray

**Prior evidence (unchanged):**
- Unsafe `load`/`load_mut` without len check still at `e5f089b` / Immutable lineage
- Call sites pass `data[8..]` while `MAX_LEN` includes discriminator
- Runtime battery: `04_OR_H01_RUNTIME_RESULT.md`, `05_OR_H01_THOROUGH_RESIDUAL_BATTERY.md` — **Critical neighbor theft DEBUNKED** under current Agave pad model

**Verdict:** Severity stays **High (UB / future-compat)** — not promoted or demoted this pass. Immutable **cannot patch** without new program ID (**WI-M02**).

---

## Board update

| ID | Prior | After residual battery |
|----|-------|------------------------|
| WB-H02 | High (config/liveness) | **Confirmed client fails closed**; ELF init bounds still unverified |
| XO-M02 | Medium residual | **Confirmed** with concrete donation matrices |
| XO-M03 | Medium UX | **Confirmed** under-mint ratios |
| XO-M01 | Medium ops | **Confirmed** code allows 0 |
| OW-H02 | High UB | **Unchanged** — Critical still debunked |
