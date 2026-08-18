# 06 — Orca xORCA staking technical audit

**Program ID:** `StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT`  
**Source:** `sources/xorca` @ `05fe66b` (HEAD)  
**Tag `v1.0.0`:** `6fb847d` — program delta vs HEAD is **comment-only** in `util/math.rs` (incinerator note)  
**xORCA mint:** `xorcaYqbXUNz3474ubUMJAdu2xgPsew3rUCe5ughT3N`  
**Upgrade authority:** `GwH3Hiv5…`  
**Initialize deployer:** `94kZD71s…` (hardcoded; distinct from upgrade auth)  
**External audit:** Sec3 2025-09-24 (`audits-external/xorca/2025-09-24.pdf`) — H-01 inflation **Resolved** via virtual offset  
**Date:** 2026-08-18  
**Status:** **PHASE-COMPLETE**

---

## 1. Identity

| Check | Result |
|-------|--------|
| `declare_id!` | matches mainnet |
| Mint decimals | ORCA + xORCA enforced **6** at init |
| Mint authority | must be **state PDA**; freeze authority must be **None** |
| Vault | ATA of state PDA (seeds verified via stored `vault_bump`) |
| Supply never 0 | incinerator burn of 1 atomic xORCA (documented in math.rs) |

---

## 2. Instruction surface

| Ix | Auth | Role |
|----|------|------|
| `Initialize` | **DEPLOYER_ADDRESS** only | Create state + vault ATA; set `update_authority`, `cool_down_period_s` |
| `Stake` | staker signer | Transfer ORCA → vault; mint xORCA |
| `Unstake` | unstaker signer | Burn xORCA; escrow ORCA; create `PendingWithdraw` PDA |
| `Withdraw` | unstaker signer | After cool-down; transfer escrowed ORCA; close pending |
| `Set` / `UpdateCoolDownPeriod` | `update_authority` | Set cool-down (`>= 0` only) |
| `Set` / `UpdateUpdateAuthority` | `update_authority` | Single-step rotate |

Small, clear surface (Pinocchio). No fee switch in-program — yield is external donation to vault.

---

## 3. Share math

```
shares_out = orca_in * (supply + 100) / (non_escrowed + 100)   // floor
orca_out   = xorca_in * (non_escrowed + 100) / (supply + 100) // floor
```

Special case: if `supply == 0` **or** `non_escrowed == 0` → stake mints **1:1**.

Virtual offset `100` implements Sec3 H-01 mitigation (OpenZeppelin ERC-4626 virtual assets pattern).

---

## 4. Findings

| ID | Sev | Title |
|----|-----|-------|
| **XO-H01** | **High** (trust) | Upgrade authority `GwH3…` (same concentration as Whirlpools/Wavebreak) can replace program |
| **XO-M01** | Medium (ops) | `UpdateCoolDownPeriod` allows **0** and has **no upper bound** — only `< 0` rejected. Affects **new** unstakes only (timestamps snapshotted at unstake). `0` = instant withdraw path; huge values risk `CoolDownOverflow` / long lock |
| **XO-M02** | Medium (residual) | Virtual offset **100** still allows dust / small stakes to mint **0 shares** after large vault donations (`InsufficientStakeAmount`). Classic residual of small virtual offsets; Sec3 H-01 marked Resolved — residual griefing remains |
| **XO-M03** | Medium (UX) | When `non_escrowed == 0` but `supply > 0` (all vault escrowed), stake uses 1:1 special case which **under-mints** vs the virtual-offset formula — hostile to new stakers in fully-escrowed state, not a vault drain |
| **XO-M04** | Medium (ops) | `UpdateUpdateAuthority` is single-step to any pubkey — mis-set ⇒ permanent lockout of cool-down/auth updates |
| **XO-L01** | Low | `withdraw_index: u8` ⇒ max **256** concurrent pending withdraws per user |
| **XO-I01** | Info | Deployer ≠ upgrade authority (good init separation) |
| **XO-I02** | Info | Cool-down change does not rewrite existing `PendingWithdraw.withdrawable_timestamp` |
| **XO-I03** | Info | Sec3 L-01 (pre-created PDA/ATA) **Acknowledged**; L-02/L-04 Resolved (`create_program_account_secure`) |
| **XO-I04** | Info | Escrow accounting: `vault - escrowed` with `InsufficientVaultBacking` guard |

**Critical unprivileged drain:** **NOT FOUND** at this pin.

---

## 5. Cool-down / escrow model

1. Unstake burns xORCA, adds `withdrawable_orca` to `state.escrowed_orca_amount`, opens pending PDA with `now + cool_down`.
2. Withdraw requires `clock >= withdrawable_timestamp`, transfers from vault, decrements escrow, closes pending to unstaker.
3. Pending PDA seeds: `[pending_withdraw, unstaker, withdraw_index]` — user-funded; secure create path per Sec3 L-04 fix.

---

## 6. Fuzz

| Campaign | Result |
|----------|--------|
| Harness | `fuzz/xorca/math_fuzz/` (mirrored convert_* + virtual offset) |
| Clean run | **764,737 iters / 91s / 0 crashes / 0 timeouts** |
| Log | `fuzz/xorca/logs/math_fuzz_clean90.txt` |
| Notes | Initial run had harness FPs from `saturating_add` near `u64::MAX` (not program bugs); fixed to `checked_add` and re-run clean |

Invariants held: immediate round-trip of newly minted shares does not profit; full-supply redeem bounded by vault + virtual slack; `non_escrowed==0` special case returns 1:1.

---

## 7. Recommendations

1. Bound cool-down: e.g. `0 < period <= MAX_COOLDOWN` (or allow 0 explicitly as a documented “liquid” mode).
2. Consider larger virtual offset or dead shares if dust griefing after donations matters operationally.
3. Replace `non_escrowed == 0` special case with virtual-offset formula (or reject stake while fully escrowed).
4. Ops: confirm `update_authority` and `GwH3…` custody (multisig/HSM).

---

## 8. Phase DoD

| Item | Status |
|------|--------|
| Identity / mint / vault | done |
| Ix + auth map | done |
| Math + Sec3 cross-check | done |
| Fuzz | 765k / 0 crashes |
| Report | this file |
| Git commit | this close-out |

**Next:** #7 Whirlpools Immutable (`iwhrLH…`) — diff vs mutable; auth burned.
