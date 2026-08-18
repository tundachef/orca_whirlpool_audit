use crate::generated::programs::WAVEBREAK_ID;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub fn get_mint_config_address(
    quote_token_mint: &Pubkey,
    instruction_discriminator: u8,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds = &[
        b"mint_config",
        quote_token_mint.as_ref(),
        &[instruction_discriminator],
    ];

    Pubkey::try_find_program_address(seeds, &WAVEBREAK_ID).ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::pubkey;

    #[test]
    fn test_get_mint_config_address() {
        let token_mint = pubkey!("A1tYHa3233WKDX5fZuZNmHMUVTSB12sR1RoVeGT8XV85");
        let (address, _) = get_mint_config_address(&token_mint, 10).unwrap();
        let mint_config = pubkey!("4UUnyGNdumaALFL21mFwocZvLuoqFeBrszJkkET3LJJH");
        assert_eq!(address, mint_config);
    }
}
