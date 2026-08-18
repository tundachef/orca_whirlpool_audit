use super::fee::protocol_fee_amount;

use super::error::AMOUNT_EXCEEDS_MAX_U64;

use super::token::{base_graduation_amount, circulating_supply, total_supply_round_up};

use super::curve::PriceCurveFacade;

use super::token::quote_graduation_amount;

use super::price::quote_to_base_amount;

use super::curve::AmountResult;

use super::fee::{fee_from_post_fee_amount, BPS_DENOMINATOR};

use super::{error::ARITHMETIC_OVERFLOW, fee::fee_from_pre_fee_amount};

use super::{curve::PriceCurve, error::CoreError};

#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub enum QuoteType {
    BuyExactIn,
    BuyExactOut,
    SellExactIn,
    SellExactOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub struct Quote {
    pub quote_type: QuoteType,
    pub amount_in: u64,
    pub amount_out: u64,
    /// Always in the quote token
    pub fee_amount: u64,
}

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn exact_in_buy_quote(
    price_curve_facade: PriceCurveFacade,
    requested_amount_in: u64, // quote
    swap_fee_bps: u16,
    current_quote_amount: u64,
    graduation_target: u64,
    max_buy_amount: u64,
) -> Result<Quote, CoreError> {
    let price_curve: PriceCurve = price_curve_facade.try_into()?;
    let requested_fee_amount = fee_from_pre_fee_amount(requested_amount_in, swap_fee_bps)?;
    let requested_amount_in_post_fee = requested_amount_in
        .checked_sub(requested_fee_amount)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let AmountResult {
        base_amount: amount_out,
        quote_amount: amount_in_post_fee,
    } = price_curve.amount(
        current_quote_amount,
        requested_amount_in_post_fee,
        true,
        false,
        graduation_target,
    )?;

    if amount_out > max_buy_amount {
        // Re-computing the whole new quote is not super efficient but is the safest

        let exact_out_quote = exact_out_buy_quote(
            price_curve_facade,
            max_buy_amount,
            swap_fee_bps,
            current_quote_amount,
            graduation_target,
            max_buy_amount,
        )?;
        if exact_out_quote.amount_in > requested_amount_in {
            unreachable!("Actual amount in can never exceed the requested amount in");
        }
        return Ok(Quote {
            quote_type: QuoteType::BuyExactIn,
            amount_in: exact_out_quote.amount_in,
            amount_out: exact_out_quote.amount_out,
            fee_amount: exact_out_quote.fee_amount,
        });
    }

    let is_full_fill = requested_amount_in_post_fee == amount_in_post_fee;
    if is_full_fill {
        return Ok(Quote {
            quote_type: QuoteType::BuyExactIn,
            amount_in: requested_amount_in,
            amount_out,
            fee_amount: requested_fee_amount,
        });
    }

    // If this is a partial fill, we need to recalculate the input and fee amounts
    let fee_amount = fee_from_post_fee_amount(amount_in_post_fee, swap_fee_bps)?;
    let amount_in = amount_in_post_fee
        .checked_add(fee_amount)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    Ok(Quote {
        quote_type: QuoteType::BuyExactIn,
        amount_in,
        amount_out,
        fee_amount,
    })
}

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn exact_out_buy_quote(
    price_curve_facade: PriceCurveFacade,
    requested_amount_out: u64, // base
    swap_fee_bps: u16,
    current_quote_amount: u64,
    graduation_target: u64,
    max_buy_amount: u64,
) -> Result<Quote, CoreError> {
    let price_curve: PriceCurve = price_curve_facade.try_into()?;
    let AmountResult {
        base_amount: amount_out,
        quote_amount: amount_in_post_fee,
    } = price_curve.amount(
        current_quote_amount,
        requested_amount_out.min(max_buy_amount),
        false,
        true,
        graduation_target,
    )?;

    let fee_amount = fee_from_post_fee_amount(amount_in_post_fee, swap_fee_bps)?;
    let amount_in = amount_in_post_fee
        .checked_add(fee_amount)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    Ok(Quote {
        quote_type: QuoteType::BuyExactOut,
        amount_in,
        amount_out,
        fee_amount,
    })
}

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn exact_in_sell_quote(
    price_curve_facade: PriceCurveFacade,
    requested_amount_in: u64, // base
    swap_fee_bps: u16,
    current_quote_amount: u64,
    graduation_target: u64,
    max_sell_amount: u64,
) -> Result<Quote, CoreError> {
    let price_curve: PriceCurve = price_curve_facade.try_into()?;
    let AmountResult {
        base_amount: amount_in,
        quote_amount: amount_out_pre_fee,
    } = price_curve.amount(
        current_quote_amount,
        requested_amount_in.min(max_sell_amount),
        true,
        true,
        graduation_target,
    )?;

    let fee_amount = fee_from_pre_fee_amount(amount_out_pre_fee, swap_fee_bps)?;
    let amount_out = amount_out_pre_fee
        .checked_sub(fee_amount)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    Ok(Quote {
        quote_type: QuoteType::SellExactIn,
        amount_in,
        amount_out,
        fee_amount,
    })
}

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn exact_out_sell_quote(
    price_curve_facade: PriceCurveFacade,
    requested_amount_out: u64, // quote
    swap_fee_bps: u16,
    current_quote_amount: u64,
    graduation_target: u64,
    max_sell_amount: u64,
) -> Result<Quote, CoreError> {
    let price_curve: PriceCurve = price_curve_facade.try_into()?;
    let requested_fee_amount = fee_from_post_fee_amount(requested_amount_out, swap_fee_bps)?;
    let requested_amount_out_pre_fee = requested_amount_out
        .checked_add(requested_fee_amount)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    let AmountResult {
        base_amount: amount_in,
        quote_amount: amount_out_pre_fee,
    } = price_curve.amount(
        current_quote_amount,
        requested_amount_out_pre_fee,
        false,
        false,
        graduation_target,
    )?;

    if amount_in > max_sell_amount {
        // Re-computing the whole new quote is not super efficient but is the safest
        let exact_in_quote = exact_in_sell_quote(
            price_curve_facade,
            max_sell_amount,
            swap_fee_bps,
            current_quote_amount,
            graduation_target,
            max_sell_amount,
        )?;

        if exact_in_quote.amount_out > requested_amount_out {
            unreachable!("Actual amount out can never exceed the requested amount out");
        }
        return Ok(Quote {
            quote_type: QuoteType::SellExactOut,
            amount_in: exact_in_quote.amount_in,
            amount_out: exact_in_quote.amount_out,
            fee_amount: exact_in_quote.fee_amount,
        });
    }

    let is_full_fill = requested_amount_out_pre_fee == amount_out_pre_fee;
    if is_full_fill {
        return Ok(Quote {
            quote_type: QuoteType::SellExactOut,
            amount_in: amount_in,
            amount_out: requested_amount_out,
            fee_amount: requested_fee_amount,
        });
    }

    // If this is a partial fill, we need to recalculate the input and fee amounts
    let fee_amount = fee_from_pre_fee_amount(amount_out_pre_fee, swap_fee_bps)?;
    let amount_out = amount_out_pre_fee
        .checked_sub(fee_amount)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    Ok(Quote {
        quote_type: QuoteType::SellExactOut,
        amount_in,
        amount_out,
        fee_amount,
    })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub struct GraduateQuote {
    pub quote_amount: u64,
    pub base_amount: u64,
}

// Protocol fees are distributed as follows:
// 1. creator gets a cut for creating the token (defined in bonding curve)
// 2. signer of this tx gets a small cut (defined in bonding curve)
// 3. rest of the fees go to the fee authority
// Rest of the quote tokens to be used for liquidity

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn graduate_quote(
    price_curve: PriceCurveFacade,
    split_bps: u16,
    quote_amount: u64,
    creator_reward: u64,
    graduation_reward: u64,
    quote_protocol_fee_bps: u16,
) -> Result<GraduateQuote, CoreError> {
    let total_quote_graduation_amount = quote_graduation_amount(
        quote_amount,
        quote_protocol_fee_bps,
        creator_reward,
        graduation_reward,
    )?;

    let quote_amount: u64 = u128::from(total_quote_graduation_amount)
        .checked_mul(split_bps as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .try_into()
        .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;

    let base_amount = quote_to_base_amount(price_curve.end_price.into(), quote_amount, false)?;

    Ok(GraduateQuote {
        quote_amount,
        base_amount,
    })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub struct CloseQuote {
    /// Total supply of the token
    pub total_supply: u64,
    /// The amount of base tokens to mint to as protocol fee
    pub base_protocol_fee: u64,
    /// Remaining base amount that was not dedicated to the bonding curve
    pub remaining_base_amount: u64,
}

/// This function is used to calculate the quote for the closing of the bonding curve
/// it only works after all graduation methods have been called.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn close_quote(
    price_curve: PriceCurveFacade,
    base_amount: u64,
    graduation_target: u64,
    creator_reward: u64,
    graduation_reward: u64,
    preminted_supply: u64,
    quote_protocol_fee_bps: u16,
    base_protocol_fee_bps: u16,
    base_allocation_bps: u16,
) -> Result<CloseQuote, CoreError> {
    // There are two graduation scenarios:
    // 1. Graduation target is reached
    // 2. Graduation time is reached and quote amount exceeds reserve target
    // In the second case, the price that the curve ended on is not equal to price_curve.end_price
    // We would like the ensure that the intended total supply is reached regardless of the price at graduation
    // To do this, we calculate the total supply as if the graduation target is reached
    // and then we calcuate how much more we need to mint (remaining_base_amount) to reach the intended total supply

    let base_amount_at_graduation =
        circulating_supply(price_curve, graduation_target, graduation_target)?;

    let base_graduation_amount = base_graduation_amount(
        price_curve.end_price,
        graduation_target,
        quote_protocol_fee_bps,
        creator_reward,
        graduation_reward,
    )?;

    let total_supply = total_supply_round_up(
        base_amount_at_graduation,
        preminted_supply,
        base_graduation_amount,
        base_protocol_fee_bps,
        base_allocation_bps,
    )?;

    let base_protocol_fee = protocol_fee_amount(total_supply, base_protocol_fee_bps)?;

    let remaining_base_amount = total_supply
        .checked_sub(base_protocol_fee)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_sub(base_amount)
        .ok_or(ARITHMETIC_OVERFLOW)?
        .checked_sub(preminted_supply)
        .ok_or(ARITHMETIC_OVERFLOW)?;

    Ok(CloseQuote {
        base_protocol_fee,
        total_supply,
        remaining_base_amount,
    })
}

#[cfg(all(test, feature = "lib"))]
mod tests {
    use super::*;
    use crate::curve::{flat, linear};
    use rstest::rstest;

    #[rstest]
    #[case(1 << 64, 990, 10)] // price = 1 B/A
    #[case(2 << 64, 247, 10)] // price = 4 B/A
    #[case(4 << 64, 61, 10)] // price = 16 B/A
    #[case((1 << 64) / 2, 3960, 10)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 15840, 10)] // price = 0.0625 B/A
    fn test_exact_in_buy(#[case] price: u128, #[case] amount_out: u64, #[case] fee_amount: u64) {
        let price_curve = flat(price);
        let quote = exact_in_buy_quote(price_curve, 1000, 100, 25000, 1000192, u64::MAX).unwrap();
        assert_eq!(quote.quote_type, QuoteType::BuyExactIn);
        assert_eq!(quote.amount_in, 1000);
        assert_eq!(quote.amount_out, amount_out);
        assert_eq!(quote.fee_amount, fee_amount);
    }

    #[rstest]
    #[case(1 << 64, 1011, 11)] // price = 1 B/A
    #[case(2 << 64, 4041, 41)] // price = 4 B/A
    #[case(4 << 64, 16172, 162)] // price = 16 B/A
    #[case((1 << 64) / 2, 253, 3)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 64, 1)] // price = 0.0625 B/A
    fn test_exact_out_buy(#[case] price: u128, #[case] amount_in: u64, #[case] fee_amount: u64) {
        let price_curve = flat(price);
        let quote = exact_out_buy_quote(price_curve, 1000, 100, 25000, 1000192, u64::MAX).unwrap();
        assert_eq!(quote.quote_type, QuoteType::BuyExactOut);
        assert_eq!(quote.amount_in, amount_in);
        assert_eq!(quote.amount_out, 1000);
        assert_eq!(quote.fee_amount, fee_amount);
    }

    #[rstest]
    #[case(1 << 64, 990, 10)] // price = 1 B/A
    #[case(2 << 64, 3960, 40)] // price = 4 B/A
    #[case(4 << 64, 15833, 160)] // price = 16 B/A
    #[case((1 << 64) / 2, 247, 3)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 61, 1)] // price = 0.0625 B/A
    fn test_exact_in_sell(#[case] price: u128, #[case] amount_out: u64, #[case] fee_amount: u64) {
        let price_curve = flat(price);
        let quote = exact_in_sell_quote(price_curve, 1000, 100, 25000, 1000192, u64::MAX).unwrap();
        assert_eq!(quote.quote_type, QuoteType::SellExactIn);
        assert_eq!(quote.amount_in, 1000);
        assert_eq!(quote.amount_out, amount_out);
        assert_eq!(quote.fee_amount, fee_amount);
    }

    #[rstest]
    #[case(1 << 64, 1011, 11)] // price = 1 B/A
    #[case(2 << 64, 253, 11)] // price = 4 B/A
    #[case(4 << 64, 64, 11)] // price = 16 B/A
    #[case((1 << 64) / 2, 4044, 11)] // price = 0.25 B/A
    #[case((1 << 64) / 4, 16176, 11)] // price = 0.0625 B/A
    fn test_exact_out_sell(#[case] price: u128, #[case] amount_in: u64, #[case] fee_amount: u64) {
        let price_curve = flat(price);
        let quote = exact_out_sell_quote(price_curve, 1000, 100, 25000, 1000192, u64::MAX).unwrap();
        assert_eq!(quote.quote_type, QuoteType::SellExactOut);
        assert_eq!(quote.amount_in, amount_in);
        assert_eq!(quote.amount_out, 1000);
        assert_eq!(quote.fee_amount, fee_amount);
    }

    #[rstest]
    #[case(QuoteType::BuyExactIn)]
    #[case(QuoteType::BuyExactOut)]
    #[case(QuoteType::SellExactIn)]
    #[case(QuoteType::SellExactOut)]
    fn test_max_amount_partial_fill(#[case] quote_type: QuoteType) {
        let price_curve = flat(1 << 64);
        let quote = match quote_type {
            QuoteType::BuyExactIn => {
                exact_in_buy_quote(price_curve, 1000, 0, 25000, 1000192, 500).unwrap()
            }
            QuoteType::BuyExactOut => {
                exact_out_buy_quote(price_curve, 1000, 0, 25000, 1000192, 500).unwrap()
            }
            QuoteType::SellExactIn => {
                exact_in_sell_quote(price_curve, 1000, 0, 25000, 1000192, 500).unwrap()
            }
            QuoteType::SellExactOut => {
                exact_out_sell_quote(price_curve, 1000, 0, 25000, 1000192, 500).unwrap()
            }
        };
        assert_eq!(quote.amount_in, 500);
        assert_eq!(quote.amount_out, 500);
        assert_eq!(quote.fee_amount, 0);
        assert_eq!(quote.quote_type, quote_type);
    }

    #[rstest]
    #[case(BPS_DENOMINATOR, 100000000000, 98999998900, 24749999725)]
    #[case(BPS_DENOMINATOR, 50000000000, 49499998900, 12374999725)]
    #[case(5000, 100000000000, 49499999450, 12374999862)]
    #[case(5000, 50000000000, 24749999450, 6187499862)]
    #[case(0, 100000000000, 0, 0)]
    #[case(0, 50000000000, 0, 0)]
    fn test_graduate_quote(
        #[case] split_bps: u16,
        #[case] quote_amount: u64,
        #[case] expected_quote_amount: u64,
        #[case] expected_base_amount: u64,
    ) {
        let price_curve = linear(1 << 64, 2 << 64);
        let quote = graduate_quote(price_curve, split_bps, quote_amount, 1000, 100, 100).unwrap();
        assert_eq!(quote.quote_amount, expected_quote_amount);
        assert_eq!(quote.base_amount, expected_base_amount);
    }

    #[rstest]
    #[case(1 << 64, 198999999998900, 0, 201011000000000, 2010110000000, 890001100)]
    #[case(1 << 64, 100000000000000, 0, 201011000000000, 2010110000000, 99000890000000)]
    #[case(1 << 64, 100000000000000, 100, 201011000000000, 2010110000000, 99000889999900)]
    #[case(1 << 64, 100000000000000, 1000000000, 201012000000000, 2010120000000, 99000880000000)]
    #[case(2 << 64, 49749999999725, 0, 50253000000000, 502530000000, 470000275)]
    #[case(2 << 64, 40000000000000, 0, 50253000000000, 502530000000, 9750470000000)]
    #[case(2 << 64, 40000000000000, 100, 50253000000000, 502530000000, 9750469999900)]
    #[case(2 << 64, 40000000000000, 1000000000, 50254000000000, 502540000000, 9750460000000)]
    #[case((1 << 64) / 2, 795999999995600, 0, 804041000000000, 8040410000000, 590004400)]
    #[case((1 << 64) / 2, 700000000000000, 0, 804041000000000, 8040410000000, 96000590000000)]
    #[case((1 << 64) / 2, 700000000000000, 100, 804041000000000, 8040410000000, 96000589999900)]
    #[case((1 << 64) / 2, 700000000000000, 1000000000, 804042000000000, 8040420000000, 96000580000000)]
    fn test_close_quote(
        #[case] price: u128,
        #[case] base_amount: u64,
        #[case] preminted_supply: u64,
        #[case] expected_total_supply: u64,
        #[case] expected_base_protocol_fee: u64,
        #[case] expected_remaining_base_amount: u64,
    ) {
        let price_curve = flat(price);
        let quote = close_quote(
            price_curve,
            base_amount,
            100000000000000,
            1000,
            100,
            preminted_supply,
            100,
            100,
            BPS_DENOMINATOR,
        )
        .unwrap();
        assert_eq!(quote.total_supply, expected_total_supply);
        assert_eq!(quote.base_protocol_fee, expected_base_protocol_fee);
        assert_eq!(quote.remaining_base_amount, expected_remaining_base_amount);
    }
}
