# 03 — Orca Aquafarm technical audit

**Program ID:** `82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ`  
**Status:** Identity captured (pre-staged while V1 fuzz finishes)  
**Date:** 2026-08-18

---

## 1. Identity

| Check | Result |
|-------|--------|
| Loader | Upgradeable (`BPFLoaderUpgradeab1e`) |
| Authority | `23zF9Azpe9CN4iPeTsQndD1mQpcb5Gz1qFREL5gPTZvG` (same as Token Swap V2) |
| Dump | `fuzz/aquafarm/aquafarm.so` (562 848 bytes) |
| sha256 | `8e9b4884f561bf00f7b69e935e220e648f61afa6b61fe57fb5e7c13f9a4bb0fd` |
| Build fingerprint | `solana-program-1.7.1`, builder `/Users/orca/...`, sources `src/processor.rs`, `src/global_farm.rs` |

## 2. Instruction surface (ELF + SDK)

From ELF log strings + `aquafarm-sdk` `INSTRUCTIONS` enum:

| Tag | Name |
|-----|------|
| 0 | `InitGlobalFarm` |
| 1 | `InitUserFarm` |
| 2 | `ConvertTokens` |
| 3 | `RevertTokens` |
| 4 | `Harvest` |
| 5 | `RemoveRewards` |
| 6 | `SetEmissionsPerSecond` |

Privileged: `emissions_authority`, `remove_rewards_authority` (set at global farm init).

## 3. Codebase

| Artifact | Role |
|----------|------|
| Public program Rust | **None** in orca-so org |
| `orca-so/aquafarm-sdk` | Client + layouts + tests |
| `typescript-sdk` | Farm configs / `ORCA_FARM_ID` |

## 4. Fuzz (SDK-model)

Harness: `fuzz/aquafarm/emissions_fuzz` — harvest/accrue math from SDK:

`h = farm_tokens * (cumulativeEmissionsPerFarmToken - checkpoint) / 1e12`

| Sample | Result |
|--------|--------|
| 60s honggfuzz | **No crashes** (cumulative monotonic; synced harvest 0; rewind fails closed) |

**Caveat:** Model is reconstructed from client formulas, not the closed-source on-chain processor — treat as **hypothesis** until ELF path review.

## 5. Findings (so far)

| ID | Sev | Title |
|----|-----|-------|
| OA-I01 | **High** (trust) | Upgrade authority `23zF9…` live (shared with Token Swap V2) |
| OA-M01 | **Medium** (trust) | `RemoveRewards` — privileged drain of reward vault to arbitrary dest (by `remove_rewards_authority`) |
| OA-M02 | **Medium** (trust) | `SetEmissionsPerSecond` — `emissions_authority` can zero or spike emissions; no timelock in client model |
| OA-M03 | Info | Harvest math `h = baseTokensConverted * Δcumulative` (SDK); emissions fuzz sample clean |
| OA-I02 | Info | Dump sha256 `8e9b4884…`; built with solana-program **1.7.1** by `/Users/orca/...` |

## 6. Planned (remaining)

- Convert/revert farm-token ↔ base-token conservation vs dump.
- Confirm reward vault PDA binding in ELF.
- Longer emissions fuzz + convert/revert harness.
