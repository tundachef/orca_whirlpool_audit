#![allow(unexpected_cfgs)]

#[allow(clippy::all, unused_imports)]
mod generated;
mod math;
mod pda;

pub use generated::accounts::*;
pub use generated::errors::*;
pub use generated::instructions::*;
pub use generated::programs::WAVEBREAK_ID as ID;
pub use generated::programs::*;
#[cfg(feature = "fetch")]
pub use generated::shared::*;
pub use generated::types::*;

pub use math::*;

#[cfg(feature = "fetch")]
pub(crate) use generated::*;

pub use pda::*;

use math::curve::PriceCurveFacade;

impl From<BondingCurve> for PriceCurveFacade {
    fn from(bonding_curve: BondingCurve) -> Self {
        PriceCurveFacade {
            start_price: bonding_curve.start_price,
            end_price: bonding_curve.end_price,
            control_points: bonding_curve.control_points,
        }
    }
}

impl From<MintConfig> for PriceCurveFacade {
    fn from(mint_config: MintConfig) -> Self {
        PriceCurveFacade {
            start_price: mint_config.default_start_price,
            end_price: mint_config.default_end_price,
            control_points: mint_config.default_control_points,
        }
    }
}
