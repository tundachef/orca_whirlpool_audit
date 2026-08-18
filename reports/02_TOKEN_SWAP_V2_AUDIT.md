# 02 — Orca Token Swap V2 technical audit

**Program ID:** `9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP`  
**Status:** Identity captured; full review starts after V1 close-out  
**Date:** 2026-08-18

---

## 1. Identity (G1) — CLOSED

| Check | Result |
|-------|--------|
| Loader | `BPFLoaderUpgradeab1e` — **upgradeable** |
| ProgramData | `DrrJDyBzyuyYAzkkjd6Vu9ZzaDLsKRf4RPXyRE7Uk2A8` |
| Authority | `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` (live) |
| Last deploy slot (header) | `161177892` (~2022-11-15); ProgramData also touched 2024-01/02 |
| Data length | 691 104 |
| Dump | `audit_work/dumps/orca_v2.so` sha256 `930a47e23fe7f8185557c1b5acabf658f97ca0f4c6cdd1f0d0510a897a9322e6` |
| Fresh on-chain dump | **byte-identical** |

---

## 2. Diff vs V1 (ELF fingerprint)

| | V1 `DjVE6…` | V2 `9W959…` |
|--|-------------|-------------|
| Size | 310 464 | 691 104 |
| Loader | Loader2 immutable | Upgradeable |
| Curves in ELF | CP, ConstantPrice, Stable, Offset | **Same four** |
| `InitOffsetCurve` string | present | **absent** |
| Constraint error strings | yes | yes |
| Token-2022 strings | no | no |
| Build id string | `solana-program-1.5.4` | not found as `solana-program-1.*` |

**Hypothesis:** V2 is a later/larger build of the same Token-Swap family (still Stable/Offset era), not the modern v3 Token-2022 tree. Same lineage source pin: `token-swap-v2.0.0` (+ later patches), pending instruction-layout confirmation.

---

## 3. Planned review (after V1 DoD)

1. ELF/ix layout vs V1 — confirm same tag set; note any new ixs.  
2. Production constraints / embedded owner key probe.  
3. Reuse V1 math fuzz; add any V2-only surfaces.  
4. **Upgrade authority residual** (`23zF9…`) as High trust finding (shared with Aquafarm).  
5. Write findings OV2-* and commit.

---

## 4. Preliminary findings (identity-only)

| ID | Sev | Title |
|----|-----|-------|
| OV2-I01 | **High** (trust) | Live upgrade authority `23zF9…` — code can change anytime |
| OV2-I02 | Info | Same curve family as V1 including Stable/Offset |
| OV2-I03 | Info | Dump matches chain — G1 held |
