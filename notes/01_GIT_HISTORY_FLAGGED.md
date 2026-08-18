# Whirlpools git history — full scan & flagged commits

**Repo:** `orca-so/whirlpools` (clone: `sources/whirlpools`)  
**First commit:** `f0b4de883fd259fdd46b382200fad4ade8048e2a` — 2022-04-21 — *Initial commit*  
**Open-source drop:** `c55588a6eae1144be58bd348992f693e8b5ddc01` — 2022-05-02 — *Open Source commit for Orca Whirlpools (#1)*  
**HEAD (clone):** `46dc1c26bc553423f1c7bad35ba5cf9d19f6b4e7` — 2026-08-10 — *Publish Packages*  
**Total commits on `main`:** **953**

| Year | Commits (approx) |
|------|------------------|
| 2022 | 81 |
| 2023 | 43 |
| 2024 | 359 |
| 2025 | 370 |
| 2026 | 100 (through Aug) |

Top authors (incl. bots): dependabot (500), yugure / yugure-orca, Will / wjthieme, josh-orca, meep, shio, Calin, …

---

## Era map (program-relevant)

| Era | Dates | Theme |
|-----|-------|--------|
| Genesis OSS | 2022-04 → 2022-05 | Initial commit → public #1 → SDK docs |
| Early CLMM | 2022–2023 | Two-hop swap, bundled positions, fee-rate limits |
| Token-2022 / monorepo | 2024-05 → 2024-08 | TokenExtensions, security.txt, NX monorepo, Kinobi clients |
| Adaptive fee / events | 2025-Q1–Q2 | Adaptive Fee release, events, MAX_FEE_RATE |
| Hardening / Pinocchio | 2025-Q3 → 2026-Q1 | Safer init, non-transferable positions, ADMINS multisig comment, Pinocchio entrypoint + liquidity ixs |
| Immutable + license | 2025-02 license; 2026-03–05 immutable program | Orca License; immutable deploy path; temp admin then dropped |
| Post-verify SDK churn | 2026-02 → 2026-08 | Mostly publishes / deps; on-chain pin stays at `e5f089b` |

---

## Flagged commits (review these first — trust / deploy / surface)

### A. Trust, license, admin, verify

| Flag | Commit | Date | Why |
|------|--------|------|-----|
| **LICENSE** | `ca5f054` | 2025-02-28 | Apache-2.0 → proprietary Orca License (#782) |
| **LICENSE email** | `d5c26c3` | 2026-07-23 | License contact tweak (#1329) |
| **security.txt** | `bf46095` | 2024-05-24 | On-chain security.txt + Immunefi policy URL (#144) |
| **ADMINS “multi-sig”** | `648f632` | 2025-09-12 | `make ADMINS multi-sig (#1061)` — labels GwH3 as upgrade auth |
| **Verifiable build CI** | `71bd6d1` / `08a984d` | 2026-02-02 | Add Verifiable Build Action / test |
| **Osec pin (live bytecode)** | `e5f089b` | 2026-02-02 | PR #1229 — **matches Osec verified on-chain hash** |
| **Immutable temp admin** | `895c6df` | 2026-03-24 | add temporary admin to init immutable whirlpool |
| **Drop temp / Eclipse admin** | `58194bd` | 2026-05-18 | drop temporary admin and Eclipse admin (aligns with immutable burn window) |

### B. Program surface / consensus-critical mechanics

| Flag | Commit | Date | Why |
|------|--------|------|-----|
| **OSS program birth** | `c55588a` | 2022-05-02 | First public program tree |
| **Two-hop** | `c3a02ee` | 2023-02-17 | Two-hop swap ix |
| **Bundled positions** | `44021b1` | 2023-04-07 | Position bundles |
| **Token-2022** | `103f504` | 2024-05-24 | TokenExtensions contract+SDK |
| **ExactOut partial-fill fix** | `bbe761a` | 2024-08-31 | Reject partial fill ExactOut if sqrt_price_limit=0 |
| **Adaptive Fee** | `03525a8` | 2025-04-30 | Adaptive Fee Release (#918) |
| **Events** | `7488726` | 2025-02-28 | Event emission |
| **MAX_FEE_RATE** | `0f478e7` | 2025-02-14 | Increase MAX_FEE_RATE |
| **Safer account init** | `d5a8bfd` | 2025-08-13 | Safer account initialization (#1010) |
| **Non-transferable position** | `19875ce` | 2025-08-18 | Lock / non-transferable NFT positions (#1038) |
| **Dynamic TickArray** | `2509ad9` | 2025-07-03 | Dynamic TickArray (#970) |
| **Anchor 0.32.1** | `3edef23` | 2025-10-23 | Major framework upgrade |
| **Pinocchio base** | `822f213` | 2026-01-30 | Pinocchio Base and Liquidity Ops (#1226) |
| **Entrypoint / multisig reject** | `04324b6` / `2d5cc76` | 2026-01-26 | update entrypoint; **reject multisig account** |
| **Reposition liquidity** | `6017e1d` | 2026-02-02 | reposition liquidity (#1227) |
| **Increase by token amounts** | `e5f089b` | 2026-02-02 | #1229 — live verified tip |
| **Remove migrate reward-auth space** | `ad183d8` | 2026-06-05 | remove migrate_repurpose_reward_authority_space ix |

### C. Ops / deploy scripts (non-bytecode but trust-adjacent)

| Flag | Commit | Date | Why |
|------|--------|------|-----|
| Deploy script flags | `71569ae` | 2026-01-27 | remove dry run flag from deploy script |
| Deploy scope | `bee5bae` | 2026-01-22 | strip special chars, scope deploy script |
| Publish→deploy rename | `fa6429d` | 2025-01-31 | Rename publish script to deploy |

### D. Noise to deprioritize for program audit

- Dependabot bumps (majority of 2026 commits)  
- “Publish Packages” tags / SDK-only releases  
- Example repositioning-bot dependency bumps  
- Docs site deploy workflow churn  

---

## xORCA (`sources/xorca`) — short history

| Item | Value |
|------|--------|
| First commit | `9e6c606` 2025-06-05 *Init commit* |
| Commits | 111 |
| Tip | `05fe66b` 2026-07-22 *change license email* |
| External audit PDF | `audits-external/xorca/2025-09-24.pdf` |
| Tag | `v1.0.0` |

---

## Classic SDKs (no program history)

| Repo | Commits / note |
|------|----------------|
| `typescript-sdk` | Classic pool/farm client; last meaningful ~2023 |
| `aquafarm-sdk` | Single meaningful push era 2021; SDK only |

---

## Method

```bash
cd sources/whirlpools
git log --reverse --format='%H %ci %s' | head   # first commits
git log --all --grep='...' -i                     # keyword flags
git log -- programs/whirlpool                     # program-only
git rev-list --count e5f089b..HEAD -- programs/whirlpool
```

Full clone is **not** shallow (`rev-list --count` = 953). Prior `audit_work/sources/whirlpools` was shallow (1 commit) until unshallowed in parallel.
