use crate::generated::programs::WAVEBREAK_ID;
use crate::PermissionSignature;
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

pub fn get_permission_config_address(
    consumer_program: &Pubkey,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds = &[b"permission_config", consumer_program.as_ref()];
    Pubkey::try_find_program_address(seeds, &WAVEBREAK_ID).ok_or(ProgramError::InvalidSeeds)
}

pub fn get_consumed_permission_address(
    signature: &PermissionSignature,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds = &[
        b"consumed_permission",
        signature.bytes[0..32].as_ref(),
        signature.bytes[32..64].as_ref(),
    ];
    Pubkey::try_find_program_address(seeds, &WAVEBREAK_ID).ok_or(ProgramError::InvalidSeeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::pubkey;

    #[test]
    fn test_get_permission_address() {
        let consumer_program = pubkey!("62dSkn5ktwY1PoKPNMArZA4bZsvyemuknWUnnQ2ATTuN");
        let (address, _) = get_permission_config_address(&consumer_program).unwrap();
        let permission_config = pubkey!("GBriG7QANWP33M5diRdCUruEXcWmu3YCo8kAjEJDEUA9");
        assert_eq!(address, permission_config);
    }

    #[test]
    fn test_get_consumed_permission_address() {
        let signature = PermissionSignature {
            bytes: [5u8; 64],
            recovery_id: 0,
        };
        let (address, _) = get_consumed_permission_address(&signature).unwrap();
        let consumed_permission = pubkey!("8oS8bTZJidTSVDU2suCTWRWs4fSBKDMY58vTBNGBVgGe");
        assert_eq!(address, consumed_permission);
    }
}
