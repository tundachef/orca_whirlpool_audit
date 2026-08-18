# Orca Contract Inventory (Solana Mainnet)

**Purpose:** Full picture of Orca-deployed / Orca-operated on-chain programs for an oldest→newest deep-dive audit.  
**As of:** 2026-08-18 (RPC: public mainnet-beta)  
**Org:** [github.com/orca-so](https://github.com/orca-so)

---

## 1. High-level timeline (oldest → newest)

Ordered by **first ProgramData touch** (approx. first deploy) where available; V1 has no ProgramData (Loader2).

| # | Product | Program ID | First ProgramData / product | `program show` last-deploy slot | Loader | Upgrade authority |
|---|---------|------------|-----------------------------|----------------------------------|--------|-------------------|
| 1 | **Token Swap V1** | `DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1` | Earliest classic Orca AMM (~2021) | n/a (immutable) | `BPFLoader2` | **none** |
| 2 | **Token Swap V2** | `9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP` | ProgramData from **2021-06-10** | `161177892` (2022-11-15)* | Upgradeable | `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` |
| 3 | **Aquafarm** (+ Double-Dip) | `82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ` | ProgramData from **2021-08-02** | `109658654` (2021-12-02)* | Upgradeable | `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` |
| 4 | **Whirlpools** (mutable) | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | Product launch **2022-03 / 2022-04** | `398059635` (2026-02-04)* | Upgradeable | `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` |
| 5 | **Wavebreak** | `waveQX2yP3H1pVU8djGvEHmYg8uamQ84AuyGtpsrXTF` | ProgramData from **2025-07-12** | `365746180` (2025-09-09) | Upgradeable | `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` |
| 6 | **xORCA staking** | `StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT` | ProgramData from **2025-09-29** | `370039372` (2025-09-29) | Upgradeable | `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` |
| 7 | **Whirlpools Immutable** | `iwhrLHdsgrvmnwU8GF2FSmyabSMjfHwFGJAX2ufJ3ZN` | ProgramData from **2026-03-23** | `420510431` (2026-05-18) | Upgradeable (auth burned) | **none** |

\* `solana program show` “Last Deployed In Slot” is the last **code** upgrade recorded in the ProgramData header. Later ProgramData txs (e.g. V2/Aquafarm Jan–Feb 2024) can be authority changes / other loader ops — see §1.1.

**Audit order (oldest first):**  
`V1 → V2 → Aquafarm → Whirlpools (mutable) → Wavebreak → xORCA → Whirlpools Immutable`

### 1.1 ProgramData signature history (deploy / loader ops)

Newest first. Truncated to what public RPC returned (Whirlpool/Wavebreak capped at 15).

**Token Swap V2** (`DrrJDy…`): 2024-02-18, 2024-01-30, 2022-11-15, 2022-11-10, 2022-09-21, **2021-09-21**, **2021-06-10** (first).

**Aquafarm** (`93dBKV…`): 2024-02-17, 2024-01-13, 2022-09-21, 2021-12-02, 2021-09-21, **2021-08-06**, **2021-08-02** (first).

**Whirlpools** (`CtXfPz…`, latest 15): 2026-02-13, 2026-02-04, 2026-01-22, 2025-09-11, 2025-08-21, 2025-07-05, 2025-06-25, 2025-06-10, 2025-05-13, 2025-05-10 (×2), 2025-05-05, 2025-04-21, 2025-02-28, 2025-02-27 — *active upgrade cadence; older history needs pagination*.

**Wavebreak** (`nEuknU…`): 2025-09-09 (×2), 2025-09-09 earlier, failed attempts 2025-08-11, then 2025-08-11 ok, 2025-07-29 cluster, 2025-07-23, 2025-07-22, **2025-07-12** (oldest in sample).

**xORCA** (`7TdF3a…`): 2025-12-10, 2025-10-03, 2025-10-01, **2025-09-29** (first).

**Whirlpools Immutable** (`7j2FgC…`): 2026-05-18 (×2), **2026-03-23** (first) — then authority burned.

---

## 2. Per-program detail

### 2.1 Token Swap V1 — classic constant-product (immutable)

| Field | Value |
|-------|--------|
| Program ID | `DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1` |
| Owner / loader | `BPFLoader2111111111111111111111111111111111` |
| Data length | 310 464 bytes (full binary on program account) |
| Local dump | `audit_work/dumps/orca_v1.so` |
| Source | **Not** in `orca-so/whirlpools`. Lineage is SPL Token-Swap; SDK refs in `orca-so/typescript-sdk` historically pointed at V2 for live pools. |
| Status | Live, residual classic pools; **cannot be upgraded** |

### 2.2 Aquafarm — yield farm / Double-Dip

| Field | Value |
|-------|--------|
| Program ID | `82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ` |
| ProgramData | `93dBKVunij6iKjnrskLh1rgJSJMp4ZfMTXETRTNtuD98` |
| Authority | `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` |
| Last deploy slot | `109658654` (~2021-12-02) |
| Data length | 562 848 bytes |
| GitHub | `orca-so/aquafarm-sdk` (SDK only; **no on-chain program source in public org**) |
| SDK constant | `ORCA_FARM_ID` in `typescript-sdk` `src/public/utils/constants.ts` |
| Notes | Double-Dip uses the **same** farm program ID (farm-of-farm pattern in SDK), not a separate deploy |

### 2.3 Token Swap V2 — classic AMM (upgradeable)

| Field | Value |
|-------|--------|
| Program ID | `9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP` |
| ProgramData | `DrrJDyBzyuyYAzkkjd6Vu9ZzaDLsKRf4RPXyRE7Uk2A8` |
| Authority | `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` |
| Last deploy slot | `161177892` (~2022-11-15) |
| Data length | 691 104 bytes |
| Local dump | `audit_work/dumps/orca_v2.so` |
| SDK | `ORCA_TOKEN_SWAP_ID` in `typescript-sdk` |
| Devnet swap ID (SDK) | `3xQ8SWv2GaFXXpHZNqkXsdxq5DZciHBz6ZFoPPfbFd7U` |
| Source | **Not** in whirlpools monorepo; treat as SPL Token-Swap lineage |

### 2.4 Whirlpools — concentrated liquidity (primary product)

| Field | Value |
|-------|--------|
| Program ID | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` (same ID Solana + Eclipse) |
| ProgramData | `CtXfPzz36dH5Ws4UYKZvrQ1Xqzn42ecDW6y8NKuiN8nD` |
| Authority | `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` (commented as Solana multisig in `auth/admin.rs`) |
| Last deploy slot | `398059635` (~2026-02-04) |
| Data length | 10 485 715 bytes (~10 MB) |
| Mainnet config | `2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ` |
| Devnet config | `FcrweFY1G9HJAHG5inkGB6pKg1HZ6x9UC2WioAfWrGkR` |
| GitHub | `orca-so/whirlpools` · default branch **`main`** |
| Local clone HEAD | `46dc1c26bc553423f1c7bad35ba5cf9d19f6b4e7` (2026-08-10, “Publish Packages”) — **shallow clone caveat** |
| On-chain crate | `programs/whirlpool` version **0.9.0**, Anchor **0.32.1** |
| Local SDK pins | `@orca-so/whirlpools` **8.0.1**; `@orca-so/whirlpools-client` **7.0.0**; legacy `@orca-so/whirlpools-sdk` **0.22.0** |
| Admins (`mainnet` feature) | `GwH3Hiv5…` (Solana); `AqiJTdr9…` (Eclipse fee auth) |

**Selected git tags (remote `orca-so/whirlpools`):**  
`v1.0.2`, `v1.0.1`, `v1.0.0`, `v0.12.5`, `v0.4.3` … `v0.1.16` (25+ tags listed via API).  
Notable branches: `main`, `immutable/78a6ff3`, `crnorthc/immutable-whirlpool-program`, `verifiable-build`, `solana-sdk-v3`, `yugure/adaptive-fee-sdk`, …

### 2.5 Wavebreak — bonding-curve launchpad

| Field | Value |
|-------|--------|
| Program ID | `waveQX2yP3H1pVU8djGvEHmYg8uamQ84AuyGtpsrXTF` |
| ProgramData | `nEuknUvGZK5UVyq3Tf18tcpNtC7dmjPMirrry66SkAs` |
| Authority | `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` |
| Last deploy slot | `365746180` → **2025-09-09 19:53 UTC** |
| Data length | 500 448 bytes |
| Public source | **Program repo not public** (as of inventory). Client: npm `@orca-so/wavebreak`, crate `orca_wavebreak` **2.0.0** (`WAVEBREAK_ID` in generated `programs.rs`) |
| Examples | `orca-so/wavebreak-sdk-examples` |
| Docs | https://docs.orca.so/campaigns/wavebreak/overview |

### 2.6 xORCA — liquid staking

| Field | Value |
|-------|--------|
| Program ID | `StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT` |
| ProgramData | `7TdF3aLJXvwo24azTD3vBTMzr5ScscQUdHzdfcf41kbD` |
| Authority | `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` |
| Last deploy slot | `370039372` → **2025-09-29** |
| Data length | 102 624 bytes |
| Hardcoded initializer | `DEPLOYER_ADDRESS` = `94kZD71sbTKhqhcvY9D9Ra5BsLzKRZgznbBbQpBWmKrT` (only this key can `initialize`) |
| ORCA mint | `orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE` |
| xORCA mint | `xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N` |
| GitHub | `orca-so/xorca` · tag **`v1.0.0`** (`6fb847d2149c`) · HEAD `05fe66b67c95` (2026-07-22) |
| Related | `orca-so/jup-xorca-integration` |

### 2.7 Whirlpools Immutable

| Field | Value |
|-------|--------|
| Program ID | `iwhrLHdsgrvmnwU8GF2FSmyabSMjfHwFGJAX2ufJ3ZN` |
| ProgramData | `7j2FgCQqJKs89k8w8dFdXrkeu8TT1EPJwkz5zuRUj1uo` |
| Authority | **none** (upgrade authority revoked) |
| Last deploy slot | `420510431` → **2026-05-18** |
| Data length | 1 538 016 bytes |
| Config | `8pm8erUsaMpmZ47LttHAPgnDx7xGZUvxY4q47vTCs5Nj` |
| SDK | `WhirlpoolDeployment.mainnetImmutable` in `ts-sdk/client/src/config.ts` |
| Branch signal | `crnorthc/immutable-whirlpool-program`, tag-like branch `immutable/78a6ff3` |

---

## 3. Upgrade authorities / “Orca deployer” wallets

| Address | Role | On-chain type | Controls |
|---------|------|---------------|----------|
| `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` | **Legacy** upgrade authority | System-owned, space=0 (~23.9 SOL) | Token Swap **V2**, **Aquafarm** |
| `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` | **Modern** upgrade authority (labeled “multi-sig” in `auth/admin.rs`) | System-owned, space=0 (~4.9 SOL) — **not** a program-owned Squads PDA | Whirlpools mutable, **Wavebreak**, **xORCA** |
| `94kZD71sbTKhqhcvY9D9Ra5BsLzKRZgznbBbQpBWmKrT` | xORCA `DEPLOYER_ADDRESS` (initialize-only hardcode) | System-owned, space=0 (~1.2 SOL) | Can initialize xORCA state; **not** the upgrade authority |
| *(none)* | Burned | — | Whirlpools Immutable; Token Swap V1 (Loader2) |

**Wallet-history methodology:**  
- Confirmed upgrade authority via `solana program show` for every upgradeable program.  
- Confirmed deploy cadence via `getSignaturesForAddress` on each **ProgramData** account (loader-only history).  
- Authorities are plain system accounts (custody / off-chain M-of-N possible); they are **not** on-chain Squads program accounts.  
- **No additional mainnet program IDs** beyond the seven above appear in Orca public SDKs, crates (`xorca`, `orca_wavebreak`), docs (`docs.orca.so/llms.txt`), or typescript-sdk farm/swap constants. Aquafarm *farm accounts* are numerous PDAs — not separate programs.  
- Residual risk: an obscure program upgraded once by `23zF9`/`GwH3` and never referenced in SDKs would need a full indexer sweep of those wallets’ BPF-loader txs.

---

## 4. Related accounts (not programs)

| Kind | Address | Notes |
|------|---------|-------|
| WhirlpoolsConfig (mainnet) | `2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ` | Pairs with mutable Whirlpool |
| WhirlpoolsConfig (devnet) | `FcrweFY1G9HJAHG5inkGB6pKg1HZ6x9UC2WioAfWrGkR` | |
| Immutable config | `8pm8erUsaMpmZ47LttHAPgnDx7xGZUvxY4q47vTCs5Nj` | |
| ORCA mint | `orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE` | |
| xORCA mint | `xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N` | |
| Aquafarm GlobalFarm accounts | many (see `typescript-sdk` `OrcaFarmConfig`) | User/farm state under Aquafarm program |

---

## 5. GitHub map (`orca-so` public repos relevant to contracts)

| Repo | Role | Default branch | Notes |
|------|------|----------------|-------|
| **whirlpools** | On-chain Whirlpool + SDKs | `main` | Primary audit target; pushed 2026-08-17 |
| **xorca** | On-chain xORCA + clients | `main` | Tag `v1.0.0`; pushed 2026-07-22 |
| **aquafarm-sdk** | Aquafarm client only | `main` | Last push 2021-12-21; **no program source** |
| **typescript-sdk** | Classic pools + farms SDK | `main` | Tags `1.2.25` …; last meaningful 2023-05 |
| **whirlpool-sdk** | Legacy Whirlpool TS | `main` | Superseded by monorepo; last 2022-10 |
| **orca-sdks** | Aggregated TS SDKs | `main` | 2025-03 |
| **wavebreak-sdk-examples** | Wavebreak examples | `main` | Program source private |
| **jup-xorca-integration** | Jupiter AMM interface for xORCA | `main` | |
| **stablecurve** | Curve math crate | `main` | Library, not a deployed program |
| **program-registry** | Eclipse program list | `main` | Not Orca Solana deploys |
| **multisig-monitor** | Multisig monitoring | `main` | Ops tooling |

Other org repos (`utl-*`, tutorials, CPI samples) are SDKs/tooling, not additional mainnet program deploys.

---

## 6. Audit sequence — **COMPLETE** (2026-08-18)

| # | Program | Result |
|---|---------|--------|
| 1 | Token Swap V1 | COMPLETE — see `01_TOKEN_SWAP_V1_AUDIT.md` |
| 2 | Token Swap V2 | COMPLETE — see `02_TOKEN_SWAP_V2_AUDIT.md` |
| 3 | Aquafarm | PHASE-COMPLETE — see `03_AQUAFARM_AUDIT.md` |
| 4 | Whirlpools mutable @ `e5f089b` | PHASE-COMPLETE — `04_WHIRLPOOLS_AUDIT.md` + math fuzz 1.31M/0 |
| 5 | Wavebreak | PHASE-COMPLETE — `05_WAVEBREAK_AUDIT.md` + client math fuzz 805k/0 |
| 6 | xORCA | PHASE-COMPLETE — `06_XORCA_AUDIT.md` + math fuzz 765k/0 |
| 7 | Whirlpools Immutable | PHASE-COMPLETE — `07_WHIRLPOOLS_IMMUTABLE_AUDIT.md`; auth burned |

Executive board: **`08_ORCA_QUEUE_CLOSEOUT.md`**.

---

## 7. Gaps / follow-ups

- [x] Ordered code+fuzz pass for all seven program IDs  
- [ ] Paginate ProgramData signature history for upgrade cadence charts (Whirlpool has frequent upgrades).  
- [ ] Resolve Squads/multisig member set behind `GwH3Hiv5…` and `23zF9…`.  
- [ ] Obtain Wavebreak program source or verified build artifact (repo appears private).  
- [ ] Locate Aquafarm / Token-Swap **program** sources if any private/archive exists; public org only has SDKs.  
- [ ] Deep wallet indexer scan for any *other* program IDs ever upgraded by `GwH3` / `23zF9` (unlikely but not fully closed with public RPC alone).  
- [ ] Eclipse deployments (same Whirlpool program ID; separate fee auth `AqiJTdr9…`).

---

## 8. Quick copy-paste ID list

```
DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1  # Token Swap V1
82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ  # Aquafarm
9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP  # Token Swap V2
whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc  # Whirlpools
waveQX2yP3H1pVU8djGvEHmYg8uamQ84AuyGtpsrXTF  # Wavebreak
StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT  # xORCA
iwhrLHdsgrvmnwU8GF2FSmyabSMjfHwFGJAX2ufJ3ZN  # Whirlpools Immutable

# Authorities
23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG  # legacy (V2 + Aquafarm)
GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV  # modern (Whirlpool + Wavebreak + xORCA)
94kZD71sbTKhqhcvY9D9Ra5BsLzKRZgznbBbQpBWmKrT  # xORCA initialize deployer
```
