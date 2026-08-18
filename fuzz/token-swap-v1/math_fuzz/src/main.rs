//! Pure-math fuzzer for Orca Token Swap V1 lineage invariants.
//! Covers constant-product swap + StableCurve (token-swap-v2.0.0) + fee helper.
use arbitrary::Arbitrary;
use honggfuzz::fuzz;

const N_COINS: u8 = 2;

#[derive(Debug, Arbitrary, Clone)]
struct Fees {
    trade_num: u16,
    trade_den: u16,
    owner_num: u16,
    owner_den: u16,
}

fn calc_fee(amount: u128, num: u128, den: u128) -> Option<u128> {
    if num == 0 || amount == 0 {
        return Some(0);
    }
    if den == 0 {
        return None; // invalid — must not be used unchecked on-chain
    }
    let fee = amount.checked_mul(num)?.checked_div(den)?;
    Some(if fee == 0 { 1 } else { fee })
}

/// Uniswap-style CP: dy = y * dx / (x + dx)  (no fees)
fn cp_swap(x: u128, y: u128, dx: u128) -> Option<u128> {
    if x == 0 || y == 0 || dx == 0 {
        return None;
    }
    let num = y.checked_mul(dx)?;
    let den = x.checked_add(dx)?;
    num.checked_div(den)
}

/// Simplified Stable get_d (2-coin) from SPL token-swap-v2.0.0 logic shape.
fn stable_get_d(amount_a: u128, amount_b: u128, amp: u64) -> Option<u128> {
    let sum = amount_a.checked_add(amount_b)?;
    if sum == 0 {
        return Some(0);
    }
    let n = N_COINS as u128;
    let amp_n = (amp as u128).checked_mul(n)?;
    let mut d = sum;
    for _ in 0..32 {
        let mut d_p = d;
        d_p = d_p.checked_mul(d)?.checked_div(amount_a.checked_mul(n)?)?;
        d_p = d_p.checked_mul(d)?.checked_div(amount_b.checked_mul(n)?)?;
        let d_prev = d;
        // D = (amp_n * sum + d_p * n) * D / ((amp_n - 1) * D + (n + 1) * d_p)
        let numerator = amp_n
            .checked_mul(sum)?
            .checked_add(d_p.checked_mul(n)?)?
            .checked_mul(d)?;
        let denominator = amp_n
            .checked_sub(1)?
            .checked_mul(d)?
            .checked_add(d_p.checked_mul(n.checked_add(1)?)?)?;
        if denominator == 0 {
            return None;
        }
        d = numerator.checked_div(denominator)?;
        if d > d_prev {
            if d - d_prev <= 1 {
                return Some(d);
            }
        } else if d_prev - d <= 1 {
            return Some(d);
        }
    }
    Some(d)
}

#[derive(Debug, Arbitrary)]
enum Action {
    CpSwap { x: u64, y: u64, dx: u64 },
    StableD { a: u64, b: u64, amp: u16 },
    Fee { amount: u64, fees: Fees },
}

fn main() {
    loop {
        fuzz!(|action: Action| {
            match action {
                Action::CpSwap { x, y, dx } => {
                    let x = (x as u128) % (1u128 << 80) + 1;
                    let y = (y as u128) % (1u128 << 80) + 1;
                    let dx = (dx as u128) % (1u128 << 80);
                    if let Some(dy) = cp_swap(x, y, dx) {
                        // output cannot exceed pool
                        assert!(dy <= y, "CP dy>y x={x} y={y} dx={dx} dy={dy}");
                        // with fees removed later; conservation: x' * y' >= x * y roughly for zero fee
                        let xp = x.saturating_add(dx);
                        let yp = y.saturating_sub(dy);
                        // constant product non-decrease (floor division may leave dust)
                        assert!(
                            xp.saturating_mul(yp) + xp + yp >= x.saturating_mul(y),
                            "CP k decreased more than dust allows"
                        );
                    }
                }
                Action::StableD { a, b, amp } => {
                    let a = (a as u128) % (1u128 << 60) + 1;
                    let b = (b as u128) % (1u128 << 60) + 1;
                    let amp = (amp as u64) % 10_000;
                    // amp==0: leverage 0 — expect None or fail closed
                    let d = stable_get_d(a, b, amp);
                    if amp == 0 {
                        // must not panic; None or Some is ok if finite
                        let _ = d;
                    } else if let Some(d) = d {
                        assert!(d > 0, "D==0 with nonzero balances");
                        // D should be in ballpark of sum for high amp, >= min(a,b)*2-ish
                        assert!(d < a.saturating_add(b).saturating_mul(4).saturating_add(1_000_000), "D exploded");
                    }
                }
                Action::Fee { amount, fees } => {
                    let amount = amount as u128;
                    // Mirror on-chain Fees::validate_fraction: num < den, or both 0.
                    // Invalid fractions are rejected at Initialize — do not treat as vault bugs.
                    let trade_ok = (fees.trade_num == 0 && fees.trade_den == 0)
                        || (fees.trade_den != 0 && fees.trade_num < fees.trade_den);
                    let owner_ok = (fees.owner_num == 0 && fees.owner_den == 0)
                        || (fees.owner_den != 0 && fees.owner_num < fees.owner_den);

                    let r1 = calc_fee(amount, fees.trade_num as u128, fees.trade_den as u128);
                    let r2 = calc_fee(amount, fees.owner_num as u128, fees.owner_den as u128);
                    if fees.trade_den == 0 && fees.trade_num != 0 {
                        assert!(r1.is_none(), "fee with den=0 must fail closed");
                    }
                    // Only assert fee ≤ amount for fractions that could pass init validation.
                    if trade_ok {
                        if let Some(f) = r1 {
                            assert!(
                                f <= amount || amount == 0,
                                "valid-fraction fee > amount (trade)"
                            );
                        }
                    }
                    if owner_ok {
                        if let Some(f) = r2 {
                            assert!(
                                f <= amount || amount == 0,
                                "valid-fraction fee > amount (owner)"
                            );
                        }
                    }
                    if trade_ok && owner_ok {
                        if let (Some(f1), Some(f2)) = (r1, r2) {
                            assert!(
                                f1.saturating_add(f2) <= amount.saturating_add(amount / 2).saturating_add(2)
                                    || amount == 0,
                                "combined valid fees absurdly large"
                            );
                        }
                    }
                }
            }
        });
    }
}
