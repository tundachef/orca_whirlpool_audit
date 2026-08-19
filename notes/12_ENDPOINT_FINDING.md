# 12 — Endpoint finding: DynamicTickArray unsafe mapping

**Date:** 2026-08-19  
**Status:** Investigation closed for exploit hunting (alternate-path pass complete — see `#13`)  
**Scope:** Orca Whirlpools `DynamicTickArray` / `DynamicTickArrayLoader` (+ Pinocchio twin)

---

## One-paragraph finding

The DynamicTickArray loaders construct a fixed-size `MAX_LEN` (10,004-byte) view over account data whose actual length may be shorter (e.g. body 252 bytes at `N_at_rotate=260`), discarding the slice length. The production `update_tick` path can therefore perform `rotate_left/right(112)` over a falsely extended tick region. That out-of-bounds access is real: host canaries and LiteSVM rotate-only tests show attributable intermediate account-data mutation. However, every production rotate site finishes with bitmap update and Borsh serialize into the hole; under a LiteSVM FeatureSet reconstructed from the 301 currently activated mainnet feature accounts, the full production-like transition (rotate → bitmap → serialize) at `N_at_rotate=260` leaves a final account body **byte-identical** to a correctly bounded reference. Neighbor accounts were not observed to change. **No persistent DEX-state corruption, economic diversion, or victim DoS has been demonstrated.**

---

## Why the obvious exploit fails

```text
unsafe cast → OOB rotate  (real intermediate effect)
                    ↓
            serialize repair
                    ↓
         final == reference
```

---

## Severity posture (honest)

| Claim | Status |
|-------|--------|
| Memory-safety defect | **Yes — confirmed** |
| Fund loss / fee theft | **Not demonstrated** |
| Persistent wrong ticks | **Not demonstrated** |
| Victim DoS | **Not demonstrated** |

Recommend reporting as **memory-safety / correctness vulnerability without demonstrated economic impact**, not Critical fund theft.

---

## Evidence index

| Artifact | Role |
|----------|------|
| `notes/07…`–`11…` | Investigation trail |
| `fuzz/dta_canary` | Production-path L3 write measurement |
| `fuzz/dta_layout` / `dta_shared_ta` / `dta_p2b` / `dta_e31` | Host negatives + attribution |
| `fuzz/logs/mainnet_activated_features.json` | E0.5 FeatureSet dump (301) |
| `fuzz/logs/phase_e_mainnet.txt` | Mainnet FeatureSet + mode6 Δref=0 |
| `audit_work/.../parity_n260_mode6.js` | Agave parity script (AVX required) |

---

## Alternate-path pass (`#13`)

Verified: no CPI/observe between rotate and serialize; no fallible gap that leaves unrepaired state visible; initialize is header-only; swap never U↔I; no DTA migration/close alternate; all rotate sites classify as **REPAIRED**; zero **INTERESTING/CRITICAL** call-graph paths.

## Explicitly not doing further

- Broader OOB / bitmap / economic fuzz without final-state diff  
- E4 on the repaired path  
- Inflating `InvalidRealloc` under non-mainnet `all_enabled` into victim DoS  
- Neighbor-smash as primary impact  
- Further alternate-path hunting without a new concrete INTERESTING path  
