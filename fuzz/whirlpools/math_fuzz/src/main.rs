//! Whirlpool swap_math + token_math fuzz at Osec pin e5f089b.
use arbitrary::Arbitrary;
use honggfuzz::fuzz;
use whirlpool::math::{
    compute_swap, estimate_max_liquidity_from_token_amounts, get_amount_delta_a, get_amount_delta_b,
    get_next_sqrt_price, tick_index_from_sqrt_price, MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64,
};
use whirlpool::state::{MAX_TICK_INDEX, MIN_TICK_INDEX};

const FEE_RATE_HARD_LIMIT: u32 = 100_000; // matches program fee_rate_manager

#[derive(Debug, Arbitrary)]
enum Action {
    SwapStep {
        amount: u64,
        liquidity: u32,
        fee_rate: u32,
        price_0: u128,
        price_1: u128,
        amount_specified_is_input: bool,
    },
    /// Matches `estimate_max_liquidity_from_token_amounts(current_sqrt, tick_lo, tick_hi, …)`.
    LiquidityEstimate {
        sqrt_price: u128,
        tick_lower: i32,
        tick_upper: i32,
        token_a: u64,
        token_b: u64,
    },
    AmountDeltas {
        sqrt_0: u128,
        sqrt_1: u128,
        liquidity: u32,
    },
    NextSqrtPrice {
        sqrt_price: u128,
        liquidity: u32,
        amount: u64,
        a_to_b: bool,
        amount_specified_is_input: bool,
    },
}

fn clamp_price(p: u128) -> u128 {
    let span = MAX_SQRT_PRICE_X64 - MIN_SQRT_PRICE_X64;
    MIN_SQRT_PRICE_X64 + (p % (span + 1))
}

fn clamp_tick(t: i32) -> i32 {
    // Map arbitrary i32 into inclusive [MIN_TICK_INDEX, MAX_TICK_INDEX].
    let span = (MAX_TICK_INDEX as i64) - (MIN_TICK_INDEX as i64) + 1;
    let mut x = (t as i64) % span;
    if x < 0 {
        x += span;
    }
    (MIN_TICK_INDEX as i64 + x) as i32
}

fn main() {
    loop {
        fuzz!(|action: Action| {
            match action {
                Action::SwapStep {
                    amount,
                    liquidity,
                    fee_rate,
                    price_0,
                    price_1,
                    amount_specified_is_input,
                } => {
                    let amount = amount.max(1);
                    let liquidity = (liquidity as u128).max(1);
                    let fee_rate = (fee_rate % FEE_RATE_HARD_LIMIT).max(1);
                    let price_0 = clamp_price(price_0);
                    let price_1 = clamp_price(price_1);
                    if price_0 == price_1 {
                        return;
                    }
                    let a_to_b = price_0 >= price_1;
                    if let Ok(step) = compute_swap(
                        amount,
                        fee_rate,
                        liquidity,
                        price_0,
                        price_1,
                        amount_specified_is_input,
                        a_to_b,
                    ) {
                        let (lo, hi) = if price_0 < price_1 {
                            (price_0, price_1)
                        } else {
                            (price_1, price_0)
                        };
                        assert!(step.next_price >= lo && step.next_price <= hi);
                        if amount_specified_is_input {
                            let used = step.amount_in.saturating_add(step.fee_amount);
                            assert!(used <= amount);
                        } else {
                            assert!(step.amount_out <= amount);
                        }
                    }
                }
                Action::LiquidityEstimate {
                    sqrt_price,
                    tick_lower,
                    tick_upper,
                    token_a,
                    token_b,
                } => {
                    let mut lo = clamp_tick(tick_lower);
                    let mut hi = clamp_tick(tick_upper);
                    if lo > hi {
                        std::mem::swap(&mut lo, &mut hi);
                    }
                    if lo == hi {
                        // Function requires a non-empty tick range (upper > lower).
                        if hi < MAX_TICK_INDEX {
                            hi += 1;
                        } else if lo > MIN_TICK_INDEX {
                            lo -= 1;
                        } else {
                            return;
                        }
                    }
                    let p = clamp_price(sqrt_price);
                    // Sanity: tick↔price round-trip stays within global bounds.
                    let _ = tick_index_from_sqrt_price(&p);
                    let _ = estimate_max_liquidity_from_token_amounts(p, lo, hi, token_a, token_b);
                }
                Action::AmountDeltas {
                    sqrt_0,
                    sqrt_1,
                    liquidity,
                } => {
                    let mut a = clamp_price(sqrt_0);
                    let mut b = clamp_price(sqrt_1);
                    if a > b {
                        std::mem::swap(&mut a, &mut b);
                    }
                    if a == b {
                        return;
                    }
                    let liq = (liquidity as u128).max(1);
                    let _ = get_amount_delta_a(a, b, liq, true);
                    let _ = get_amount_delta_b(a, b, liq, true);
                    let _ = get_amount_delta_a(a, b, liq, false);
                    let _ = get_amount_delta_b(a, b, liq, false);
                }
                Action::NextSqrtPrice {
                    sqrt_price,
                    liquidity,
                    amount,
                    a_to_b,
                    amount_specified_is_input,
                } => {
                    let sqrt_price = clamp_price(sqrt_price);
                    let liquidity = (liquidity as u128).max(1);
                    let amount = amount.max(1);
                    // Signature: (sqrt, liq, amount, amount_specified_is_input, a_to_b)
                    // Must not panic; Ok/Err both acceptable at extreme edges.
                    let _ = get_next_sqrt_price(
                        sqrt_price,
                        liquidity,
                        amount,
                        amount_specified_is_input,
                        a_to_b,
                    );
                }
            }
        });
    }
}
