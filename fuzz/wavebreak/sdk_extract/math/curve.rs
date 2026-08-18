use super::error::AMOUNT_EXCEEDS_MAX_U128;
use super::error::AMOUNT_EXCEEDS_MAX_U64;

use super::error::INVALID_PRICE_CURVE;
use super::fee::BPS_DENOMINATOR;

use super::U128;

use super::price::{base_to_quote_amount, quote_to_base_amount};

use super::error::{CoreError, ARITHMETIC_OVERFLOW, BEYOND_GRADUATION_TARGET};

#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

// The number of bins in the curve
#[cfg_attr(feature = "wasm", wasm_expose)]
pub const NUM_BINS: usize = 64;

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const MIN_GRADUATION_TARGET: u64 = 10000;

const ONE: u128 = BPS_DENOMINATOR as u128;
const T_PER_BIN: u128 = ONE / NUM_BINS as u128;
const T_REMAINDER: u128 = ONE % NUM_BINS as u128;

// price: ~1e-13 B per A
// tick: -300000
#[cfg_attr(feature = "wasm", wasm_expose)]
pub const MIN_SQRT_PRICE: u128 = 5647135299341;

// price: ~2e13 B per A
// tick: +300000
#[cfg_attr(feature = "wasm", wasm_expose)]
pub const MAX_SQRT_PRICE: u128 = 60257519765924248467716150;

// The bonding curve a mechanism to accumulate quote tokens towards
// the graduation target. In return users that deposit quote tokens
// will receive base tokens. The curve itself is defined as a series of bins
// each containing a fixed amount of quote tokens (to be collected). Each bin
// also has a corresponding price and together with the amount of quote tokens,
// the amount of base tokens (to be minted) in the bin can be calculated.

// This implementation is designed in such a way that the quote amount
// in a bin is always fixed meaning that in order for the curve to move
// to the next bin, the full amount of quote tokens must be collected.
// This implementation handles rounding issues by either 'minting less' (on buy)
// or 'collecting more' (on sell) of base tokens. This means that the amount of
// base tokens in a bin is unfixed. Any base tokens that are missing (due to rounding)
// are minted at graduation to make sure the intended target supply is reached.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub struct PriceCurveFacade {
    pub start_price: u128,
    pub end_price: u128,
    pub control_points: [u16; 4],
}

impl PriceCurveFacade {
    pub fn is_valid(&self) -> bool {
        [
            self.start_price >= MIN_SQRT_PRICE,
            self.end_price >= self.start_price,
            self.end_price <= MAX_SQRT_PRICE,
            self.control_points[0] <= BPS_DENOMINATOR,
            self.control_points[1] <= BPS_DENOMINATOR,
            self.control_points[2] <= BPS_DENOMINATOR,
            self.control_points[3] <= BPS_DENOMINATOR,
        ]
        .iter()
        .all(|&x| x)
    }
}

// Creates a curve where the price is constant
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn flat(sqrt_price: u128) -> PriceCurveFacade {
    PriceCurveFacade {
        start_price: sqrt_price,
        end_price: sqrt_price,
        control_points: [3333, 5000, 6667, 5000],
    }
}

/// Creates a curve where the **sqrt_price** moves linearly from start_price to end_price
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn linear(start_price: u128, end_price: u128) -> PriceCurveFacade {
    PriceCurveFacade {
        start_price,
        end_price,
        control_points: [3333, 3333, 6667, 6667],
    }
}

/// Creates a curve where the **sqrt_price** moves exponentially from start_price to end_price
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn exponential(start_price: u128, end_price: u128, rate: u16) -> PriceCurveFacade {
    PriceCurveFacade {
        start_price,
        end_price,
        control_points: [rate, 0, BPS_DENOMINATOR, BPS_DENOMINATOR - rate],
    }
}

/// Creates a curve where the **sqrt_price** moves sigmoidally from start_price to end_price
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn sigmoid(start_price: u128, end_price: u128, rate: u16) -> PriceCurveFacade {
    PriceCurveFacade {
        start_price,
        end_price,
        control_points: [rate, 0, BPS_DENOMINATOR - rate, BPS_DENOMINATOR],
    }
}

impl TryFrom<PriceCurveFacade> for PriceCurve {
    type Error = CoreError;

    fn try_from(value: PriceCurveFacade) -> Result<Self, Self::Error> {
        if !value.is_valid() {
            return Err(INVALID_PRICE_CURVE);
        }

        let bins = bezier_curve(value.control_points)?;
        Ok(Self {
            start_price: value.start_price,
            end_price: value.end_price,
            control_points: value.control_points,
            bins,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub struct Bin {
    pub x: u16,
    pub y: u16,
}

#[cfg_attr(feature = "wasm", wasm_expose)]
pub struct Bins(#[allow(dead_code)] pub Vec<Bin>);

#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn curve(curve: PriceCurveFacade) -> Result<Bins, CoreError> {
    if !curve.is_valid() {
        return Err(INVALID_PRICE_CURVE);
    }
    bezier_curve(curve.control_points).map(|x| Bins(x.to_vec()))
}

// Since first point is always 0,0 and last point is always 1,1 we can
// simplify the formulas a little bit
// x(t) = 3 * (1-t)^2 * t * x1 + 3 * (1-t) * t^2 * x2 + t^3;
// y(t) = 3 * (1-t)^2 * t * y1 + 3 * (1-t) * t^2 * y2 + t^3;
// as long as the control points are between 0 and BPS_DENOMINATOR this function
// will not overflow.
pub(crate) fn bezier_curve(control_points: [u16; 4]) -> Result<[Bin; NUM_BINS + 1], CoreError> {
    let mut bins = [Bin { x: 0, y: 0 }; NUM_BINS + 1];
    for i in 0..=NUM_BINS {
        let t = i as u128 * T_PER_BIN + (i as u128 * T_REMAINDER) / NUM_BINS as u128;
        let t1 = 3 * ((ONE - t) * (ONE - t) * t);
        let t2 = 3 * ((ONE - t) * t * t);
        let t3 = t * t * t;

        let x1 = control_points[0] as u128;
        let y1 = control_points[1] as u128;
        let x2 = control_points[2] as u128;
        let y2 = control_points[3] as u128;

        let x = (t1 * x1 + t2 * x2 + t3 * ONE) / (ONE * ONE * ONE);
        let y = (t1 * y1 + t2 * y2 + t3 * ONE) / (ONE * ONE * ONE);

        bins[i] = Bin {
            x: x as u16,
            y: y as u16,
        };
    }

    Ok(bins)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AmountResult {
    pub base_amount: u64,
    pub quote_amount: u64,
}

impl AmountResult {
    pub fn new(base_amount: u64, quote_amount: u64) -> Self {
        Self {
            base_amount,
            quote_amount,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PriceCurve {
    pub start_price: u128,
    pub end_price: u128,
    pub control_points: [u16; 4],
    pub bins: [Bin; NUM_BINS + 1],
}

impl PriceCurve {
    pub(crate) fn sqrt_price(
        &self,
        quote_amount: u64,
        graduation_target: u64,
    ) -> Result<U128, CoreError> {
        let bin_index = self.bin_at_quote_amount(quote_amount, graduation_target)?;
        self.bin_price(bin_index)
    }

    pub(crate) fn amount(
        &self,
        current_quote_amount: u64,
        swap_amount: u64,
        amount_is_input: bool,
        amount_is_base: bool,
        graduation_target: u64,
    ) -> Result<AmountResult, CoreError> {
        if swap_amount == 0 {
            return Ok(AmountResult::new(0, 0));
        }

        let mut bin_index = self.bin_at_quote_amount(current_quote_amount, graduation_target)?;
        let mut spent_amount: u64 = 0;
        let mut received_amount: u64 = 0;
        let mut amount_remaining = swap_amount;
        let mut current_quote_amount = current_quote_amount;

        let is_l_to_r = match (amount_is_input, amount_is_base) {
            (true, false) => true,   // Buy base, ExactIn
            (false, true) => true,   // Buy base, ExactOut
            (true, true) => false,   // Sell base, ExactIn
            (false, false) => false, // Sell base, ExactOut
        };

        let mut next_quote_amount = if is_l_to_r {
            self.quote_amount_at_bin(bin_index + 1, graduation_target)?
        } else {
            self.quote_amount_at_bin(bin_index, graduation_target)?
        };

        while amount_remaining > 0 {
            let bin_price: u128 = self.bin_price(bin_index)?.into();

            let quote_amount_in_bin = next_quote_amount.abs_diff(current_quote_amount);

            let base_amount_in_bin =
                quote_to_base_amount(bin_price.into(), quote_amount_in_bin, !is_l_to_r)?;

            let delta_amount = if amount_is_base {
                base_amount_in_bin.min(amount_remaining)
            } else {
                quote_amount_in_bin.min(amount_remaining)
            };

            let other_delta_amount = if amount_is_base {
                if delta_amount == base_amount_in_bin {
                    quote_amount_in_bin
                } else {
                    base_to_quote_amount(bin_price.into(), delta_amount, !amount_is_input)?
                }
            } else {
                if delta_amount == quote_amount_in_bin {
                    base_amount_in_bin
                } else {
                    quote_to_base_amount(bin_price.into(), delta_amount, !amount_is_input)?
                }
            };

            amount_remaining = amount_remaining
                .checked_sub(delta_amount)
                .ok_or(ARITHMETIC_OVERFLOW)?;
            spent_amount = spent_amount
                .checked_add(delta_amount)
                .ok_or(ARITHMETIC_OVERFLOW)?;
            received_amount = received_amount
                .checked_add(other_delta_amount)
                .ok_or(ARITHMETIC_OVERFLOW)?;

            current_quote_amount = next_quote_amount;

            // If we are at the end (if buying) there is no more left
            // to buy so we partial fill
            if is_l_to_r && bin_index == NUM_BINS - 1 {
                break;
            }

            // If we are at the start (if selling) there is no more left
            // to sell so we partial fill
            if !is_l_to_r && bin_index == 0 {
                break;
            }

            bin_index = if is_l_to_r {
                bin_index + 1
            } else {
                bin_index - 1
            };

            let bin_quote_amount = self.bin_quote_amount(bin_index, graduation_target)?;
            if is_l_to_r {
                next_quote_amount = next_quote_amount
                    .checked_add(bin_quote_amount)
                    .ok_or(ARITHMETIC_OVERFLOW)?;
            } else {
                next_quote_amount = next_quote_amount
                    .checked_sub(bin_quote_amount)
                    .ok_or(ARITHMETIC_OVERFLOW)?;
            }
        }

        if amount_is_base {
            Ok(AmountResult {
                base_amount: spent_amount,
                quote_amount: received_amount,
            })
        } else {
            Ok(AmountResult {
                base_amount: received_amount,
                quote_amount: spent_amount,
            })
        }
    }
}

impl PriceCurve {
    fn bin_price(&self, bin_index: usize) -> Result<U128, CoreError> {
        if bin_index >= NUM_BINS {
            return Err(BEYOND_GRADUATION_TARGET);
        }
        // Take the average of the sqrt_price at the bin boundaries
        let lower_price = self.transform_y(self.bins[bin_index].y)?;
        let upper_price = self.transform_y(self.bins[bin_index + 1].y)?;
        let sqrt_price = lower_price
            .checked_add(upper_price)
            .ok_or(ARITHMETIC_OVERFLOW)?
            .checked_shr(1)
            .ok_or(ARITHMETIC_OVERFLOW)?;
        sqrt_price.try_into().map_err(|_| AMOUNT_EXCEEDS_MAX_U128)
    }

    fn bin_quote_amount(&self, bin_index: usize, graduation_target: u64) -> Result<u64, CoreError> {
        if bin_index >= NUM_BINS {
            return Err(BEYOND_GRADUATION_TARGET);
        }
        let lower_amount = self.transform_x(self.bins[bin_index].x, graduation_target)?;
        let upper_amount = self.transform_x(self.bins[bin_index + 1].x, graduation_target)?;
        let quote_amount = lower_amount.abs_diff(upper_amount);
        Ok(quote_amount)
    }

    fn quote_amount_at_bin(
        &self,
        bin_index: usize,
        graduation_target: u64,
    ) -> Result<u64, CoreError> {
        if bin_index > NUM_BINS {
            return Err(BEYOND_GRADUATION_TARGET);
        }
        let quote_amount = self.transform_x(self.bins[bin_index].x, graduation_target)?;
        Ok(quote_amount)
    }

    fn bin_at_quote_amount(
        &self,
        quote_amount: u64,
        graduation_target: u64,
    ) -> Result<usize, CoreError> {
        if quote_amount > graduation_target {
            return Err(BEYOND_GRADUATION_TARGET);
        }

        if quote_amount == graduation_target {
            return Ok(NUM_BINS);
        }

        let mut found_quote_amount = 0;

        for index in 0..NUM_BINS {
            let bin_quote_amount = self.bin_quote_amount(index, graduation_target)?;
            found_quote_amount += bin_quote_amount;
            if found_quote_amount > quote_amount {
                return Ok(index);
            }
        }

        Ok(NUM_BINS)
    }
}

impl PriceCurve {
    fn transform_x(&self, x: u16, graduation_target: u64) -> Result<u64, CoreError> {
        let product = u128::from(graduation_target)
            .checked_mul(u128::from(x))
            .ok_or(ARITHMETIC_OVERFLOW)?;
        let quotient = product
            .checked_div(u128::from(BPS_DENOMINATOR))
            .ok_or(ARITHMETIC_OVERFLOW)?;
        let remainder = product
            .checked_sub(
                quotient
                    .checked_mul(u128::from(BPS_DENOMINATOR))
                    .ok_or(ARITHMETIC_OVERFLOW)?,
            )
            .ok_or(ARITHMETIC_OVERFLOW)?;
        let result = if remainder > 5000 {
            quotient + 1
        } else {
            quotient
        };
        result.try_into().map_err(|_| AMOUNT_EXCEEDS_MAX_U64)
    }

    fn transform_y(&self, y: u16) -> Result<u128, CoreError> {
        let price_diff = self
            .end_price
            .checked_sub(self.start_price)
            .ok_or(ARITHMETIC_OVERFLOW)?;

        let delta = price_diff
            .checked_mul(y as u128)
            .ok_or(ARITHMETIC_OVERFLOW)?
            .checked_div(BPS_DENOMINATOR as u128)
            .ok_or(ARITHMETIC_OVERFLOW)?;

        self.start_price
            .checked_add(delta)
            .ok_or(ARITHMETIC_OVERFLOW)?
            .try_into()
            .map_err(|_| AMOUNT_EXCEEDS_MAX_U128)
    }
}

#[cfg(all(test, feature = "lib"))]
mod tests {
    use super::*;
    use crate::price::{price_to_sqrt_price, sqrt_price_to_price};
    use rstest::rstest;

    #[rstest]
    #[case(0, 1000192, Ok(18589706340280800641))]
    #[case(1, 1000192, Ok(18589706340280800641))]
    #[case(16000, 1000192, Ok(18876553210626984168))]
    #[case(500096, 1000192, Ok(27814000714339261926))]
    #[case(1000191, 1000192, Ok(36748681206440483251))]
    #[case(1000192, 1000192, Err(BEYOND_GRADUATION_TARGET))]
    #[case(1000193, 1000192, Err(BEYOND_GRADUATION_TARGET))]
    fn test_price(
        #[case] quote_amount: u64,
        #[case] graduation_target: u64,
        #[case] expected_price: Result<u128, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let result = curve
            .sqrt_price(quote_amount, graduation_target)
            .map(|x| x.into());
        assert_eq!(result, expected_price);
    }

    // Buying with quote amount specified
    #[rstest]
    #[case(10000, 0, Ok(AmountResult::new(0, 0)))]
    #[case(10000, 1000, Ok(AmountResult::new(984, 1000)))]
    #[case(10000, 2000, Ok(AmountResult::new(1969, 2000)))]
    #[case(0, 100000000000, Ok(AmountResult::new(49998221241, 100000000000)))]
    #[case(99999999995, 1000, Ok(AmountResult::new(1, 5)))]
    #[case(99999999999, 1000, Ok(AmountResult::new(0, 1)))]
    fn test_amount_input_quote(
        #[case] current_quote_amount: u64,
        #[case] swap_amount: u64,
        #[case] expected_amount: Result<AmountResult, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let result = curve.amount(current_quote_amount, swap_amount, true, false, 100000000000);
        assert_eq!(result, expected_amount);
    }

    // Buying with base amount specified
    #[rstest]
    #[case(10000, 0, Ok(AmountResult::new(0, 0)))]
    #[case(10000, 1000, Ok(AmountResult::new(1000, 1016)))]
    #[case(10000, 2000, Ok(AmountResult::new(2000, 2032)))]
    #[case(0, 100000000000, Ok(AmountResult::new(49998221241, 100000000000)))]
    #[case(99999999995, 1000, Ok(AmountResult::new(1, 5)))]
    #[case(99999999999, 1000, Ok(AmountResult::new(0, 1)))]
    fn test_amount_output_base(
        #[case] current_quote_amount: u64,
        #[case] swap_amount: u64,
        #[case] expected_amount: Result<AmountResult, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let result = curve.amount(current_quote_amount, swap_amount, false, true, 100000000000);
        assert_eq!(result, expected_amount);
    }

    // Selling with quote amount specified
    #[rstest]
    #[case(10000, 0, Ok(AmountResult::new(0, 0)))]
    #[case(10000, 1000, Ok(AmountResult::new(985, 1000)))]
    #[case(10000, 2000, Ok(AmountResult::new(1970, 2000)))]
    #[case(
        99999999999,
        100000000000,
        Ok(AmountResult::new(49998221305, 99999999999))
    )]
    #[case(5, 1000, Ok(AmountResult::new(5, 5)))]
    #[case(1, 1000, Ok(AmountResult::new(1, 1)))]
    fn test_amount_output_quote(
        #[case] current_quote_amount: u64,
        #[case] swap_amount: u64,
        #[case] expected_amount: Result<AmountResult, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let result = curve.amount(
            current_quote_amount,
            swap_amount,
            false,
            false,
            100000000000,
        );
        assert_eq!(result, expected_amount);
    }

    // Selling with base amount specified
    #[rstest]
    #[case(10000, 0, Ok(AmountResult::new(0, 0)))]
    #[case(10000, 1000, Ok(AmountResult::new(1000, 1015)))]
    #[case(10000, 2000, Ok(AmountResult::new(2000, 2031)))]
    #[case(
        99999999999,
        100000000000,
        Ok(AmountResult::new(49998221305, 99999999999))
    )]
    #[case(5, 1000, Ok(AmountResult::new(5, 5)))]
    #[case(1, 1000, Ok(AmountResult::new(1, 1)))]
    fn test_amount_input_base(
        #[case] current_quote_amount: u64,
        #[case] swap_amount: u64,
        #[case] expected_amount: Result<AmountResult, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let result = curve.amount(current_quote_amount, swap_amount, true, true, 100000000000);
        assert_eq!(result, expected_amount);
    }

    #[rstest]
    #[case(0, Ok(10064.306153126028))]
    #[case(1, Ok(10193.954972048326))]
    #[case(32, Ok(14649.172196802781))]
    #[case(63, Ok(19908.137256787628))]
    #[case(64, Err(BEYOND_GRADUATION_TARGET))]
    #[case(65, Err(BEYOND_GRADUATION_TARGET))]
    fn test_bin_price(
        #[case] bin_index: usize,
        #[case] expected_bin_price: Result<f64, CoreError>,
    ) {
        let curve: PriceCurve = linear(
            price_to_sqrt_price(10000.0, 9, 9).into(),
            price_to_sqrt_price(20000.0, 9, 9).into(),
        )
        .try_into()
        .unwrap();
        let bin_price = curve
            .bin_price(bin_index)
            .map(|x| x.into())
            .map(|x| sqrt_price_to_price(x, 9, 9));
        assert_eq!(bin_price, expected_bin_price);
    }

    #[rstest]
    #[case(0, 1000192, Ok(15503))]
    #[case(1, 1000192, Ok(15603))]
    #[case(32, 1000192, Ok(15603))]
    #[case(63, 1000192, Ok(15703))]
    #[case(64, 1000192, Err(BEYOND_GRADUATION_TARGET))]
    #[case(65, 1000192, Err(BEYOND_GRADUATION_TARGET))]
    fn test_bin_quote_amount(
        #[case] bin_index: usize,
        #[case] graduation_target: u64,
        #[case] expected_bin_amounts: Result<u64, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let bin_quote_amount = curve.bin_quote_amount(bin_index, graduation_target);
        assert_eq!(bin_quote_amount, expected_bin_amounts);
    }

    #[rstest]
    #[case(0, 1000192, Ok(0))]
    #[case(1, 1000192, Ok(0))]
    #[case(15503, 1000192, Ok(1))]
    #[case(500096, 1000192, Ok(32))]
    #[case(1000191, 1000192, Ok(63))]
    #[case(1000192, 1000192, Ok(64))]
    #[case(1000193, 1000192, Err(BEYOND_GRADUATION_TARGET))]
    fn test_bin_at_quote_amount(
        #[case] quote_amount: u64,
        #[case] graduation_target: u64,
        #[case] expected_bin_index: Result<usize, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let bin_index = curve.bin_at_quote_amount(quote_amount, graduation_target);
        assert_eq!(bin_index, expected_bin_index);
    }

    #[rstest]
    #[case(0, 1000192, Ok(0))]
    #[case(1, 1000192, Ok(15503))]
    #[case(32, 1000192, Ok(500096))]
    #[case(63, 1000192, Ok(984489))]
    #[case(64, 1000192, Ok(1000192))]
    #[case(65, 1000192, Err(BEYOND_GRADUATION_TARGET))]
    fn test_quote_amount_at_bin(
        #[case] bin_index: usize,
        #[case] graduation_target: u64,
        #[case] expected_quote_amount: Result<u64, CoreError>,
    ) {
        let curve: PriceCurve = linear(1 << 64, 2 << 64).try_into().unwrap();
        let quote_amount = curve.quote_amount_at_bin(bin_index, graduation_target);
        assert_eq!(quote_amount, expected_quote_amount);
    }
}
