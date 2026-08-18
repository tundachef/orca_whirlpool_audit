use super::curve::PriceCurveFacade;

use super::error::{AMOUNT_EXCEEDS_MAX_U64, ARITHMETIC_OVERFLOW};

use super::quote::{Quote, QuoteType};
use super::{curve::PriceCurve, error::CoreError, U128};

use ethnum::U256;

#[cfg(feature = "floats")]
use libm::{floor, pow, sqrt};

#[cfg(feature = "floats")]
const Q64_RESOLUTION: f64 = 18446744073709551616.0;

use super::fee::BPS_DENOMINATOR;

#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

/// Get the price of the base token expressed in the quote token. This is the spot price
/// and not a quote for a specific amount.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn token_price(
    price_curve: PriceCurveFacade,
    quote_amount: u64,
    graduation_target: u64,
) -> Result<U128, CoreError> {
    if quote_amount == graduation_target {
        return Ok(price_curve.end_price.into());
    }
    let price_curve: PriceCurve = price_curve.try_into()?;
    price_curve.sqrt_price(quote_amount, graduation_target)
}

/// Convert a price into a sqrt priceX64
/// IMPORTANT: floating point operations can reduce the precision of the result.
/// Make sure to do these operations last and not to use the result for further calculations.
///
/// # Parameters
/// * `price` - The price to convert
/// * `decimals_a` - The number of decimals of the base token
/// * `decimals_b` - The number of decimals of the quote token
///
/// # Returns
/// * `u128` - The sqrt priceX64
#[cfg(feature = "floats")]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn price_to_sqrt_price(price: f64, decimals_a: u8, decimals_b: u8) -> U128 {
    let power = pow(10f64, decimals_a as f64 - decimals_b as f64);
    (floor(sqrt(price / power) * Q64_RESOLUTION) as u128).into()
}

/// Convert a sqrt priceX64 into a tick index
/// IMPORTANT: floating point operations can reduce the precision of the result.
/// Make sure to do these operations last and not to use the result for further calculations.
///
/// # Parameters
/// * `sqrt_price` - The sqrt priceX64 to convert
/// * `decimals_a` - The number of decimals of the base token
/// * `decimals_b` - The number of decimals of the quote token
///
/// # Returns
/// * `f64` - The decimal price
#[cfg(feature = "floats")]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn sqrt_price_to_price(sqrt_price: U128, decimals_a: u8, decimals_b: u8) -> f64 {
    let power = pow(10f64, decimals_a as f64 - decimals_b as f64);
    let sqrt_price: u128 = sqrt_price.into();
    pow(sqrt_price as f64 / Q64_RESOLUTION, 2.0) * power
}

// Given a current spot price and a quote amount, return the base amount that would be
// bought or sold for the quote amount.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn quote_to_base_amount(
    sqrt_price: U128,
    quote_amount: u64,
    round_up: bool,
) -> Result<u64, CoreError> {
    let sqrt_price: u128 = sqrt_price.into();

    let numerator = U256::from(quote_amount)
        .checked_shl(128)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let denominator = U256::from(sqrt_price)
        .checked_mul(U256::from(sqrt_price))
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let quotient: U256 = numerator
        .checked_div(denominator)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let remainder = numerator
        .checked_rem(denominator)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let result = if remainder > 0 && round_up {
        quotient + 1
    } else {
        quotient
    };

    result.try_into().map_err(|_| AMOUNT_EXCEEDS_MAX_U64)
}

/// Given a current spot price and a base amount, return the quote amount that would be
/// bought or sold for the base amount.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn base_to_quote_amount(
    sqrt_price: U128,
    base_amount: u64,
    round_up: bool,
) -> Result<u64, CoreError> {
    let sqrt_price: u128 = sqrt_price.into();

    let price = U256::from(sqrt_price)
        .checked_mul(U256::from(sqrt_price))
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let unshifted = U256::from(base_amount)
        .checked_mul(price)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let product = unshifted.checked_shr(128).ok_or(ARITHMETIC_OVERFLOW)?;
    let remainder = unshifted & u128::MAX;

    let result = if remainder > 0 && round_up {
        product + 1
    } else {
        product
    };

    result.try_into().map_err(|_| AMOUNT_EXCEEDS_MAX_U64)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub struct PriceThreshold(u64, u64);

impl From<(u64, u64)> for PriceThreshold {
    fn from((numerator, denominator): (u64, u64)) -> Self {
        Self(numerator, denominator)
    }
}

impl From<&(u64, u64)> for PriceThreshold {
    fn from((numerator, denominator): &(u64, u64)) -> Self {
        Self(*numerator, *denominator)
    }
}

impl From<PriceThreshold> for (u64, u64) {
    fn from(PriceThreshold(numerator, denominator): PriceThreshold) -> Self {
        (numerator, denominator)
    }
}

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn slippage_threshold(
    quote: Quote,
    slippage_tolerance_bps: u16,
) -> Result<PriceThreshold, CoreError> {
    let other_amount = match quote.quote_type {
        QuoteType::BuyExactIn | QuoteType::SellExactIn => quote.amount_out,
        QuoteType::BuyExactOut | QuoteType::SellExactOut => quote.amount_in,
    };

    let product = match quote.quote_type {
        QuoteType::BuyExactIn | QuoteType::SellExactIn => BPS_DENOMINATOR
            .checked_sub(slippage_tolerance_bps)
            .ok_or(ARITHMETIC_OVERFLOW)?,
        QuoteType::BuyExactOut | QuoteType::SellExactOut => BPS_DENOMINATOR
            .checked_add(slippage_tolerance_bps)
            .ok_or(ARITHMETIC_OVERFLOW)?,
    };

    let numerator = u128::from(other_amount)
        .checked_mul(product as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let quotient = numerator
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let remainder = numerator
        .checked_rem(BPS_DENOMINATOR as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let should_round_up = match quote.quote_type {
        QuoteType::BuyExactIn | QuoteType::SellExactIn => true,
        QuoteType::BuyExactOut | QuoteType::SellExactOut => false,
    };

    let other_amount_threshold = if should_round_up && remainder > 0 {
        quotient + 1
    } else {
        quotient
    }
    .try_into()
    .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;

    // quote
    let numerator = match quote.quote_type {
        QuoteType::BuyExactIn => quote.amount_in,
        QuoteType::BuyExactOut | QuoteType::SellExactIn => other_amount_threshold,
        QuoteType::SellExactOut => quote.amount_out,
    };

    // base
    let denominator = match quote.quote_type {
        QuoteType::BuyExactOut => quote.amount_out,
        QuoteType::BuyExactIn | QuoteType::SellExactOut => other_amount_threshold,
        QuoteType::SellExactIn => quote.amount_in,
    };

    Ok((numerator, denominator).into())
}

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn slippage_threshold_exceeded(
    quote: Quote,
    price_threshold: PriceThreshold,
) -> Result<bool, CoreError> {
    // numerator / denominator = actual_quote / actual_base
    // numerator * actual_base = denominator * actual_quote

    let (numerator, denominator) = price_threshold.into();

    let actual_quote = match quote.quote_type {
        QuoteType::BuyExactIn | QuoteType::BuyExactOut => quote.amount_in,
        QuoteType::SellExactIn | QuoteType::SellExactOut => quote.amount_out,
    };

    let actual_base = match quote.quote_type {
        QuoteType::BuyExactIn | QuoteType::BuyExactOut => quote.amount_out,
        QuoteType::SellExactIn | QuoteType::SellExactOut => quote.amount_in,
    };

    let lhs = u128::from(numerator)
        .checked_mul(actual_base as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?;
    let rhs = u128::from(denominator)
        .checked_mul(actual_quote as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    match quote.quote_type {
        QuoteType::BuyExactIn | QuoteType::BuyExactOut => Ok(lhs < rhs),
        QuoteType::SellExactIn | QuoteType::SellExactOut => Ok(lhs > rhs),
    }
}

#[cfg(all(test, feature = "lib"))]
mod tests {
    use super::*;
    use crate::{
        curve::{linear, MAX_SQRT_PRICE, MIN_SQRT_PRICE},
        error::BEYOND_GRADUATION_TARGET,
    };
    use rstest::rstest;

    #[rstest]
    #[case(0, Ok(18589706340280800641))]
    #[case(500096, Ok(27814000714339261926))]
    #[case(1000191, Ok(36748681206440483251))]
    #[case(1000192, Ok(36893488147419103232))]
    #[case(1000193, Err(BEYOND_GRADUATION_TARGET))]
    fn test_token_price(
        #[case] quote_vault_amount: u64,
        #[case] expected_token_price: Result<u128, CoreError>,
    ) {
        let curve = linear(1 << 64, 2 << 64);
        let token_price = token_price(curve, quote_vault_amount, 1000192).map(|x| x.into());
        assert_eq!(token_price, expected_token_price);
    }

    #[rstest]
    #[case(1.0, 9, 9, 1 << 64)]
    #[case(1000.0, 9, 6, 1 << 64)]
    #[case(0.001, 6, 9, 1 << 64)]
    #[case(1.0, 6, 6, 1 << 64)]
    #[case(0.0, 6, 6, 0)]
    fn test_price_to_sqrt_price(
        #[case] price: f64,
        #[case] decimals_a: u8,
        #[case] decimals_b: u8,
        #[case] expected_sqrt_price: u128,
    ) {
        let sqrt_price = price_to_sqrt_price(price, decimals_a, decimals_b);
        assert_eq!(sqrt_price, expected_sqrt_price);
    }

    #[rstest]
    #[case(1 << 64, 9, 9, 1.0)]
    #[case(1 << 64, 9, 6, 1000.0)]
    #[case(1 << 64, 6, 9, 0.001)]
    #[case(1 << 64, 6, 6, 1.0)]
    #[case(0, 6, 6, 0.0)]
    #[case(MIN_SQRT_PRICE, 9, 9, 9.371668998785515e-14)]
    #[case(MAX_SQRT_PRICE, 9, 9, 10670457952892.91)]
    fn test_sqrt_price_to_price(
        #[case] sqrt_price: u128,
        #[case] decimals_a: u8,
        #[case] decimals_b: u8,
        #[case] expected_price: f64,
    ) {
        let price = sqrt_price_to_price(sqrt_price.into(), decimals_a, decimals_b);
        assert_eq!(price, expected_price);
    }

    #[rstest]
    #[case(1 << 64, 100, true, 100)] // price = 1 B/A
    #[case(1 << 64, 100, false, 100)] // price = 1 B/A
    #[case(2 << 64, 100, true, 25)] // price = 4 B/A
    #[case(2 << 64, 100, false, 25)] // price = 4 B/A
    #[case(4 << 64, 100, true, 7)] // price = 16 B/A
    #[case(4 << 64, 100, false, 6)] // price = 16 B/A
    #[case((1 << 64) / 2, 100, true, 400)] // price = 0.25 B/A
    #[case((1 << 64) / 2, 100, false, 400)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 100, true, 1600)] // price = 0.0625 B/A
    #[case((1 << 64) / 4, 100, false, 1600)] // price = 0.0625 B/A
    fn test_quote_to_base_amount(
        #[case] sqrt_price: u128,
        #[case] quote_amount: u64,
        #[case] round_up: bool,
        #[case] expected_base_amount: u64,
    ) {
        let base_amount = quote_to_base_amount(sqrt_price.into(), quote_amount, round_up).unwrap();
        assert_eq!(base_amount, expected_base_amount);
    }

    #[rstest]
    #[case(1 << 64, 100, true, 100)] // price = 1 B/A
    #[case(1 << 64, 100, false, 100)] // price = 1 B/A
    #[case(2 << 64, 100, true, 400)] // price = 4 B/A
    #[case(2 << 64, 100, false, 400)] // price = 4 B/A
    #[case(4 << 64, 100, true, 1600)] // price = 16 B/A
    #[case(4 << 64, 100, false, 1600)] // price = 16 B/A
    #[case((1 << 64) / 2, 100, true, 25)] // price = 0.25 B/A
    #[case((1 << 64) / 2, 100, false, 25)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 100, true, 7)] // price = 0.0625 B/A
    #[case((1 << 64) / 4, 100, false, 6)] // price = 0.0625 B/A
    fn test_base_to_quote_amount(
        #[case] sqrt_price: u128,
        #[case] base_amount: u64,
        #[case] round_up: bool,
        #[case] expected_quote_amount: u64,
    ) {
        let quote_amount = base_to_quote_amount(sqrt_price.into(), base_amount, round_up).unwrap();
        assert_eq!(quote_amount, expected_quote_amount);
    }

    #[rstest]
    #[case(1000, (100, 100), false)]
    #[case(999, (100, 100), true)]
    #[case(1001, (100, 100), false)]
    #[case(1000, (100, 99), false)]
    #[case(1000, (100, 101), true)]
    #[case(1000, (99, 100), true)]
    #[case(1000, (101, 100), false)]
    fn test_slippage_threshold_exceeded_buy_exact_in(
        #[case] actual_amount_out: u64,
        #[case] price_threshold: (u64, u64),
        #[case] result: bool,
    ) {
        let quote = Quote {
            quote_type: QuoteType::BuyExactIn,
            amount_in: 1000,
            amount_out: actual_amount_out,
            fee_amount: 0,
        };
        let slippage_threshold_exceeded =
            slippage_threshold_exceeded(quote, price_threshold.into()).unwrap();
        assert_eq!(slippage_threshold_exceeded, result);
    }

    #[rstest]
    #[case(1000, (100, 100), false)]
    #[case(999, (100, 100), false)]
    #[case(1001, (100, 100), true)]
    #[case(1000, (100, 99), false)]
    #[case(1000, (100, 101), true)]
    #[case(1000, (99, 100), true)]
    #[case(1000, (101, 100), false)]
    fn test_slippage_threshold_exceeded_buy_exact_out(
        #[case] actual_amount_in: u64,
        #[case] price_threshold: (u64, u64),
        #[case] result: bool,
    ) {
        let quote = Quote {
            quote_type: QuoteType::BuyExactOut,
            amount_in: actual_amount_in,
            amount_out: 1000,
            fee_amount: 0,
        };
        let slippage_threshold_exceeded =
            slippage_threshold_exceeded(quote, price_threshold.into()).unwrap();
        assert_eq!(slippage_threshold_exceeded, result);
    }

    #[rstest]
    #[case(1000, (100, 100), false)]
    #[case(999, (100, 100), true)]
    #[case(1001, (100, 100), false)]
    #[case(1000, (100, 99), true)]
    #[case(1000, (100, 101), false)]
    #[case(1000, (99, 100), false)]
    #[case(1000, (101, 100), true)]
    fn test_slippage_threshold_exceeded_sell_exact_in(
        #[case] actual_amount_out: u64,
        #[case] price_threshold: (u64, u64),
        #[case] result: bool,
    ) {
        let quote = Quote {
            quote_type: QuoteType::SellExactIn,
            amount_in: 1000,
            amount_out: actual_amount_out,
            fee_amount: 0,
        };
        let slippage_threshold_exceeded =
            slippage_threshold_exceeded(quote, price_threshold.into()).unwrap();
        assert_eq!(slippage_threshold_exceeded, result);
    }

    #[rstest]
    #[case(1000, (100, 100), false)]
    #[case(999, (100, 100), false)]
    #[case(1001, (100, 100), true)]
    #[case(1000, (100, 99), true)]
    #[case(1000, (100, 101), false)]
    #[case(1000, (99, 100), false)]
    #[case(1000, (101, 100), true)]
    fn test_slippage_threshold_exceeded_sell_exact_out(
        #[case] actual_amount_in: u64,
        #[case] price_threshold: (u64, u64),
        #[case] result: bool,
    ) {
        let quote = Quote {
            quote_type: QuoteType::SellExactOut,
            amount_in: actual_amount_in,
            amount_out: 1000,
            fee_amount: 0,
        };
        let slippage_threshold_exceeded =
            slippage_threshold_exceeded(quote, price_threshold.into()).unwrap();
        assert_eq!(slippage_threshold_exceeded, result);
    }

    #[rstest]
    #[case(100, 100000, 99000)]
    #[case(100, 10000, 9900)]
    #[case(100, 5000, 4950)]
    #[case(100, 1000, 990)]
    #[case(100, 500, 495)]
    #[case(100, 100, 99)]
    #[case(100, 10, 10)]
    #[case(20, 100000, 99800)]
    #[case(20, 10000, 9980)]
    #[case(20, 5000, 4990)]
    #[case(20, 1000, 998)]
    #[case(20, 500, 499)]
    #[case(20, 100, 100)]
    #[case(20, 10, 10)]
    #[case(0, 100000, 100000)]
    #[case(0, 10000, 10000)]
    #[case(0, 5000, 5000)]
    #[case(0, 1000, 1000)]
    #[case(0, 500, 500)]
    #[case(0, 100, 100)]
    #[case(0, 10, 10)]
    fn test_slippage_threshold_buy_exact_in(
        #[case] slippage_tolerance_bps: u16,
        #[case] amount: u64,
        #[case] expected_slippage_threshold: u64,
    ) {
        let quote = Quote {
            quote_type: QuoteType::BuyExactIn,
            amount_in: 50,
            amount_out: amount,
            fee_amount: 0,
        };
        let result = slippage_threshold(quote, slippage_tolerance_bps).unwrap();
        assert_eq!(result.0, 50);
        assert_eq!(result.1, expected_slippage_threshold);
    }

    #[rstest]
    #[case(100, 100000, 101000)]
    #[case(100, 10000, 10100)]
    #[case(100, 5000, 5050)]
    #[case(100, 1000, 1010)]
    #[case(100, 500, 505)]
    #[case(100, 100, 101)]
    #[case(100, 10, 10)]
    #[case(20, 100000, 100200)]
    #[case(20, 10000, 10020)]
    #[case(20, 5000, 5010)]
    #[case(20, 1000, 1002)]
    #[case(20, 500, 501)]
    #[case(20, 100, 100)]
    #[case(20, 10, 10)]
    #[case(0, 100000, 100000)]
    #[case(0, 10000, 10000)]
    #[case(0, 5000, 5000)]
    #[case(0, 1000, 1000)]
    #[case(0, 500, 500)]
    #[case(0, 100, 100)]
    #[case(0, 10, 10)]
    fn test_slippage_threshold_buy_exact_out(
        #[case] slippage_tolerance_bps: u16,
        #[case] amount: u64,
        #[case] expected_slippage_threshold: u64,
    ) {
        let quote = Quote {
            quote_type: QuoteType::BuyExactOut,
            amount_in: amount,
            amount_out: 50,
            fee_amount: 0,
        };
        let result = slippage_threshold(quote, slippage_tolerance_bps).unwrap();
        assert_eq!(result.0, expected_slippage_threshold);
        assert_eq!(result.1, 50);
    }

    #[rstest]
    #[case(100, 100000, 99000)]
    #[case(100, 10000, 9900)]
    #[case(100, 5000, 4950)]
    #[case(100, 1000, 990)]
    #[case(100, 500, 495)]
    #[case(100, 100, 99)]
    #[case(100, 10, 10)]
    #[case(20, 100000, 99800)]
    #[case(20, 10000, 9980)]
    #[case(20, 5000, 4990)]
    #[case(20, 1000, 998)]
    #[case(20, 500, 499)]
    #[case(20, 100, 100)]
    #[case(20, 10, 10)]
    #[case(0, 100000, 100000)]
    #[case(0, 10000, 10000)]
    #[case(0, 5000, 5000)]
    #[case(0, 1000, 1000)]
    #[case(0, 500, 500)]
    #[case(0, 100, 100)]
    #[case(0, 10, 10)]
    fn test_slippage_threshold_sell_exact_in(
        #[case] slippage_tolerance_bps: u16,
        #[case] amount: u64,
        #[case] expected_slippage_threshold: u64,
    ) {
        let quote = Quote {
            quote_type: QuoteType::SellExactIn,
            amount_in: 50,
            amount_out: amount,
            fee_amount: 0,
        };
        let result = slippage_threshold(quote, slippage_tolerance_bps).unwrap();
        assert_eq!(result.0, expected_slippage_threshold);
        assert_eq!(result.1, 50);
    }

    #[rstest]
    #[case(100, 100000, 101000)]
    #[case(100, 10000, 10100)]
    #[case(100, 5000, 5050)]
    #[case(100, 1000, 1010)]
    #[case(100, 500, 505)]
    #[case(100, 100, 101)]
    #[case(100, 10, 10)]
    #[case(20, 100000, 100200)]
    #[case(20, 10000, 10020)]
    #[case(20, 5000, 5010)]
    #[case(20, 1000, 1002)]
    #[case(20, 500, 501)]
    #[case(20, 100, 100)]
    #[case(20, 10, 10)]
    #[case(0, 100000, 100000)]
    #[case(0, 10000, 10000)]
    #[case(0, 5000, 5000)]
    #[case(0, 1000, 1000)]
    #[case(0, 500, 500)]
    #[case(0, 100, 100)]
    #[case(0, 10, 10)]
    fn test_slippage_threshold_sell_exact_out(
        #[case] slippage_tolerance_bps: u16,
        #[case] amount: u64,
        #[case] expected_slippage_threshold: u64,
    ) {
        let quote = Quote {
            quote_type: QuoteType::SellExactOut,
            amount_in: amount,
            amount_out: 50,
            fee_amount: 0,
        };
        let result = slippage_threshold(quote, slippage_tolerance_bps).unwrap();
        assert_eq!(result.0, 50);
        assert_eq!(result.1, expected_slippage_threshold);
    }
}
