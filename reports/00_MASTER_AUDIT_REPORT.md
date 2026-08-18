# Master Audit Report — Pre-2023 Solana AMM / DEX Programs

**Date:** 2026-08-13  
**Method:** Source-first deep review + assumption registers + unit/fuzz where runnable  
**Policy:** No mainnet exploit txs; findings are analysis-grade unless noted  
**Work dir:** `audit_work/` (sources/, findings/, notes/)

---

## 0. Global assumptions (audited continuously)

| ID | Assumption | Challenge / result | Final status |
|----|------------|-------------------|--------------|
| G0 | Pre-2023 inventory from prior session covers major DEX/AMMs | Cross-checked Vybe list + DefiLlama residual brands; missing some niche forks | **PARTIAL** — core set covered |
| G1 | Public GitHub HEAD ≈ deployed mainnet bytecode | `declare_id!` matches known IDs for Raydium AMM, CLMM, Saber, Whirlpool, Serum/OpenBook trees. **ELF hash not compared** (no program dump this pass to save RPC) | **OPEN** — findings are source-level |
| G2 | DefiLlama residual TVL sits behind these program IDs | Brand-level TVL can mix product versions; vault mapping not fully walked | **OPEN** for per-pool residual |
| G3 | Source review finds real bugs without bytecode | Closed-source / unverified farm programs not fully covered | **PARTIAL** |
| G4 | Helius account metadata from prior session is accurate | Used only for inventory; not re-hit heavily this pass | **ACCEPTED** for inventory |
| G5 | No mainnet exploit broadcast | Policy | **ACCEPTED** |
| G6 | Scope includes economic, admin, liveness, memory-safety — not only MSC/MOC | Applied | **ACCEPTED** |
| G7 | “Bugs exist” may include historical / privileged / edge — not only unprivileged drain | Reported with severity + preconditions | **ACCEPTED** |

**Implication of G1 OPEN:** A finding is “confirmed in this source tree.” Promotion to “confirmed on mainnet” needs dump/hash or on-fork PoC against live ELF.

---

## 1. Coverage matrix

| Program | Mainnet ID | Source used | Depth | Residual TVL signal | Report |
|---------|------------|-------------|-------|---------------------|--------|
| Serum DEX v3 | `9xQeWv…` | serum-dex | Deep | ~$15M brand | `serum_openbook.md` |
| Serum V1/V2 | `BJ3jr…` / `4ckm…` / `EUqoj…` | same family (no dump) | Medium (by lineage) | in Serum TVL | same |
| OpenBook v1 | `srmqPvym…` | openbook | Deep | ~$1.1M | same |
| Saber | `SSwpk…` | saber-hq/stable-swap | Deep + unit tests + partial fuzz | ~$4.3M | `saber_stableswap.md` |
| Raydium AMM v4 | `675kPX9…` | raydium-amm | Deep | large (brand) | `raydium_amm_v4.md` |
| Raydium CLMM | `CAMMCzo…` | raydium-clmm | Deep | large (brand) | `raydium_clmm.md` |
| Orca Whirlpool | `whirLb…` | whirlpools | Deep | large (brand) | `orca_whirlpools.md` |
| Orca TokenSwap V1/V2 | `DjVE6…` / `9W959…` | SPL token-swap lineage | Medium | unknown split | this report §5 |
| Raydium Stable / Farms | listed | partial / no public farm source | Low | unknown | deferred |
| Mercurial / Aldrin / Cropper / Crema / Lifinity / GooseFX / DOOAR | listed | incomplete / SDK-only | Low | $28k–$426k | deferred |
| Phoenix | `PhoeNi…` | phoenix-v1 clone | Low (not finished) | ~$0.9M | deferred |
| Invariant | `HyaB3…` | protocol clone | Low | ~$28k | deferred |
| Raydium AMM V3 (legacy exploit) | not in current docs | public postmortem only | Historical | mostly drained 2026 | known public critical |

---

## 2. Executive findings board (highest signal)

### Confirmed-in-source **High** (with preconditions)

| ID | Target | Title | Preconditions | User funds? |
|----|--------|-------|---------------|-------------|
| **OB-H01** | Serum/OpenBook | Permissionless `InitializeMarket` with `lot_size=0` | New market / front-run create | Anyone who trades that market |
| **OB-H02** | Serum (+OpenBook keys) | Hardcoded disable + fee-sweep authorities | Key custody (FTX-era for Serum) | All markets freeze / fee drain |
| **OB-H03** | Serum `SendTake` | Fee/deposit accounting can underflow `pc_deposits_total` on thin/ask paths | SendTake still enabled on Serum | Can DoS accounting / panic path |
| **SB-H1** | Saber | `SetNewFees` unbounded — admin can set fees ≫ 100% | Admin key | Yes, via fee extraction |
| **OR-H01** | Orca Whirlpool | `DynamicTickArrayLoader` casts account to `MAX_LEN` and `rotate_*` over virtual tail (**UB / potential OOB write**) | Dynamic tick arrays + init tick path; **Agave memory layout assumption OPEN** | Possibly other writable accounts if layout contiguous — **needs PoC** |
| **CL-F01** | Raydium CLMM | Support-mint allowlist can skip TransferHook filter | Admin (or allowlisted authority) adds hook mint | Re-entrancy style drain risk post-swap state write |

### Confirmed-in-source **Medium** (selected)

| ID | Target | Title |
|----|--------|-------|
| **RA-F01** | Raydium AMM v4 | `Initialize2` any market pubkey; front-run honeypot of pool PDAs |
| **RA-F02** | Raydium AMM v4 | `calc_take_pnl` can hard-error / stick deposits-withdraws |
| **RA-F03** | Raydium AMM v4 | Orderbook settle paths removed — residual OO/book balances may be stuck |
| **RA-STRICT** | Raydium AMM v4 | Withdraw requires `coin_amount < vault && pc_amount < vault` (**strict**); last full LP path can hit `TakePnlError` (parent audit addition) |
| **SB-M1..M5** | Saber | Init front-run; admin “timelock” is same-slot; fee denom 0 freezes; donation / 0 LP mint; last withdraw_one dust |
| **OB-M01..M06** | Serum/OpenBook | IOC rest regression (OpenBook); full event queue liveness; CloseOpenOrders rebate stuck; self-trade via dual OO; zeroed OO init defense-in-depth; full-book eviction |
| **OR-M01..M03** | Whirlpool | Fee growth mul overflow → 0 fees period; Token-2022 permanent delegate/hook/pausable with badge; adaptive fee admin extract |
| **CL-F02..F08** | CLMM | Extreme fee/status admin; Token-2022 slippage vs fee amount; protocol position rent to admin; fragile seeds; empty remaining_accounts panic; etc. |

### Public historical critical (out of current source trees)

| ID | Target | Title | Notes |
|----|--------|-------|-------|
| **RX-V3** | Raydium AMM **V3** (phased 2021) | LP mint not bound → fake LP withdraw (~$1.34M Jun 2026) | **Proven in wild**; exact ID not in current Raydium docs; validates residual-legacy thesis |

### Critical unprivileged drain in current audited sources

**None confirmed** without privileged keys, user interaction with malicious markets, or unproven memory-layout PoC (OR-H01).

That does **not** mean “no bugs.” Several High findings are real; residual risk is concentrated in **admin keys**, **malicious market creation**, **Token-2022 allowlists**, and **memory-safety UB**.

---

## 3. Assumption failures that matter

| Failed / weak assumption | Why it matters |
|--------------------------|----------------|
| “GitHub main == mainnet” (G1) | Raydium AMM tree is post-2026-07 cleanup (orderbook CPI removed). Live ELF may differ by upgrade slot. |
| “Pause is the strongest freeze” (Saber) | Fee denominator 0 freezes harder than Pause and bricks withdraws. |
| “Admin transfer is a 3-day timelock” (Saber comments) | Commit+Apply same slot. |
| “Locked positions cannot change liquidity” (Orca docs/intent) | Increase path does not check freeze (Low but real). |
| “DynamicTickArray only touches account bytes” | Explicitly documents UB past underlying data; rotate uses MAX_LEN view. |
| “Serum disable keys are safe” | Hardcoded, unrotatable on immutable loaders; FTX-era risk class. |
| “Raydium V4 withdraw always succeeds for full LP” | Strict `<` vault check can reject last-unit paths. |

---

## 4. Per-protocol summary

### 4.1 Serum / OpenBook — `serum_openbook.md`

**Assumptions held:** OO owner signer + owner match on settle/cancel/new-order; vault PDA; vault token owner checks at market init.  
**Assumptions failed:** “only honest lot sizes”; “global authorities rotatable”; “SendTake accounting invariant always holds.”  
**Fuzz:** not run (large matching engine); unit paths reviewed.  
**Residual:** brand still has multi-million TVL; treat disable keys and SendTake as priority residual risks.

### 4.2 Saber — `saber_stableswap.md`

**Assumptions held:** LP mint + reserves bound (anti–Raydium-V3). Admin signer required for admin Ixs. Ramp A bounded.  
**Assumptions failed:** fee bounds; admin delay; pause strength; first-depositor / last-LP dust.  
**Tests:** client 5/5, program 22/22 pass. Fuzz harness ran briefly (0 crashes) but **does not model malicious fee configs**.  
**Residual:** ~$4.3M; admin key is the primary live risk (H-1).

### 4.3 Raydium AMM v4 — `raydium_amm_v4.md` + parent notes

**Assumptions held:** `amm.lp_mint` checked on deposit/withdraw; vault keys bound; token program hardcoded Tokenkeg; signer for LP burn owner.  
**Assumptions failed / weak:** market binding on init; pnl calc robustness; post-orderbook cleanup completeness; strict vault inequality on withdraw.  
**Not reproduced:** V3 fake LP mint (fixed class in this tree).  
**Residual:** still large TVL; upgrade auth active (`FytDrV…` from prior inventory).

### 4.4 Raydium CLMM — `raydium_clmm.md`

**Assumptions held:** many PDA binds; 2024 remaining-accounts bitmap drain patched; tick liquidity checked.  
**Assumptions failed / weak:** support-mint allowlist vs TransferHook filter; extreme admin fee/status; some Token-2022 UX.  
**Residual:** active product; admin + allowlist process is security-critical.

### 4.5 Orca Whirlpools — `orca_whirlpools.md`

**Assumptions held:** position NFT authority; vault address constraints; tick array pool binding for swap sequencer.  
**Assumptions failed:** DynamicTickArray size safety (H-01); fee growth overflow path (M-01); locked-position increase (L).  
**Residual:** large TVL; H-01 needs local PoC before treating as critical.

### 4.6 Orca TokenSwap V1/V2 (SPL lineage) — this section

**Source assumption:** Orca V1/V2 historically track SPL Token Swap; modern `spl-lib/token-swap` may be **newer** than 2021 Orca deploy → **G1 weak**.

**What the SPL processor enforces (`check_accounts`):**
- swap account owner == program
- authority == PDA
- token_a/b accounts match state
- **pool_mint matches state** (R5 PASS in this tree)
- token program id matches state
- user accounts ≠ vaults
- fee account matches if provided

**Not a Raydium-V3-class LP mint miss** in this modern tree.  
**Remaining risks for legacy Orca pools:** older binary may differ; need dump of `DjVE6…` / `9W959…` for G1 close. Permissionless init / curve constraints depend on optional `SwapConstraints`.

### 4.7 Deferred programs (honest incompleteness)

Phoenix, Invariant, Mercurial, Aldrin, Cropper, Crema, Lifinity, GooseFX, Raydium farms/stable, Serum Swap deep pass: **not completed to same depth**. They remain on-chain with residual TVL. Next pass should prioritize by TVL: Phoenix → Aldrin → Crema/Cropper → Lifinity V1.

---

## 5. Fuzz / dynamic testing status

| Target | What ran | Result | Gap |
|--------|----------|--------|-----|
| Saber unit tests | cargo test program + client | pass | — |
| Saber fuzz | short `cargo fuzz` | 0 crashes / low execs | fees fixed; no malicious admin |
| SPL token-swap | has fuzz crate | **not run this pass** | should run next |
| Serum matching | no harness used | — | high value |
| Whirlpool DynamicTickArray | no PoC harness yet | — | **required for H-01** |
| Raydium AMM/CLMM | no custom fuzz | — | math/pnl edge cases |

---

## 6. Parent-verified additions (independent of agents)

### RA-STRICT — Raydium AMM v4 last-LP / full withdraw inequality

**Location:** `raydium-amm/program/src/processor.rs` ~1777  
**Code:** `if coin_amount < amm_coin_vault.amount && pc_amount < amm_pc_vault.amount`  
**Issue:** Uses **strict less-than**. A proportional withdraw that should empty (or fully claim remaining free reserves) can fail with `TakePnlError` when amounts equal vault balances.  
**Severity:** Medium (stuck dust / last LP friction; not third-party theft).  
**Assumption challenged:** “Withdraw always works for any `amount ≤ user_lp` when pool solvent.” **FAILED** at equality edge.  
**Fix:** use `<=` after reserving `need_take_pnl`, or compare against free reserves explicitly.

### OR-H01 — DynamicTickArray loader (independently reviewed)

**Location:** `whirlpool/src/state/dynamic_tick_array.rs`  
- `load_mut` casts `&mut [u8]` → `&mut [u8; MAX_LEN]` **without size check** (comment admits UB).  
- `update_tick` does `rotate_right/left` on `tick_data_mut()` which is the **full virtual tail**, not `account.data_len()`.  
- Resize in `update_tick_array_accounts` grows by **one tick (112 bytes)** before update — still far below MAX_LEN for sparse arrays.  
**Severity:** High pending runtime PoC; Critical only if cross-account write proven.  
**Assumption:** Agave isolates account allocations so OOB = crash not corruption. **OPEN**.

---

## 7. What we did **not** find (important negatives)

- Unprivileged Raydium **v4** fake-LP-mint drain like V3 (LP mint is bound).  
- Saber LP mint substitution drain (bound).  
- Serum settle without owner signer (signer + owner match present).  
- CLMM 2024 remaining-accounts bitmap drain (patched in this tree).  
- Trivial missing `is_signer` on primary withdraw paths of major audited AMMs.

---

## 8. Residual risk ranking (practical)

1. **Admin / upgrade keys** (Raydium, Saber, CLMM allowlist, Orca admin) — highest practical funds risk  
2. **OR-H01 memory safety** if PoC works on Agave  
3. **CL-F01 TransferHook allowlist** if process fails  
4. **Serum SendTake + global disable keys** on residual books  
5. **Malicious market init** (Serum lot sizes, Raydium init front-run)  
6. **Legacy closed-source / undumped** programs with residual TVL  

---

## 9. Planted / CTF note

Local Halborn Solana CTF criticals (relationship + underflow) remain documented in `PLANTED_CRITICALS_FINDINGS.txt`.  
No additional planted mainnet program IDs were discoverable in-repo. If planted DEXes with balance exist on-chain, provide IDs for targeted dump+audit.

---

## 10. Recommended next steps (thorough completion)

1. **PoC OR-H01** on local Agave/LiteSVM with undersized DynamicTickArray + adjacent writable account  
2. **`solana program dump`** for top residual: Serum V1/V2, Orca V1/V2, Saber, Raydium v4 — hash vs source (close G1)  
3. **Finish Phoenix + Aldrin + Serum Swap** deep passes  
4. **Extend Saber fuzz** to malicious fee denominators / admin fee supersizing  
5. **Vault sampling** for Serum top markets (batch getMultipleAccounts on known vaults only)  
6. Extract **Raydium V3 program ID** and confirm residual vaults empty  

---

## 11. File index

```
audit_work/
  notes/00_ASSUMPTIONS_FRAMEWORK.md
  findings/
    00_MASTER_AUDIT_REPORT.md          ← this file
    serum_openbook.md
    saber_stableswap.md
    raydium_amm_v4.md
    raydium_clmm.md
    orca_whirlpools.md
  sources/  (cloned repos)
```

---

## 12. Bottom line

We audited the **major open-source pre-2023 DEX/AMM surfaces** thoroughly enough to:

- Map money paths and validation  
- Reject several false “easy crit” narratives (e.g. Raydium v4 ≠ V3 LP mint bug)  
- Surface **multiple High/Medium real issues** with explicit assumption registers  
- Run Saber unit tests and a limited fuzz  
- Honestly mark **incomplete** targets and **G1 bytecode gap**

There **are** bugs. There is **not** (in this pass) a clean unprivileged multi-million drain recipe against current audited source without keys, victim interaction, or a still-unproven memory-layout PoC.

*End of master report.*
