use crate::generated::programs::WAVEBREAK_ID;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub fn get_lp_escrow_address(authority: &Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let seeds = &[b"lp_escrow", authority.as_ref()];

    Pubkey::try_find_program_address(seeds, &WAVEBREAK_ID).ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::pubkey;

    #[test]
    fn test_get_lp_escrow_address() {
        let authority = pubkey!("A1tYHa3233WKDX5fZuZNmHMUVTSB12sR1RoVeGT8XV85");
        let (address, _) = get_lp_escrow_address(&authority).unwrap();
        let lp_escrow = pubkey!("DKX4hTCVRzu9nLJJvmtxiKU2wBLUyQby5k6cMDgNsSz2");
        assert_eq!(address, lp_escrow);
    }
}
