use super::error::{CoreError, AMOUNT_EXCEEDS_MAX_U64, ARITHMETIC_OVERFLOW, BPS_EXCEEDS_MAX_U16};

#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const BPS_DENOMINATOR: u16 = 10000;

pub(crate) fn protocol_fee_amount(amount: u64, protocol_fee_bps: u16) -> Result<u64, CoreError> {
    if protocol_fee_bps > BPS_DENOMINATOR {
        Err(BPS_EXCEEDS_MAX_U16)
    } else if protocol_fee_bps == 0 || amount == 0 {
        Ok(0)
    } else {
        let numerator = u128::from(amount)
            .checked_mul(protocol_fee_bps as u128)
            .ok_or(ARITHMETIC_OVERFLOW)?;
        let protocol_fee = numerator
            .div_ceil(BPS_DENOMINATOR as u128)
            .try_into()
            .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;
        Ok(protocol_fee)
    }
}

pub(crate) fn fee_from_pre_fee_amount(
    pre_fee_amount: u64,
    protocol_fee_bps: u16,
) -> Result<u64, CoreError> {
    if protocol_fee_bps > BPS_DENOMINATOR {
        Err(BPS_EXCEEDS_MAX_U16)
    } else if protocol_fee_bps == 0 || pre_fee_amount == 0 {
        Ok(0)
    } else {
        let numerator = <u128>::from(pre_fee_amount)
            .checked_mul(protocol_fee_bps.into())
            .ok_or(ARITHMETIC_OVERFLOW)?;
        let fee_amount: u64 = numerator
            .div_ceil(BPS_DENOMINATOR.into())
            .try_into()
            .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;
        Ok(fee_amount)
    }
}

pub(crate) fn fee_from_post_fee_amount(
    post_fee_amount: u64,
    protocol_fee_bps: u16,
) -> Result<u64, CoreError> {
    if protocol_fee_bps > BPS_DENOMINATOR {
        Err(BPS_EXCEEDS_MAX_U16)
    } else if protocol_fee_bps == 0 || post_fee_amount == 0 {
        Ok(0)
    } else if protocol_fee_bps == BPS_DENOMINATOR {
        Ok(u64::MAX)
    } else {
        let numerator = <u128>::from(post_fee_amount)
            .checked_mul(BPS_DENOMINATOR.into())
            .ok_or(ARITHMETIC_OVERFLOW)?;
        let denominator = <u128>::from(BPS_DENOMINATOR) - <u128>::from(protocol_fee_bps);
        let pre_fee_amount = numerator.div_ceil(denominator);
        let fee_amount: u64 = pre_fee_amount
            .checked_sub(post_fee_amount.into())
            .ok_or(ARITHMETIC_OVERFLOW)?
            .try_into()
            .map_err(|_| AMOUNT_EXCEEDS_MAX_U64)?;
        Ok(fee_amount)
    }
}

#[cfg(all(test, feature = "lib"))]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1000, 100, 10)]
    #[case(1000, 0, 0)]
    #[case(1000, 10000, 1000)]
    #[case(0, 100, 0)]
    #[case(9, 100, 1)]
    fn test_protocol_fee_amount(
        #[case] amount: u64,
        #[case] protocol_fee_bps: u16,
        #[case] expected_fee_amount: u64,
    ) {
        let fee_amount = protocol_fee_amount(amount, protocol_fee_bps).unwrap();
        assert_eq!(fee_amount, expected_fee_amount);
    }

    #[rstest]
    #[case(1000, 100, 10)]
    #[case(1000, 0, 0)]
    #[case(1000, 10000, 1000)]
    #[case(0, 100, 0)]
    #[case(9, 100, 1)]
    fn test_fee_from_pre_fee_amount(
        #[case] pre_fee_amount: u64,
        #[case] protocol_fee_bps: u16,
        #[case] expected_fee_amount: u64,
    ) {
        let fee_amount = fee_from_pre_fee_amount(pre_fee_amount, protocol_fee_bps).unwrap();
        assert_eq!(fee_amount, expected_fee_amount);
    }

    #[rstest]
    #[case(990, 100, 10)]
    #[case(1000, 0, 0)]
    #[case(1000, 10000, u64::MAX)]
    #[case(0, 100, 0)]
    #[case(9, 100, 1)]
    fn test_fee_from_post_fee_amount(
        #[case] post_fee_amount: u64,
        #[case] protocol_fee_bps: u16,
        #[case] expected_fee_amount: u64,
    ) {
        let fee_amount = fee_from_post_fee_amount(post_fee_amount, protocol_fee_bps).unwrap();
        assert_eq!(fee_amount, expected_fee_amount);
    }
}
