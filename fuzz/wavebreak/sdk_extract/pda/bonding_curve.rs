use crate::generated::programs::WAVEBREAK_ID;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub fn get_bonding_curve_address(base_mint: &Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let seeds = &[b"bonding_curve", base_mint.as_ref()];

    Pubkey::try_find_program_address(seeds, &WAVEBREAK_ID).ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::pubkey;

    #[test]
    fn test_get_bonding_curve_address() {
        let token_mint = pubkey!("62dSkn5ktwY1PoKPNMArZA4bZsvyemuknWUnnQ2ATTuN");
        let (address, _) = get_bonding_curve_address(&token_mint).unwrap();
        let bonding_curve = pubkey!("umyTygGGkyBw4oCxxKRPrkFCAFg1bL7DqSHwtgfKoh3");
        assert_eq!(address, bonding_curve);
    }
}
