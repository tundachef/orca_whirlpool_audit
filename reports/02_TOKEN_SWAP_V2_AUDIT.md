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

## 3. Production constraints (ELF-confirmed)

Unlike V1, V2 **embeds** the modern SPL `production` fee constant blob at file offset `184384`:

| Field | Value |
|-------|--------|
| trade | **0 / 10000** |
| owner_trade | **5 / 10000** |
| owner_withdraw | **0 / 0** |
| host | **20 / 100** (20% of owner trade fee) |

**Fee-account owner key (ASCII in ELF):**  
`2YM8LrJGRtsDcWeqsjX2EQwJfhArxyDdtDzgt7vrwwbV`

Constraint error strings present (“fee does not match the program owner”, “curve type is not supported by the program owner”). Production allowlist historically is **ConstantProduct + ConstantPrice only** — Stable/Offset remain in the binary (swap existing pools) but **new** Stable/Offset inits should fail constraints if `SWAP_CONSTRAINTS` is `Some`.

**Contrast with V1:** V1 has **no** `u64 10000` and **no** fee blob → unconstrained. V2 is **constrained**.

---

## 4. Findings

| ID | Sev | Title |
|----|-----|-------|
| OV2-I01 | **High** (trust) | Live upgrade authority `23zF9…` — bytecode mutable |
| OV2-M01 | **Medium** (positive vs V1) | Production fee + owner constraints embedded — blocks free-form malicious fee/curve init |
| OV2-M02 | Info | Same curve calculators as V1 still linked (Stable/Offset code present) |
| OV2-M03 | Info | Fee owner `2YM8Lr…` must own pool fee token accounts at init — map to known Orca treasury if possible |
| OV2-I03 | Info | Dump matches chain — G1 held |

---

## 5. Fuzz / next

- Reuse V1 math fuzz (same curve family).  
- Confirm `validate_fees` is exact-equality (older) vs minimum-numerator (newer) against this binary’s behavior if needed.  
- Map `2YM8Lr…` on-chain (owner of fee ATAs / known Orca wallets).  
- Then close #2 and proceed to Aquafarm.
