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

## 4. Planned (after V1/V2)

- Reconstruct emissions accounting from SDK layouts + ELF.
- Fuzz convert/revert/harvest invariants (farm token conservation, cumulative emissions monotonicity).
- Authority residual shared with V2 (`23zF9…`).
