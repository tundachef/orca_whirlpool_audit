//! Wavebreak client math fuzz against orca_wavebreak 2.0.0 (public crate).
//! Closed-source on-chain ELF cannot be proven identical; this validates client math.
use arbitrary::Arbitrary;
use honggfuzz::fuzz;
use orca_wavebreak::curve::{
    curve as bezier_bins, exponential, flat, linear, sigmoid, PriceCurveFacade, MAX_SQRT_PRICE,
    MIN_GRADUATION_TARGET, MIN_SQRT_PRICE,
};
use orca_wavebreak::fee::BPS_DENOMINATOR;
use orca_wavebreak::price::{base_to_quote_amount, quote_to_base_amount};
use orca_wavebreak::quote::{
    exact_in_buy_quote, exact_in_sell_quote, exact_out_buy_quote, exact_out_sell_quote,
    graduate_quote,
};

#[derive(Debug, Arbitrary)]
enum Action {
    Quotes {
        start_price: u128,
        end_price: u128,
        cp0: u16,
        cp1: u16,
        cp2: u16,
        cp3: u16,
        amount: u64,
        fee_bps: u16,
        current_quote: u64,
        graduation_target: u64,
        max_amount: u64,
        kind: u8,
    },
    PriceConvert {
        sqrt_price: u128,
        amount: u64,
        round_up: bool,
        quote_to_base: bool,
    },
    CurveBuilders {
        price_a: u128,
        price_b: u128,
        rate: u16,
        which: u8,
    },
    Graduate {
        start_price: u128,
        end_price: u128,
        cp0: u16,
        cp1: u16,
        cp2: u16,
        cp3: u16,
        split_bps: u16,
        quote_amount: u64,
        creator_reward: u64,
        graduation_reward: u64,
        quote_protocol_fee_bps: u16,
    },
}

fn clamp_sqrt(p: u128) -> u128 {
    let span = MAX_SQRT_PRICE - MIN_SQRT_PRICE;
    MIN_SQRT_PRICE + (p % (span + 1))
}

fn clamp_bps(b: u16) -> u16 {
    b % (BPS_DENOMINATOR + 1)
}

fn make_curve(
    start_price: u128,
    end_price: u128,
    cp0: u16,
    cp1: u16,
    cp2: u16,
    cp3: u16,
) -> Option<PriceCurveFacade> {
    let mut start = clamp_sqrt(start_price);
    let mut end = clamp_sqrt(end_price);
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    let facade = PriceCurveFacade {
        start_price: start,
        end_price: end,
        control_points: [
            clamp_bps(cp0),
            clamp_bps(cp1),
            clamp_bps(cp2),
            clamp_bps(cp3),
        ],
    };
    if facade.is_valid() {
        Some(facade)
    } else {
        None
    }
}

fn clamp_grad(g: u64) -> u64 {
    // Keep targets in a range that exercises bins without instantly overflowing loops.
    let base = g.max(MIN_GRADUATION_TARGET);
    // Cap to avoid pathological multi-bin walks with huge u64 spans in fuzz timeouts.
    base.min(10_000_000_000_000) // 1e13
}

fn main() {
    loop {
        fuzz!(|action: Action| {
            match action {
                Action::Quotes {
                    start_price,
                    end_price,
                    cp0,
                    cp1,
                    cp2,
                    cp3,
                    amount,
                    fee_bps,
                    current_quote,
                    graduation_target,
                    max_amount,
                    kind,
                } => {
                    let Some(curve) = make_curve(start_price, end_price, cp0, cp1, cp2, cp3) else {
                        return;
                    };
                    let grad = clamp_grad(graduation_target);
                    let current = current_quote % (grad.saturating_add(1));
                    let amount = amount.max(1);
                    let fee = clamp_bps(fee_bps);
                    let max_amount = max_amount.max(1);
                    // Must not panic; Err is fine.
                    let q = match kind % 4 {
                        0 => exact_in_buy_quote(curve, amount, fee, current, grad, max_amount),
                        1 => exact_out_buy_quote(curve, amount, fee, current, grad, max_amount),
                        2 => exact_in_sell_quote(curve, amount, fee, current, grad, max_amount),
                        _ => exact_out_sell_quote(curve, amount, fee, current, grad, max_amount),
                    };
                    if let Ok(quote) = q {
                        // Fee is always denominated in quote token.
                        match kind % 4 {
                            0 | 1 => {
                                // Buys: fee taken from quote input.
                                assert!(quote.fee_amount <= quote.amount_in);
                            }
                            _ => {
                                // Sells: fee taken from quote output (pre-fee out = out + fee).
                                let pre = quote.amount_out.saturating_add(quote.fee_amount);
                                assert!(quote.fee_amount <= pre);
                            }
                        }
                    }
                }
                Action::PriceConvert {
                    sqrt_price,
                    amount,
                    round_up,
                    quote_to_base,
                } => {
                    let p = clamp_sqrt(sqrt_price);
                    let amount = amount.max(1);
                    if quote_to_base {
                        let _ = quote_to_base_amount(p, amount, round_up);
                    } else {
                        let _ = base_to_quote_amount(p, amount, round_up);
                    }
                }
                Action::CurveBuilders {
                    price_a,
                    price_b,
                    rate,
                    which,
                } => {
                    let a = clamp_sqrt(price_a);
                    let mut b = clamp_sqrt(price_b);
                    if a > b {
                        b = a;
                    }
                    let rate = clamp_bps(rate);
                    let facade = match which % 4 {
                        0 => flat(a),
                        1 => linear(a, b),
                        2 => exponential(a, b, rate),
                        _ => sigmoid(a, b, rate),
                    };
                    assert!(facade.is_valid());
                    // Builder curves should produce monotone non-decreasing x bins.
                    if let Ok(bins) = bezier_bins(facade) {
                        let mut prev_x = 0u16;
                        for bin in bins.0.iter() {
                            assert!(bin.x >= prev_x, "non-monotone bezier x from builder");
                            prev_x = bin.x;
                        }
                    }
                }
                Action::Graduate {
                    start_price,
                    end_price,
                    cp0,
                    cp1,
                    cp2,
                    cp3,
                    split_bps,
                    quote_amount,
                    creator_reward,
                    graduation_reward,
                    quote_protocol_fee_bps,
                } => {
                    let Some(curve) = make_curve(start_price, end_price, cp0, cp1, cp2, cp3) else {
                        return;
                    };
                    let split = clamp_bps(split_bps);
                    let fee = clamp_bps(quote_protocol_fee_bps);
                    let quote_amount = quote_amount.max(MIN_GRADUATION_TARGET);
                    // Must not panic; Err expected when rewards+fees exceed quote.
                    let _ = graduate_quote(
                        curve,
                        split,
                        quote_amount,
                        creator_reward,
                        graduation_reward,
                        fee,
                    );
                }
            }
        });
    }
}
