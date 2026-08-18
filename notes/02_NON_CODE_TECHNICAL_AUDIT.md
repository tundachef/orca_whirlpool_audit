# Non-code technical audit — Orca / Whirlpools stack

**Phase:** Everything that is *not* instruction-handler / math / account-layout code review.  
**Date:** 2026-08-18  
**Sources:** on-chain RPC, Osec verify API, Immunefi URLs, repo metadata, ProgramData histories, `WhirlpoolsConfig` decode, prior inventory.

Severity scale used here: **Critical / High / Medium / Low / Info** — trust & operational, not “fund theft via bug” unless noted.

---

## Executive summary

| Area | Verdict (non-code) |
|------|---------------------|
| Program identity (Whirlpool) | **Held** — declare_id matches; Osec verifies commit `e5f089b` ↔ on-chain hash |
| Upgrade authority | **Live EOA-shaped key** `GwH3…` (system account, space=0) controls Whirlpool + Wavebreak + xORCA — labeled “multi-sig” in source comments but **not** an on-chain Squads program account |
| Legacy authority | `23zF9…` still controls V2 + Aquafarm — same shape |
| Immutable Whirlpool | Authority **burned** — positive |
| Verifiable build | Present + currently matching; **stale risk** if ProgramData moves without re-verify |
| License | **Material change** Feb 2025 — no longer Apache-2.0; competitive/commercial restrictions |
| Fee / config claims | Protocol fee rate on config **1300** with TOKEN_BADGE flag set — aligns with ~13% of swap fee to protocol side (docs: 12% DAO + 1% climate) |
| External audits | Six Whirlpool PDFs + one xORCA PDF present in-repo; **coverage vs current Pinocchio surface needs gap analysis** (code phase) |
| Closed-source gap | Wavebreak program + classic AMM/farm **program** sources missing publicly |

---

## NC-01 — Deployment identity & bytecode integrity

### Facts

- Program ID `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` matches `declare_id!` in `programs/whirlpool`.
- [Osec status](https://verify.osec.io/status/whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc): `is_verified: true`, `is_frozen: false`.
- Hash: `52e75447d1d49774ff6938484c9011e303860995497f0687a45febb3db21c5b0`.
- Repo URL pinned by verifier: `…/tree/e5f089bc5c49b01f5c8abb43c78457ab6c440568`.
- After that commit, only packaging commit touches `programs/whirlpool` path before clone HEAD.

### Findings

| ID | Sev | Finding |
|----|-----|---------|
| NC-01.1 | **Info** | Live Whirlpool bytecode is verifiably built from public commit `e5f089b` (2026-02-02). Good. |
| NC-01.2 | **Medium** | Upgrade authority is **not frozen** (`is_frozen: false`). Any future upgrade can invalidate the pin until re-verified. ProgramData shows a touch **2026-02-13** after Osec’s 2026-02-04 verify — confirm whether that was bytecode upgrade vs loader metadata (re-run `solana-verify` / Osec after each deploy). |
| NC-01.3 | **Low** | Local audit tip `main` @ `46dc1c2` is **83 commits** ahead of verified commit; almost all SDK/deps. Auditors must not assume tip ≡ chain. |
| NC-01.4 | **High** (process) | Wavebreak / V1 / V2 / Aquafarm have **no** public verifiable-build story equivalent to Whirlpool. |

### Recommendations

- Treat `e5f089b` as the Whirlpool code-audit pin until a newer Osec/verify succeeds.
- Require re-verify checklist after every ProgramData write on `CtXfPz…`.

---

## NC-02 — Upgrade authorities & key custody

### Facts

| Key | Role | On-chain type | Programs |
|-----|------|---------------|----------|
| `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` | Upgrade auth + Whirlpool `fee_authority` (config) | System Program, space=0, ~4.9 SOL | Whirlpool, Wavebreak, xORCA |
| `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` | Legacy upgrade auth | System, space=0, ~23.9 SOL | V2, Aquafarm |
| `94kZD71sbTKhqhcvY9D9Ra5BsLzKRZgznbBbQpBWmKrT` | xORCA `DEPLOYER_ADDRESS` (initialize) | System, space=0, ~1.2 SOL | xORCA init only |
| `CRQd5wvbf6FKVmjHC7on8w4pzFPzudij2BKXRcMCu7aK` | `collect_protocol_fees_authority` | System, space=0, **~64.2 SOL** | Config privileged |
| `DXnB9N9JLH5c9AYdKMGHQyspewSsvhFnwLK1tz1iPmZw` | `reward_emissions_super_authority` | System, space=0, ~0.06 SOL | Config privileged |

Source comment (`auth/admin.rs`, commit era `#1061`): GwH3 is described as *“program upgrade authority, multi-sig (Solana)”*.

### Findings

| ID | Sev | Finding |
|----|-----|---------|
| NC-02.1 | **High** | **Misrepresentation risk / custody opacity.** Account `GwH3…` is a **bare system-owned key** (0 data), not a Squads/Goki program account. “Multisig” must mean *off-chain* custody (MPC, hardware ceremony, or external policy) — **not verifiable on-chain**. Compromising that single pubkey upgrades three products and is also `fee_authority`. |
| NC-02.2 | **High** | **Privilege concentration.** Same `GwH3…` is both **BPF upgrade authority** and **WhirlpoolsConfig.fee_authority**. Upgrade key compromise ⇒ code rewrite *and* fee-parameter control. |
| NC-02.3 | **Medium** | Legacy `23zF9…` still live on V2 + Aquafarm years after Whirlpool migration — residual upgrade risk on classic stack. |
| NC-02.4 | **Medium** | `collect_protocol_fees_authority` holds **~64 SOL** as a hot-looking system account. If this key is also an operational fee-claimer, loss ⇒ fee theft (not pool vaults), still material ops risk. |
| NC-02.5 | **Info** | Immutable Whirlpool authority = `none` — correct trust reduction for that deployment. |
| NC-02.6 | **Low** | xORCA hardcodes a separate initializer (`94kZ…`) distinct from upgrade auth — good separation for *init*, but upgrade still GwH3. |

### Recommendations

- Publish **verifiable** on-chain multisig (e.g. Squads) as upgrade authority, or publish attestations of MPC/threshold setup.
- Split upgrade authority vs fee_authority vs collect_protocol_fees_authority.
- Rotate/retire `23zF9…` or burn V2/Aquafarm upgrade authority if programs are EOL.

---

## NC-03 — WhirlpoolsConfig (mainnet) — parameters & flags

### Decoded (`2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ`)

| Field | Value |
|-------|--------|
| Owner | Whirlpool program |
| `fee_authority` | `GwH3Hiv5…` |
| `collect_protocol_fees_authority` | `CRQd5wvb…` |
| `reward_emissions_super_authority` | `DXnB9N9J…` |
| `default_protocol_fee_rate` | **1300** |
| `feature_flags` | **1** (`TOKEN_BADGE` enabled) |

### Findings

| ID | Sev | Finding |
|----|-----|---------|
| NC-03.1 | **Info** | Docs claim ~12% DAO + 1% climate of trading fees → **13%**. Config default `1300` is consistent with protocol taking 13% of the pool fee (Whirlpool `protocol_fee_rate` semantics: portion of fee in 1/10000 units). **Docs ↔ config alignment held** at high level. |
| NC-03.2 | **Low** | Climate Fund vs DAO **split is off-chain / accounting** after `collect_protocol_fees` — not enforced by config fields. Non-code trust in treasury routing. |
| NC-03.3 | **Info** | TOKEN_BADGE feature flag enabled — Token-2022 badge path is live policy. |

---

## NC-04 — License & “open source” claims

### Facts

- README / marketing: “open-source”, Immunefi, public GitHub.
- LICENSE file (commit `ca5f054`, 2025-02-28): Apache-2.0 **until** 2025-02-26; thereafter **Orca License** with non-commercial authorized uses and **competitor restrictions**; commercial use needs written consent.
- GitHub license API: `NOASSERTION`.

### Findings

| ID | Sev | Finding |
|----|-----|---------|
| NC-04.1 | **Medium** | **Documentation / brand accuracy.** Calling the *current* tree “open source” without qualification is misleading under common OSI usage. Security researchers still appear covered under research clause; competitors/forks are not. |
| NC-04.2 | **Info** | Historical audits (2022) were under Apache-era code; post-2025 code is differently licensed — cite era when quoting rights. |
| NC-04.3 | **Low** | `security.txt` policy URL `https://immunefi.com/bounty/orca/` redirects toward Immunefi bug-bounty pages (alive). `SECURITY.md` in repo is only the Immunefi link — minimal. |

---

## NC-05 — External audits inventory (presence, not code re-review)

### Whirlpools (`.audits/` → `audits-external/`)

| Date | Firm | File |
|------|------|------|
| 2022-01-28 | Kudelski | `2022-01-28.pdf` |
| 2022-05-05 | Neodyme | `2022-05-05.pdf` |
| 2024-08-21 | OtterSec | `2024-08-21.pdf` |
| 2025-02-28 | Sec3 | `2025-02-28.pdf` |
| 2025-06-23 | Sec3 | `2025-06-23.pdf` |
| 2025-08-22 | Sec3 | `2025-08-22.pdf` |

### xORCA

| Date | File |
|------|------|
| 2025-09-24 | `audits-external/xorca/2025-09-24.pdf` |

### Findings

| ID | Sev | Finding |
|----|-----|---------|
| NC-05.1 | **Medium** | **Coverage gap risk.** Pinocchio entrypoint + increase/reposition liquidity-by-token-amounts landed **2026-01/02** — **after** the latest published Sec3 PDF (2025-08-22). Non-code observation: public audit PDFs may **not** cover the currently verified bytecode surface. Confirm with Orca whether a newer private/public audit exists. |
| NC-05.2 | **High** (scope) | No public audit PDFs located for **Wavebreak**, **Token Swap V1/V2**, or **Aquafarm** in the cloned public repos. |
| NC-05.3 | **Info** | Kudelski PDF dated **before** public GitHub open-source drop (Jan 2022 vs May 2022 OSS) — likely private-era review; still listed in README. |

---

## NC-06 — Bug bounty / disclosure channel

| Item | Observation |
|------|-------------|
| SECURITY.md | Single URL: Immunefi Orca bounty |
| security.txt policy | Immunefi `/bounty/orca/` (redirects) |
| Contacts in security.txt | Discord + Twitter only — **no security@ email** in on-chain txt |

| ID | Sev | Finding |
|----|-----|---------|
| NC-06.1 | **Low** | Disclosure path is Immunefi-centric; social contacts are not ideal primary security channels. Acceptable if Immunefi is active; verify max bounty / in-scope programs include Wavebreak & xORCA. |

---

## NC-07 — Product / deploy timeline hygiene (ProgramData)

| Program | First ProgramData (approx) | Notes |
|---------|----------------------------|-------|
| V2 | 2021-06-10 | Pre-dates Aquafarm |
| Aquafarm | 2021-08-02 | |
| Whirlpool | product 2022-03/04; frequent upgrades 2025–2026 | |
| Wavebreak | 2025-07-12 | Failed upgrade attempts 2025-08-11 visible on-chain |
| xORCA | 2025-09-29 | |
| Immutable | 2026-03-23 → auth burned by 2026-05-18 | |

| ID | Sev | Finding |
|----|-----|---------|
| NC-07.1 | **Info** | Wavebreak ProgramData shows **failed** loader txs (2025-08-11) then successful deploys — operational fragility during launch, historical only. |
| NC-07.2 | **Info** | V2/Aquafarm ProgramData touches in Jan–Feb **2024** after “last deploy” slots — likely authority/loader ops; confirm not silent bytecode changes without SDK notice. |

---

## NC-08 — Documentation claims vs chain

| Claim (docs / README) | Check | Result |
|----------------------|-------|--------|
| Program ID whirLb… | on-chain + declare_id | **Match** |
| Verifiable build | Osec API | **Match** at `e5f089b` |
| Audited (list of dates) | PDFs present | **Present**; gap vs 2026 Pinocchio (NC-05.1) |
| Fee split 87/12/1 | config default 1300 | **Consistent** with 13% protocol cut of fees |
| “Open source from day one” | LICENSE change 2025 | **Qualified / outdated** for current tip |
| Upgrade auth is multisig | GwH3 account type | **Not on-chain verifiable** |

---

## NC-09 — Eclipse / multi-chain (non-Solana-mainnet)

| Item | Note |
|------|------|
| Same program ID on Eclipse (docs) | Out of Solana-mainnet deep-dive unless extended |
| Admin list historically included Eclipse fee auth | Commit `58194bd` drops Eclipse admin from some paths — track carefully if auditing Eclipse |
| `program-registry` repo | Eclipse registry; not Solana Orca deploys |

---

## NC-10 — Scope completeness (wallet deploy sweep)

| Method | Result |
|--------|--------|
| SDK + crate program IDs | 7 mainnet programs |
| ProgramData authority cross-check | All upgradeable IDs map to `23zF9` or `GwH3` or none |
| Residual | Unreferenced one-off deploys by those keys need indexer — **open residual**, severity Info pending |

---

## Priority backlog (non-code → before/alongside code)

1. **Clarify / harden GwH3 custody** (NC-02.1–02.2) — highest trust issue.  
2. **Confirm audit coverage of Pinocchio/`e5f089b` surface** (NC-05.1).  
3. **Re-verify after any post-2026-02-04 ProgramData write** (NC-01.2).  
4. **Wavebreak source or verified build** (NC-01.4 / NC-05.2).  
5. **Legacy authority retirement** for V2/Aquafarm (NC-02.3).  
6. **Correct public “open source” language** (NC-04.1).  

---

## What this phase intentionally did *not* do

- Instruction-level vulnerability hunting (OR-H01 style) — next phases.  
- Full PDF re-audit of Kudelski/Neodyme/OtterSec/Sec3 findings.  
- Squads member enumeration (no on-chain multisig program account to enumerate).  
- Treasury wallet tracing beyond config authorities.
