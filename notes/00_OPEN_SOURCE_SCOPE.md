# Open-source scope — what we are reviewing

## In scope (public source available)

| Artifact | Repo / path | Maps to mainnet | Notes |
|----------|-------------|-----------------|-------|
| Whirlpools program | `orca-so/whirlpools` → `programs/whirlpool` | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | Primary; **953** commits since 2022-04-21 |
| Whirlpools SDKs | same monorepo (`ts-sdk/`, `rust-sdk/`, `legacy-sdk/`) | clients only | Not consensus-critical |
| xORCA program | `orca-so/xorca` → `solana-program/` | `StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT` | **111** commits since 2025-06-05 |
| xORCA clients | `xorca` js/rust clients | clients | |
| Classic AMM / farm **clients** | `orca-so/typescript-sdk`, `aquafarm-sdk` | talk to V1/V2/Aquafarm | **No program Rust in public org** |
| Wavebreak **clients/examples** | npm `@orca-so/wavebreak`, crate `orca_wavebreak`, `wavebreak-sdk-examples` | `waveQX2yP3H1pVU8djGvEHmYg8uamQ84AuyGtpsrXTF` | Program source **not public** |

## Out of public source (still in *protocol* audit via dumps / binary / SDK)

| Program | ID | Approach |
|---------|-----|----------|
| Token Swap V1 | `DjVE6…` | Dump `audit_work/dumps/orca_v1.so` + SPL Token-Swap lineage |
| Token Swap V2 | `9W959…` | Dump `orca_v2.so` + SPL lineage |
| Aquafarm | `82yxje…` | Dump + `aquafarm-sdk` / `typescript-sdk` instruction layouts |
| Wavebreak program | `waveQX…` | Closed source; client IDL + on-chain ProgramData |
| Whirlpools Immutable | `iwhrL…` | Same lineage as Whirlpool; authority burned; treat as frozen deploy |

## License reality (non-code, binding)

| Repo | License posture |
|------|-----------------|
| `whirlpools` | **Until 2025-02-26:** Apache-2.0. **From 2025-02-27:** proprietary **“Orca License”** (non-commercial / competitor restrictions; commercial use needs written consent `ip@orca.so`). Commit `ca5f054` (#782). |
| `xorca` | Same family of Orca License (see repo `LICENSE`) |
| GitHub API `license.spdx_id` | `NOASSERTION` for whirlpools — accurate; not OSI open source after Feb 2025 |

**Implication for audit:** “Open source” marketing is **partially outdated**. Research/security review of current tip is allowed under Authorized Non-Commercial Uses (research/vuln research), but redistributing a competing fork of post-2025-02-27 code is restricted.

## Bytecode ↔ source pin (Whirlpool)

| Item | Value |
|------|--------|
| Osec verified | **true** (as of inventory) |
| On-chain / executable hash | `52e75447d1d49774ff6938484c9011e303860995497f0687a45febb3db21c5b0` |
| Verified commit | `e5f089bc5c49b01f5c8abb43c78457ab6c440568` (2026-02-02, PR #1229 Increase Liquidity By Token Amounts) |
| Verified at | 2026-02-04T18:49:13Z (matches last major deploy window) |
| Local / clone `main` HEAD | `46dc1c26bc553423f1c7bad35ba5cf9d19f6b4e7` (2026-08-10 Publish Packages) |
| Commits after verified → HEAD touching `programs/whirlpool` | **1** (`0e72771` Publish Packages — packaging, not a new instruction set) |

**Audit rule:** Program *logic* review of live mainnet should pin **`e5f089b`**, not floating `main`. SDK tip may be ahead.
