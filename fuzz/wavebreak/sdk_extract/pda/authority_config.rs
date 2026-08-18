use crate::generated::programs::WAVEBREAK_ID;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub fn get_authority_config_address() -> Result<(Pubkey, u8), ProgramError> {
    let seeds: &[&[u8]] = &[b"authority_config"];

    Pubkey::try_find_program_address(seeds, &WAVEBREAK_ID).ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::pubkey;

    #[test]
    fn test_get_authority_config_address() {
        let (address, _) = get_authority_config_address().unwrap();
        let authority_config = pubkey!("5yXDawwQ5s3hZXMJjLWryvDWsNKHYKqp6vkdSfgsaee4");
        assert_eq!(address, authority_config);
    }
}
