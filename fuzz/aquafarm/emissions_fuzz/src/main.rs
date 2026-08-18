//! Aquafarm emissions accounting fuzz (from SDK harvest formula).
//! h = farm_tokens * (cumulative - checkpoint) / SCALE
//! cumulative must be monotonic when emissions_per_sec > 0 and time advances.
use arbitrary::Arbitrary;
use honggfuzz::fuzz;

const SCALE: u128 = 1_000_000_000_000; // SDK uint256ToDecimal divisor

#[derive(Debug, Arbitrary)]
struct Step {
    farm_tokens: u64,
    checkpoint: u64, // scaled
    emissions_num: u32,
    emissions_den: u32,
    dt_secs: u32,
    supply: u64, // farm token supply proxy for per-token accrual
}

fn harvest(farm_tokens: u128, cumulative: u128, checkpoint: u128) -> Option<u128> {
    if cumulative < checkpoint {
        return None; // should never happen on-chain
    }
    farm_tokens.checked_mul(cumulative - checkpoint)?.checked_div(SCALE)
}

fn accrue(cumulative: u128, num: u128, den: u128, dt: u128, supply: u128) -> Option<u128> {
    if den == 0 || supply == 0 || num == 0 || dt == 0 {
        return Some(cumulative);
    }
    // delta = (num/den)*dt / supply * SCALE  ≈ num*dt*SCALE / (den*supply)
    let delta = num
        .checked_mul(dt)?
        .checked_mul(SCALE)?
        .checked_div(den.checked_mul(supply)?)?;
    cumulative.checked_add(delta)
}

fn main() {
    loop {
        fuzz!(|s: Step| {
            let den = if s.emissions_den == 0 { 1 } else { s.emissions_den as u128 };
            let supply = (s.supply as u128) % (1u128 << 40) + 1;
            let farm = (s.farm_tokens as u128) % (supply + 1);
            let mut cum = (s.checkpoint as u128) % (1u128 << 60);
            let checkpoint = cum; // start synced
            let dt = (s.dt_secs as u128) % 86_400 + 1;
            let num = s.emissions_num as u128;

            // after time passes, cumulative must not decrease
            let cum2 = accrue(cum, num, den, dt, supply).unwrap_or(cum);
            assert!(cum2 >= cum, "cumulative decreased");

            // harvest with synced checkpoint is 0
            let h0 = harvest(farm, cum, checkpoint).unwrap_or(0);
            assert_eq!(h0, 0, "synced harvest nonzero");

            // harvest after accrue
            if let Some(h) = harvest(farm, cum2, checkpoint) {
                if farm == 0 || cum2 == checkpoint {
                    assert_eq!(h, 0);
                }
                // cannot exceed absurd bound
                assert!(h <= farm.saturating_mul(cum2.saturating_sub(checkpoint)), "harvest overflow-ish");
            } else {
                // underflow path only if cum < checkpoint — forbidden
                panic!("harvest None with cum>=checkpoint");
            }

            // rewind attack: checkpoint > cumulative must fail closed in our model
            assert!(harvest(farm, cum, cum2.saturating_add(1)).is_none());
        });
    }
}
