# Orca Whirlpools Security Audit

**Program ID (mainnet):** `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`  
**Source:** `/home/ziion/Documents/solanaProtocols/audit_work/sources/whirlpools/programs/whirlpool`  
**Crate version in tree:** `0.9.0`  
**Framework:** Anchor 0.32.1 + custom Pinocchio entrypoint for liquidity instructions  
**Date:** 2026-08-13  
**Auditor pass:** source review (no mainnet bytecode/hash compare in this pass)

---

## 0. Assumptions (every stage)

### Stage 0 — Identity / mapping

| ID | Assumption | How audited | Status |
|----|------------|-------------|--------|
| W0 | This repo (`programs/whirlpool`) is the on-chain program for `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | `declare_id!` in `src/lib.rs` matches the stated mainnet ID | **HELD** for identity; **OPEN** whether this commit equals currently deployed bytecode |
| W1 | `default = ["whirlpool-entrypoint"]` is what mainnet builds | `Cargo.toml` default feature routes increase/decrease/reposition to Pinocchio | **ASSUMED**. If mainnet is built without this feature, Anchor `unreachable!()` liquidity stubs would brick those Ixs |
| W2 | Upgrade authority is the Solana multisig in `auth/admin.rs` (`GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV`) when built with `mainnet` | Admin list is feature-gated; deploy feature flags not verified against cluster | **OPEN** (need programdata upgrade authority + last slot) |
| W3 | Token-Swap V1/V2 are **not** in this monorepo | Grep for `DjVE6…` / `9W959…` / token-swap program crate: not present | **HELD** — see §8 |
| W4 | Residual Orca TVL is mostly Whirlpools, not V1/V2 constant-product | Not measured this pass | **OPEN** |

### Stage 1 — Trust / assets

| ID | Assumption | How audited | Status |
|----|------------|-------------|--------|
| W5 | Vault token accounts are owned by the Whirlpool PDA; only the program can move funds via signed CPI | `initialize_vault_token_account` / v1 `token::authority = whirlpool`; transfers use `whirlpool.seeds()` | **HELD** for program-created vaults |
| W6 | Position authority = owner or delegate of the position NFT (amount == 1) | `verify_position_authority(_interface)` | **HELD** |
| W7 | Token-2022 risky extensions cannot enter a pool unless a config TokenBadge is issued | `is_supported_token_mint` + badge PDA. TransferFee/InterestBearing/Metadata/ScaledUi allowed **without** badge | **PARTIAL** — see M-02 |
| W8 | Locked positions cannot change liquidity | Decrease/reposition/close check freeze; **increase does not** | **FAILED** — see L-01 |
| W9 | DynamicTickArray loader only touches bytes inside the account | `DynamicTickArrayLoader` is a `MAX_LEN` view over a variable-size account; `rotate_*` uses the full virtual slice | **FAILED** — see H-01 |

### Stage 2 — Instruction invariants

| ID | Assumption | How audited | Status |
|----|------------|-------------|--------|
| W10 | Tick arrays used in swap/liquidity belong to the pool and are sequential | `load_tick_array_mut` checks owner, discriminator, `whirlpool` field; `SparseSwapTickSequenceBuilder` derives required start indexes and PDA | **HELD** |
| W11 | Uninitialized tick-array PDAs (empty, system-owned) are safe to treat as all-zero ticks | Only accepted if pubkey == derived PDA; updates to uninitialized proxy panic | **HELD** |
| W12 | `sqrt_price_limit == 0` is “no limit” and cannot bypass slippage | Mapped to MIN/MAX; exact-out without explicit limit rejects partial fill; clients still need `other_amount_threshold` | **HELD** as designed; leftover user-risk if slippage = 0 |
| W13 | Two-hop intermediate mint/amount must match; same-pool hops forbidden | Both v1 and v2 check `DuplicateTwoHopPool`, mint equality, `IntermediateTokenAmountMismatch` | **HELD** |
| W14 | V2 two-hop vault→vault transfer-fee is applied once | Exact-in hop2 consumes hop1 output as fee-included; exact-out inverts with excluded amount; inverse-fee verified | **HELD** (careful, looks correct) |
| W15 | Fee/reward growth math cannot steal other LPs’ unclaimed fees | wrapping_sub is Uniswap-v3 style; **mul overflow → 0 fees** | **FAILED** as a loss vector — see M-01 |
| W16 | Remaining-accounts slices cannot be confused (hook vs tick arrays) | Typed slices, no duplicates, max 3 supplemental TAs, invalid type rejected | **HELD** for parser; hook programs themselves are trusted via badge |

### Stage 3 — Out of scope this pass

- Bytecode/programdata hash vs this tree
- Full economic / MEV analysis of adaptive fee
- Closed-source Orca Token Swap V1/V2 (see §8)
- Off-chain SDK quote correctness (except where it documents on-chain behavior)
- Eclipse deployment (`AqiJTdr9…` admin)

---

## 1. Program map

### 1.1 On-chain crate

```
programs/whirlpool/src/
  lib.rs                 Anchor #[program] (many Ixs; liquidity = unreachable! / Pinocchio)
  entrypoint.rs          Pinocchio dispatch for 6 liquidity Ixs, else Anchor
  instructions/          Anchor account structs + handlers
  instructions/v2/       Token-2022 variants
  instructions/adaptive_fee/
  pinocchio/             Live increase/decrease/reposition implementation
  manager/               swap, liquidity, tick, fee-rate, position, whirlpool
  math/                  swap, liquidity, tick, token, u256
  state/                 Whirlpool, Position, tick arrays, oracle, badges, lock
  util/                  transfers, remaining_accounts, Token-2022
```

### 1.2 Instruction surface (audited)

**Pool / config**

| Ix | Notes |
|----|--------|
| `initialize_config` | Funder must be `ADMINS[]` |
| `initialize_pool` | SPL Token only; PDA `[whirlpool, config, mint_a, mint_b, tick_spacing]`; mint_a < mint_b |
| `initialize_pool_v2` | Token-2022; badge check; random-keypair vaults + ImmutableOwner |
| `initialize_pool_with_adaptive_fee` | Creates Oracle PDA; optional `trade_enable_timestamp` ≤ 72h |
| `initialize_fee_tier` / `initialize_adaptive_fee_tier` | `fee_authority` |
| `initialize_tick_array` | Fixed 88-tick array, PDA `[tick_array, pool, start_tick.to_string()]` |
| `initialize_dynamic_tick_array` | Same PDA seeds; variable size; `idempotent` allows existing Fixed or Dynamic |
| `initialize_config_extension` / token badge Ixs | Feature-flagged |
| `migrate_repurpose_reward_authority_space` | Permissionless leftover — I-03 |

**Liquidity / positions** (Pinocchio on default build)

| Ix | Authority | Lock? |
|----|-----------|-------|
| `open_position` / `_with_metadata` / `_with_token_extensions` | funder | n/a |
| `open_bundled_position` | bundle NFT owner/delegate | n/a |
| `increase_liquidity` / `_v2` / `_by_token_amounts_v2` | position NFT | **not checked** |
| `decrease_liquidity` / `_v2` | position NFT | blocked if frozen |
| `reposition_liquidity_v2` | position NFT | blocked if frozen |
| `reset_position_range` | position NFT | empty-only (locked can't be empty) |
| `close_position` / `_with_token_extensions` / `close_bundled_position` | NFT | TE close blocked if frozen |
| `lock_position` / `transfer_locked_position` | owner (transfer: owner only, not delegate) | — |

**Swap**

| Ix | Slippage | Tick arrays |
|----|----------|-------------|
| `swap` / `swap_v2` | `other_amount_threshold` + `sqrt_price_limit` | 3 static + up to 3 supplemental (v2) |
| `two_hop_swap` | same | 3+3; user intermediate ATAs |
| `two_hop_swap_v2` | same, fee-excluded output | vault→vault intermediate |

**Fees / rewards**

| Ix | Notes |
|----|--------|
| `update_fees_and_rewards` | Snapshots growth into position; rejects 0-liquidity |
| `collect_fees` / `_v2` | Transfers **already snapshotted** `fee_owed_*`; does **not** update first |
| `collect_reward` / `_v2` | Partial if vault short |
| `collect_protocol_fees` / `_v2` | `collect_protocol_fees_authority` |
| `initialize_reward` / `_v2` | Must be lowest unused index; `reward_authority` |
| `set_reward_emissions` / `_v2` | Vault must cover 1 day of emissions |

**Admin**

`set_fee_rate`, `set_protocol_fee_rate`, `set_default_*`, `set_fee_authority`, `set_collect_protocol_fees_authority`, `set_reward_authority`, `set_reward_authority_by_super_authority`, `set_reward_emissions_super_authority`, adaptive-fee setters, `set_config_feature_flag` (ADMINS), token-badge admin.

### 1.3 Account-constraint summary

Anchor constraints are generally tight:

- Vaults: `address = whirlpool.token_vault_{a,b}`
- Position NFT: `mint == position.position_mint && amount == 1` + owner/delegate signer
- Position PDA: `seeds = [b"position", position_mint]` on open/close/lock
- Oracle: `seeds = [b"oracle", whirlpool]`
- Tick arrays: **unchecked in the struct**, validated in loader (owner, discriminator Fixed/Dynamic, `whirlpool` field, start index)
- V2 token programs: `address = *mint.owner` (prevents spoofing Token vs Token-2022)

Pinocchio re-implements a subset of these (`verify_address`, `verify_constraint`, `load_account_mut` discriminator). Gaps vs Anchor:

- No mint check on **user** token accounts (relies on token program)
- No `has_one` style beyond `position.whirlpool()`
- **No locked-position check on increase** (Anchor increase is dead code under default feature)

---

## 2. Hunt results (requested themes)

### Tick-array manipulation

- PDA seeds shared by Fixed and Dynamic → only one account can exist per `(pool, start_tick)`.
- Swap builder dedups, reorders, and only uses the 3 arrays in the trade direction. Extra/wrong arrays are ignored, not trusted.
- Uninitialized PDA (system + empty) is proxied as zero ticks; `update_tick` on that proxy panics. Cannot hide initialized liquidity.
- **H-01:** DynamicTickArray `rotate_right/left` is performed on a `MAX_LEN` virtual buffer, not `account.data_len()`.

### Liquidity math overflow

- `add_liquidity_delta` / `convert_to_liquidity_delta` use checked math; `liquidity_amount > i128::MAX` rejected.
- Token deltas use u256; `AmountDeltaU64::ExceedsMax` for splash-pool sized moves.
- Tick `liquidity_net` checked_add/sub → `LiquidityNetError`.
- **M-01:** `checked_mul_shift_right(liquidity, growth_delta).unwrap_or(0)` zeros fees/rewards on overflow.

### Fee-growth bugs

- Cross tick: `fee_growth_outside = global.wrapping_sub(outside)` (standard).
- Init convention: all prior growth below the tick if `tick_current >= tick_index`.
- Fees not charged / not grown when `curr_liquidity == 0` (donation to LPs who later enter is *not* created; fees vanish). Known CLMM behavior.
- Collect does not refresh checkpoints (must `update_fees_and_rewards` first). Design, not theft.

### Reward theft

- Collect requires position NFT authority; vault address taken from `reward_infos[i].vault`.
- Uninitialized reward: mint default, transfer of 0.
- Growth overflow → 0 (same as fees). Cannot mint extra rewards.
- Emissions require 1-day vault cover (economic, not a hard solvency invariant).

### Position authority

- Owner or delegate with `delegated_amount == 1`.
- Locked transfer: **owner only** (delegate stripped after first transfer).
- Bundled positions: authority is the **bundle NFT**, PDA `[bundled_position, bundle_mint, index.to_string()]`, bitmap prevents double-open.

### Remaining-accounts abuse

- Typed slices; duplicate type rejected; empty length skipped; supplemental TA cap = 3.
- Transfer-hook extras are passed to `spl_transfer_hook_interface` (hook program id from mint). A **badge-approved** malicious hook + attacker-chosen remaining accounts is a CPI footgun (M-02).

### Token-2022

- v1 Ixs hard-pin `token::ID`.
- v2: transfer-fee included/excluded + inverse-fee verification (100% fee uses `maximum_fee`).
- Unsupported: NonTransferable; unknown TLV types. Freeze authority / PermanentDelegate / TransferHook / MintCloseAuthority / DefaultAccountState / Pausable need a badge.
- TransferFeeConfig, InterestBearing, ScaledUiAmount allowed without badge.
- Vaults: ImmutableOwner on Token-2022.

### Price-limit bypass

- `NO_EXPLICIT_SQRT_PRICE_LIMIT = 0` → MIN (a_to_b) or MAX (b_to_a).
- Direction vs current price enforced.
- Exact-out + implicit limit → `PartialFillError`.
- Exact-in may partially fill; protection is `other_amount_threshold`. Not a protocol bypass if the client sets slippage.

---

## 3. Findings by severity

### Critical

*None confirmed with a clean third-party drain path in this source pass.*

H-01 could become Critical if the current Agave input-buffer layout still concatenates account payloads and the OOB write is committable. That needs a local/fork PoC before raising.

---

### High

#### H-01 — DynamicTickArray `update_tick` writes past account `data_len`

**Component:** `state/dynamic_tick_array.rs`, `pinocchio/state/whirlpool/tick_array/dynamic_tick_array.rs`, `manager/tick_array_manager.rs`  
**Assumption that failed:** W9

`DynamicTickArrayLoader` is:

```rust
pub struct DynamicTickArrayLoader([u8; DynamicTickArray::MAX_LEN]); // includes 8-byte discriminator
pub fn load_mut(data: &mut [u8]) -> &mut DynamicTickArrayLoader {
    unsafe { &mut *(data.as_mut_ptr() as *mut DynamicTickArrayLoader) } // no len check
}
```

It is loaded from `account_data[8..]` (discriminator stripped) but the array type is **MAX_LEN including the discriminator**. `tick_data_mut()` is therefore a ~9952-byte slice regardless of the live account size (`MIN_LEN + n * 112`).

On first initialize / last deinitialize of a tick:

```rust
let shift_data = &mut data_mut[byte_offset..]; // to end of VIRTUAL buffer
shift_data.rotate_right(DynamicTickData::LEN); // 112
```

`rotate_*` reads and writes the entire virtual tail, i.e. **far past `data_len`**.

Liquidity paths:

1. **Increase (new tick):** `resize(+112)` then `update_tick` (rotate_right). New size is only one tick larger, still ≪ MAX_LEN.
2. **Decrease (last liquidity on tick):** `update_tick` (rotate_left) **then** `resize(-112)`.

**Impact (if SBF input region is contiguous and commits sibling accounts):**

- Attacker builds a tx: writable DynamicTickArray + a victim writable account later in the account list.
- Trigger a tick init (open a new tick via `increase_liquidity`) or deinit.
- OOB rotate can smash the next account’s header/payload (lamports, owner, token amount, pool price).

**Why not Critical yet**

- Write contents are a rotate, not a fully arbitrary payload.
- Modern runtime *might* isolate account allocations or fault on OOB (would then be DoS / broken Dynamic TA, not theft).
- Dynamic TAs have been live since 0.5.0 and later Sec3 audits exist; this may have been accepted as “tests use MAX_LEN buffers” or mitigated in-runtime.

**Assumption to falsify:** Agave 2.x copies accounts from a single input buffer without per-account guard pages, and post-ix commit uses the in-buffer bytes of *other* accounts.

**Fix**

- Never view the account as `MAX_LEN`. Use `data.len()` for all slices.
- `rotate_*` only on `[byte_offset..packed_len]` where `packed_len` is computed from the bitmap.
- Assert `required_len <= account.data_len()` before any tick read/write.
- Pinocchio mapper has the same `ticks: [u8; TICKS_MAX_USIZE]` + `rotate_*` pattern — fix both.

---

### Medium

#### M-01 — Fee / reward delta overflow zeroes the LP’s entire unclaimed period

**Component:** `manager/position_manager.rs`

```rust
let fee_delta_a = checked_mul_shift_right(position.liquidity, growth_delta_a).unwrap_or(0);
update.fee_owed_a = position.fee_owed_a.wrapping_add(fee_delta_a);
// same for B and all 3 rewards
```

On overflow the checkpoint is still advanced to the new `fee_growth_inside`. The LP **loses all fees/rewards since last update**, permanently.

**Path (Uniswap-v3 “poisoned position” / donation):**

1. Large LP is stale (has not called `update_fees_and_rewards`).
2. Active liquidity is crushed to a tiny residual (last 1-unit position, or a one-tick gap).
3. Attacker swaps a large notional through that liquidity.  
   `fee_growth += (fee << 64) / L` with `L ≈ 1` explodes.
4. Victim’s next modify/collect-update computes `L_victim * Δgrowth` that does not fit in the u64 fee path → `0`, checkpoint jumps.

`fee_owed_*` wrapping_add is the same Uniswap-v3 “collect before overflow” rule (u64). The **zero-on-overflow** of the mul is worse than saturating or using a wider owed type.

**Impact:** Loss of that LP’s uncollected fees/rewards (not a direct drain of *other* vault tokens to the attacker). Attacker pays swap fees to produce the spike.

**Fix:** Saturate `fee_owed` to `u64::MAX` (or store u128); do not advance checkpoint if the mul overflows; or reject the update so the LP can split/collect incrementally.

#### M-02 — TokenBadge-gated extensions can seize or freeze vaults

**Component:** `util/v2/token.rs` `is_supported_token_mint`

With a badge, the program accepts Permanent Delegate, Transfer Hook, Pausable, DefaultAccountState, MintCloseAuthority, freeze authority.

| Extension | Effect on vault |
|-----------|-----------------|
| Permanent Delegate | Delegate can `transfer` / `burn` vault balance with no pool signature |
| Transfer Hook | Arbitrary CPI using user-supplied remaining accounts |
| Pausable / freeze / DefaultAccountState=Frozen | Permanent DoS of swaps and LP exits |

**Assumption:** Token-badge authority is honest and reviews mint extensions. That is a **governance** control, not an on-chain one.

TransferFeeConfig is allowed **without** a badge. Swap/liquidity math adjusts included vs excluded amounts and checks inverse fee. Collect/decrease send the accounted amount; the user (not the pool) eats the withheld fee. No extra drain found in the fee-included path.

**Fix / residual:** Treat PermanentDelegate + TransferHook as never-safe for vaults (or require the vault to be excluded from delegate). Document badge review as a privileged trust root equal to fee authority.

#### M-03 — Adaptive fee + `set_adaptive_fee_constants` can extract LP surplus

**Component:** `manager/fee_rate_manager.rs`, `instructions/adaptive_fee/set_adaptive_fee_constants.rs`

- Total fee is capped at `FEE_RATE_HARD_LIMIT = 10%` (static max is 6%).
- Protocol cut ≤ 25% of that.
- `set_adaptive_fee_constants` is `fee_authority` and can change volatility mapping **while** a pool’s accumulator is high. The Ix comment already warns of LP revenue loss.

Not unauthorized theft, but a privileged economic lever (MEV / LP expropriation if fee_authority is compromised or malicious). In scope per G6.

---

### Low

#### L-01 — `increase_liquidity` / `_v2` / `_by_token_amounts_v2` ignore the lock

`lock_position` docs: *“Lock the position to prevent any liquidity changes.”*

Decrease, reposition, TE-close check `is_frozen()`. Increase (Pinocchio live path) does not.

Effect: owner (or a pre-lock delegate) can **add** more tokens into a permanently locked position. They cannot withdraw. That is self-grief / unexpected for integrations that treat “locked” as immutable inventory (e.g. voting escrow, institutional lockups).

Not theft of third-party funds.

#### L-02 — `collect_fees` does not snapshot growth

`collect_fees` / `collect_fees_v2` only pay `position.fee_owed_*`. Accrued-but-not-snapshotted growth stays in the checkpoints. Users who omit `update_fees_and_rewards` under-collect. Documented split of Ixs; easy to misuse in custom clients.

Rewards have the same split; at least reward collect handles a **short** vault. Fee collect does not (full amount or fail).

#### L-03 — Permissionless `migrate_repurpose_reward_authority_space`

Anyone may call it. It zeroes `reward_infos[1].extension` and `[2].extension` unless `[2]` is already zero (then `panic!`).

New pools already have `[2] == 0`, so they abort. Old pools lose leftover per-reward authority bytes (intended). Comment says remove after migration. Leftover surface; `panic!` instead of a proper error.

#### L-04 — Pinocchio skips user ATA mint constraints

Comments: *“token program will verify.”* A wrong-mint ATA makes the SPL transfer fail. Safe, but a worse error and a footgun if a future hook path ever skipped the token program.

#### L-05 — `collect_reward` / emissions use `reward_index` as a raw array index in constraints

`reward_index >= 3` panics in constraint evaluation (not a clean `InvalidRewardIndex`). Availability issue only.

#### L-06 — Increase-on-lock + one-sided open sentinels are MEV-sensitive

`tick_lower == i32::MIN` / `tick_upper == i32::MAX` snap to the **current** sqrt-price. Inclusion-time price can differ from simulation. `open_position` later rejects `lower >= upper`. Users must still apply their own slippage on the first `increase`.

---

### Informational

#### I-01 — Design is a mature Uniswap-v3 port with extra defenses

Checked liquidity, sequential tick search, sparse TA builder, transfer-fee inverse check (credit OtterSec in-tree), token-extension allowlist, ImmutableOwner vaults, NFT-based positions, lock via Token-2022 freeze.

Prior public audits listed in upstream README: Kudelski 2022, Neodyme 2022, OtterSec 2024, Sec3 2025 (multiple).

#### I-02 — Admin / upgrade trust

`ADMINS` (feature `mainnet`):

- `GwH3Hiv5mACLX3ufTw1pFsrhSPon5tdw252DBs4Rx4PV` (comment: program upgrade authority, Solana multisig)
- `AqiJTdr9jLPDAk5prGhWFHtSM1qJszAsdZVV7oeinxhh` (Eclipse)

`initialize_config` and `set_config_feature_flag` require an admin key. Fee / protocol-fee / reward-super / badge authorities are then config-controlled and fully trusted.

#### I-03 — Leftover migration Ix

See L-03. Should be removed once all pools report `[2].extension == 0`.

#### I-04 — Exact-in partial fills

With `sqrt_price_limit = 0`, exact-in may stop early (end of provided TAs or zero liquidity). Slippage is entirely `other_amount_threshold`. Routers that set threshold `0` accept any output.

#### I-05 — Two-hop v1 vs v2

v1 moves the intermediate through the user’s ATAs (can be two different accounts). v2 is vault-to-vault and adjusts transfer fees once. Prefer v2 for Token-2022 intermediates.

#### I-06 — InterestBearing / ScaledUiAmount

Supported without badge. Pool accounting is raw token amounts (correct). Off-chain UI that uses scaled amounts can mis-quote. Not an on-chain drain.

#### I-07 — Source / deploy mapping still open

This tree is `0.9.0` with Pinocchio liquidity. Confirm mainnet programdata hash, upgrade slot, and that the deploy used `--features mainnet`.

---

## 4. What was checked and looks solid

- Tick-array PDA + whirlpool binding; cannot inject another pool’s TA.
- Liquidity net/gross overflow/underflow.
- Swap step math: fee on input, u64 remaining checked, protocol fee wrapping_add, growth only if `L > 0`.
- Two-hop mint/amount coupling; no same-pool hop.
- V2 swap ExactIn/ExactOut transfer-fee adjustment + inverse-fee self-check.
- Position NFT mint authority removed after mint; TE positions use MintCloseAuthority = position PDA.
- Close requires empty (liquidity + fees + rewards owed).
- Bundle bitmap open/close pairing.
- Oracle trade-enable + adaptive-fee variable write requires writable initialized oracle.
- `remaining_accounts` type allow-list per Ix.
- Splash-pool (`tick_spacing >= 2^15`) forced full-range positions.

---

## 5. Token Swap V1 / V2 (not in this repo)

This monorepo is **Whirlpools only**. There is no SPL-token-swap / Orca V1/V2 AMM crate here.

| Version | Program ID | Notes |
|---------|------------|--------|
| Orca Token Swap **V1** | `DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1` | Historical constant-product (SPL Token Swap lineage). Not reviewed. |
| Orca Token Swap **V2** | `9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP` | Same family, later deploy. Not reviewed. |
| Whirlpools (this audit) | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | CLMM above. |

**Assumption:** any remaining V1/V2 TVL is a separate target. Need independent source (or dump) of those programs; do not treat this report as coverage for them.

---

## 6. Suggested PoC order (local / fork only)

1. **H-01:** Minimal DynamicTickArray, `increase_liquidity` that initializes the first tick, place a dummy writable account after the TA in the account metas, dump that account before/after. If bytes change → raise to Critical and stop.
2. **M-01:** Fixture with `L_victim` large + stale checkpoint, `L_active = 1`, huge exact-in swap, then `update_fees_and_rewards` on victim; assert `fee_owed` unchanged and checkpoint moved.
3. **L-01:** Lock a TE position, `increase_liquidity_v2`, confirm success.

Do not broadcast any of the above on mainnet.

---

## 7. Residual risk register

| Residual | Why it remains |
|----------|----------------|
| Upgrade / fee / badge authorities | Full control of fees, badges (hence PermanentDelegate), adaptive-fee constants |
| Token-2022 TransferFee on collect | User receives less; pool books are consistent |
| Zero-liquidity fee discard | Fees taken while `L == 0` never enter growth |
| u64 fee_owed wrap | Must collect before 2^64 tokens of fees (impractical on honest pools) |
| Source ≠ deploy | W0/W1/I-07 |
| V1/V2 token-swap | Not in tree |
