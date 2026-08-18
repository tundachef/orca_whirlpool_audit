//! xORCA share math fuzz — mirrors `solana-program/src/util/math.rs` at HEAD.
//! Virtual offset = 100 (ERC-4626-style inflation defense).
use arbitrary::Arbitrary;
use honggfuzz::fuzz;

const VIRTUAL_XORCA_SUPPLY: u128 = 100;
const VIRTUAL_NON_ESCROWED_ORCA_AMOUNT: u128 = 100;

fn convert_orca_to_xorca(
    orca_amount_to_convert: u64,
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<u64, ()> {
    if (xorca_supply == 0) || (non_escrowed_orca_amount == 0) {
        return Ok(orca_amount_to_convert);
    }
    let xorca_supply_with_virtual = (xorca_supply as u128)
        .checked_add(VIRTUAL_XORCA_SUPPLY)
        .ok_or(())?;
    let non_escrowed_with_virtual = (non_escrowed_orca_amount as u128)
        .checked_add(VIRTUAL_NON_ESCROWED_ORCA_AMOUNT)
        .ok_or(())?;
    let out = (orca_amount_to_convert as u128)
        .checked_mul(xorca_supply_with_virtual)
        .ok_or(())?
        .checked_div(non_escrowed_with_virtual)
        .ok_or(())?;
    out.try_into().map_err(|_| ())
}

fn convert_xorca_to_orca(
    xorca_amount_to_convert: u64,
    non_escrowed_orca_amount: u64,
    xorca_supply: u64,
) -> Result<u64, ()> {
    if (xorca_supply == 0) || (non_escrowed_orca_amount == 0) {
        return Err(());
    }
    let xorca_supply_with_virtual = (xorca_supply as u128)
        .checked_add(VIRTUAL_XORCA_SUPPLY)
        .ok_or(())?;
    let non_escrowed_with_virtual = (non_escrowed_orca_amount as u128)
        .checked_add(VIRTUAL_NON_ESCROWED_ORCA_AMOUNT)
        .ok_or(())?;
    let out = (xorca_amount_to_convert as u128)
        .checked_mul(non_escrowed_with_virtual)
        .ok_or(())?
        .checked_div(xorca_supply_with_virtual)
        .ok_or(())?;
    out.try_into().map_err(|_| ())
}

#[derive(Debug, Arbitrary)]
enum Action {
    StakeThenUnstake {
        orca_in: u64,
        non_escrowed: u64,
        supply: u64,
    },
    UnstakeOnly {
        xorca_in: u64,
        non_escrowed: u64,
        supply: u64,
    },
    FullRedeemBound {
        non_escrowed: u64,
        supply: u64,
    },
    SpecialCaseNonEscrowedZero {
        orca_in: u64,
        supply: u64,
    },
}

fn main() {
    loop {
        fuzz!(|action: Action| {
            match action {
                Action::StakeThenUnstake {
                    orca_in,
                    non_escrowed,
                    supply,
                } => {
                    let orca_in = (orca_in % (1 << 48)).max(1);
                    let non_escrowed = (non_escrowed % (1 << 48)).max(1);
                    let supply = (supply % (1 << 48)).max(1);

                    if let Ok(xorca) = convert_orca_to_xorca(orca_in, non_escrowed, supply) {
                        if xorca == 0 {
                            return;
                        }
                        // Use checked adds — program cannot create overflowed vault/supply
                        // via these paths without failing CPIs / checked math elsewhere.
                        let Some(new_non) = non_escrowed.checked_add(orca_in) else {
                            return;
                        };
                        let Some(new_sup) = supply.checked_add(xorca) else {
                            return;
                        };
                        if let Ok(orca_out) = convert_xorca_to_orca(xorca, new_non, new_sup) {
                            // With virtual offsets, floor division should not create free ORCA
                            // on an immediate round-trip of the newly minted shares.
                            assert!(
                                orca_out <= orca_in,
                                "round-trip profit: in={orca_in} out={orca_out} xorca={xorca} non={non_escrowed} supply={supply}"
                            );
                        }
                    }
                }
                Action::UnstakeOnly {
                    xorca_in,
                    non_escrowed,
                    supply,
                } => {
                    let xorca_in = (xorca_in % (1 << 48)).max(1);
                    let non_escrowed = (non_escrowed % (1 << 48)).max(1);
                    let supply = (supply % (1 << 48)).max(xorca_in);
                    let _ = convert_xorca_to_orca(xorca_in, non_escrowed, supply);
                }
                Action::FullRedeemBound {
                    non_escrowed,
                    supply,
                } => {
                    let non_escrowed = (non_escrowed % (1 << 48)).max(1);
                    let supply = (supply % (1 << 48)).max(1);
                    // Redeeming 100% of supply cannot exceed vault + virtual slack.
                    if let Ok(orca) = convert_xorca_to_orca(supply, non_escrowed, supply) {
                        // out = supply * (non+100) / (supply+100) < non+100
                        assert!(orca < non_escrowed.saturating_add(100) || orca == non_escrowed);
                        assert!(orca <= non_escrowed.saturating_add(99));
                    }
                }
                Action::SpecialCaseNonEscrowedZero { orca_in, supply } => {
                    // Documented special case: when non_escrowed==0, mint is 1:1.
                    let orca_in = (orca_in % (1 << 40)).max(1);
                    let supply = supply % (1 << 40);
                    let minted = convert_orca_to_xorca(orca_in, 0, supply).unwrap();
                    assert_eq!(minted, orca_in);
                    if supply > 0 {
                        // Immediate fair value of those shares after deposit is << orca_in
                        // when dead supply exists — user-hostile, not a vault drain.
                        let new_non = orca_in;
                        let new_sup = supply.saturating_add(orca_in);
                        if let Ok(out) = convert_xorca_to_orca(orca_in, new_non, new_sup) {
                            assert!(out <= orca_in);
                        }
                    }
                }
            }
        });
    }
}
