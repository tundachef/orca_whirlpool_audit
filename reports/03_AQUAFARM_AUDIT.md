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

## 5. Planned (full Aquafarm pass)

- Convert/revert farm-token conservation against dump semantics.
- Authority residual shared with V2 (`23zF9…`) — **High** trust.
- Privileged `RemoveRewards` / `SetEmissionsPerSecond` abuse model.
