#[cfg(feature = "floats")]
use libm::pow;

#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

use super::fee::protocol_fee_amount;

use super::curve::PriceCurveFacade;

use super::price::quote_to_base_amount;

use super::fee::BPS_DENOMINATOR;

use super::error::AMOUNT_EXCEEDS_MAX_U64;

use super::error::ARITHMETIC_OVERFLOW;

use super::curve::PriceCurve;
use super::error::CoreError;

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const DEFAULT_TOKEN_DECIMALS: u8 = 9;

const TOTAL_SUPPLY_MULTIPLE: u128 = 10u128.pow(DEFAULT_TOKEN_DECIMALS as u32);

/// Get the current circulating supply of the token.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn circulating_supply(
    price_curve: PriceCurveFacade,
    quote_amount: u64,
    graduation_target: u64,
) -> Result<u64, CoreError> {
    let price_curve: PriceCurve = price_curve.try_into()?;
    let result = price_curve.amount(0, quote_amount, true, false, graduation_target)?;
    Ok(result.base_amount)
}

/// Calculates the amount of quote tokens that can be used for the graudation methods.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn quote_graduation_amount(
    quote_amount: u64,
    quote_protocol_fee_bps: u16,
    creator_reward: u64,
    graduation_reward: u64,
) -> Result<u64, CoreError> {
    let protocol_fee = protocol_fee_amount(quote_amount, quote_protocol_fee_bps)?;

    let quote_graduation_amount = quote_amount
        .checked_sub(creator_reward)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_sub(graduation_reward)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_sub(protocol_fee)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    Ok(quote_graduation_amount)
}

/// Calculates the amount of base tokens that can be used for the graudation methods.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn base_graduation_amount(
    final_price: u128,
    quote_amount: u64,
    quote_protocol_fee_bps: u16,
    creator_reward: u64,
    graduation_reward: u64,
) -> Result<u64, CoreError> {
    let quote_graduation_amount = quote_graduation_amount(
        quote_amount,
        quote_protocol_fee_bps,
        creator_reward,
        graduation_reward,
    )?;
    let base_graduation_amount =
        quote_to_base_amount(final_price.into(), quote_graduation_amount, false)?;
    Ok(base_graduation_amount)
}

/// Get the estimated total supply of the token (at graduation).
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn total_supply(
    price_curve: PriceCurveFacade,
    graduation_target: u64,
    creator_reward: u64,
    graduation_reward: u64,
    preminted_supply: u64,
    quote_protocol_fee_bps: u16,
    base_protocol_fee_bps: u16,
    base_allocation_bps: u16,
) -> Result<u64, CoreError> {
    let base_amount = circulating_supply(price_curve, graduation_target, graduation_target)?;
    let base_graduation_amount = base_graduation_amount(
        price_curve.end_price,
        graduation_target,
        quote_protocol_fee_bps,
        creator_reward,
        graduation_reward,
    )?;

    let total_supply = total_supply_round_up(
        base_amount,
        preminted_supply,
        base_graduation_amount,
        base_protocol_fee_bps,
        base_allocation_bps,
    )?;

    Ok(total_supply)
}

pub(crate) fn total_supply_round_up(
    base_amount: u64,
    preminted_supply: u64,
    base_graduation_amount: u64,
    base_protocol_fee_bps: u16,
    base_allocation_bps: u16,
) -> Result<u64, CoreError> {
    let bonding_curve_base_amount: u64 = u128::from(base_amount)
        .checked_add(u128::from(base_graduation_amount))
        .ok_or(ARITHMETIC_OVERFLOW)?
        .try_into()
        .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;

    let total_supply_pre_protocol_fee: u64 = u128::from(bonding_curve_base_amount)
        .checked_mul(BPS_DENOMINATOR as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_div(base_allocation_bps as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_add(u128::from(preminted_supply))
        .ok_or(ARITHMETIC_OVERFLOW)?
        .try_into()
        .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;

    let raw_total_supply: u64 = u128::from(total_supply_pre_protocol_fee)
        .checked_mul(BPS_DENOMINATOR as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_div(BPS_DENOMINATOR as u128 - base_protocol_fee_bps as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .try_into()
        .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;

    let quotient = u128::from(raw_total_supply)
        .checked_div(TOTAL_SUPPLY_MULTIPLE)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let remainder = u128::from(raw_total_supply)
        .checked_rem(TOTAL_SUPPLY_MULTIPLE)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let multiple = if remainder > 0 {
        quotient + 1
    } else {
        quotient
    };

    let total_supply: u64 = multiple
        .checked_mul(TOTAL_SUPPLY_MULTIPLE)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .try_into()
        .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;

    Ok(total_supply)
}

#[cfg(feature = "floats")]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn amount_to_ui_amount(amount: u64, decimals: u8) -> f64 {
    let power = pow(10f64, decimals as f64);
    amount as f64 / power
}

#[cfg(feature = "floats")]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn ui_amount_to_amount(amount: f64, decimals: u8) -> u64 {
    let power = pow(10f64, decimals as f64);
    (amount * power) as u64
}

#[cfg(all(test, feature = "lib"))]
mod tests {
    use super::*;
    use crate::curve::flat;
    use rstest::rstest;

    #[rstest]
    #[case(1 << 64, 0, 1000192, 0)]
    #[case(1 << 64, 500096, 1000192, 500096)]
    #[case(1 << 64, 1000192, 1000192, 1000192)]
    #[case(2 << 64, 0, 1000192, 0)]
    #[case(2 << 64, 500096, 1000192, 125000)]
    #[case(2 << 64, 1000192, 1000192, 250000)]
    #[case(4 << 64, 0, 1000192, 0)]
    #[case(4 << 64, 500096, 1000192, 31247)]
    #[case(4 << 64, 1000192, 1000192, 62495)]
    #[case((1 << 64) / 2, 0, 1000192, 0)]
    #[case((1 << 64) / 2, 500096, 1000192, 2000384)]
    #[case((1 << 64) / 2, 1000192, 1000192, 4000768)]
    #[case((1 << 64) / 4, 0, 1000192, 0)]
    #[case((1 << 64) / 4, 500096, 1000192, 8001536)]
    #[case((1 << 64) / 4, 1000192, 1000192, 16003072)]
    fn test_circulating_supply(
        #[case] price: u128,
        #[case] quote_amount: u64,
        #[case] graduation_target: u64,
        #[case] expected_supply: u64,
    ) {
        let price_curve = flat(price);
        let total_supply =
            circulating_supply(price_curve, quote_amount, graduation_target).unwrap();
        assert_eq!(total_supply, expected_supply);
    }

    #[rstest]
    #[case(1 << 64, 10240, 0, 1000000000)] // price = 1 B/A
    #[case(1 << 64, 10240, 100000000000, 102000000000)] // price = 1 B/A
    #[case(2 << 64, 10240, 0, 1000000000)] // price = 4 B/A
    #[case(4 << 64, 10240, 0, 1000000000)] // price = 16 B/A
    #[case((1 << 64) / 2, 10240, 0, 1000000000)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 10240, 0, 1000000000)] // price = 0.0625 B/A
    #[case(1 << 64, 50000000000, 0, 113000000000)] // price = 1 B/A
    #[case(1 << 64, 50000000000, 100000000000, 214000000000)] // price = 1 B/A
    #[case(2 << 64, 50000000000, 0, 29000000000)] // price = 4 B/A
    #[case(4 << 64, 50000000000, 0, 8000000000)] // price = 16 B/A
    #[case((1 << 64) / 2, 50000000000, 0, 449000000000)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 50000000000, 0, 1796000000000)] // price = 0.0625 B/A
    fn test_total_supply(
        #[case] price: u128,
        #[case] graduation_target: u64,
        #[case] preminted_supply: u64,
        #[case] expected_supply: u64,
    ) {
        let price_curve = flat(price);
        let total_supply = total_supply(
            price_curve,
            graduation_target,
            0,
            0,
            preminted_supply,
            0,
            100,
            9000,
        )
        .unwrap();
        assert_eq!(total_supply, expected_supply);
    }

    #[rstest]
    #[case(1000000000, 9, 1.0)]
    #[case(1000000000, 6, 1000.0)]
    #[case(1000000000, 3, 1000000.0)]
    fn test_amount_to_ui_amount(
        #[case] amount: u64,
        #[case] decimals: u8,
        #[case] expected_ui_amount: f64,
    ) {
        let ui_amount = amount_to_ui_amount(amount, decimals);
        assert_eq!(ui_amount, expected_ui_amount);
    }

    #[rstest]
    #[case(1.0, 9, 1000000000)]
    #[case(1000.0, 6, 1000000000)]
    #[case(1000000.0, 3, 1000000000)]
    fn test_ui_amount_to_amount(
        #[case] ui_amount: f64,
        #[case] decimals: u8,
        #[case] expected_amount: u64,
    ) {
        let amount = ui_amount_to_amount(ui_amount, decimals);
        assert_eq!(amount, expected_amount);
    }
}
