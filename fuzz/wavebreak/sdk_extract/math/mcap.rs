use super::curve::PriceCurveFacade;

use super::price::{base_to_quote_amount, token_price};

use super::token::total_supply;

use super::error::CoreError;

#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

/// Get the current fully diluted mcap of the base token expressed in the quote token.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn token_current_fdv(
    price_curve: PriceCurveFacade,
    quote_amount: u64,
    graduation_target: u64,
    creator_reward: u64,
    graduation_reward: u64,
    preminted_supply: u64,
    quote_protocol_fee_bps: u16,
    base_protocol_fee_bps: u16,
    base_allocation_bps: u16,
) -> Result<u64, CoreError> {
    let total_supply = total_supply(
        price_curve,
        graduation_target,
        creator_reward,
        graduation_reward,
        preminted_supply,
        quote_protocol_fee_bps,
        base_protocol_fee_bps,
        base_allocation_bps,
    )?;
    let token_price = token_price(price_curve, quote_amount, graduation_target)?;
    base_to_quote_amount(token_price, total_supply, false)
}

/// Get the estimated fully diluted mcap of the base token at token graduation expressed in the quote token.
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn token_graduation_fdv(
    price_curve: PriceCurveFacade,
    graduation_target: u64,
    creator_reward: u64,
    graduation_reward: u64,
    preminted_supply: u64,
    quote_protocol_fee_bps: u16,
    base_protocol_fee_bps: u16,
    base_allocation_bps: u16,
) -> Result<u64, CoreError> {
    let total_supply = total_supply(
        price_curve,
        graduation_target,
        creator_reward,
        graduation_reward,
        preminted_supply,
        quote_protocol_fee_bps,
        base_protocol_fee_bps,
        base_allocation_bps,
    )?;
    base_to_quote_amount(price_curve.end_price.into(), total_supply, false)
}

#[cfg(all(test, feature = "lib"))]
mod tests {
    use super::*;
    use crate::{curve::linear, fee::BPS_DENOMINATOR};
    use rstest::rstest;

    #[rstest]
    #[case(1000, 10240, 10, 0, 1213081959)]
    #[case(1000, 10240, 100, 0, 1213081959)]
    #[case(1000, 10240, 100, 1000000000, 2426163919)]
    #[case(10000, 10240, 10, 0, 3906552249)]
    #[case(10000, 10240, 100, 0, 3906552249)]
    #[case(10000, 10240, 100, 1000000000, 7813104499)]
    #[case(10000, 20480, 10, 0, 2226362409)]
    #[case(10000, 20480, 100, 0, 2226362409)]
    #[case(10000, 20480, 100, 1000000000, 4452724819)]
    fn test_token_current_fdv(
        #[case] quote_vault_amount: u64,
        #[case] graduation_target: u64,
        #[case] base_protocol_fee_bps: u16,
        #[case] preminted_supply: u64,
        #[case] expected_fdv: u64,
    ) {
        let price_curve = linear(1 << 64, 2 << 64);
        let fdv = token_current_fdv(
            price_curve,
            quote_vault_amount,
            graduation_target,
            0,
            0,
            preminted_supply,
            0,
            base_protocol_fee_bps,
            BPS_DENOMINATOR,
        )
        .unwrap();
        assert_eq!(fdv, expected_fdv);
    }

    #[rstest]
    #[case(50000000000, 100, 0, 152000000000)]
    #[case(50000000000, 1000, 0, 168000000000)]
    #[case(50000000000, 100, 1000000000, 156000000000)]
    #[case(20480, 100, 0, 4000000000)]
    fn test_token_graduation_fdv(
        #[case] graduation_target: u64,
        #[case] base_protocol_fee_bps: u16,
        #[case] preminted_supply: u64,
        #[case] expected_fdv: u64,
    ) {
        let price_curve = linear(1 << 64, 2 << 64);
        let fdv = token_graduation_fdv(
            price_curve,
            graduation_target,
            0,
            0,
            preminted_supply,
            0,
            base_protocol_fee_bps,
            BPS_DENOMINATOR,
        )
        .unwrap();
        assert_eq!(fdv, expected_fdv);
    }
}
