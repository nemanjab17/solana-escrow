use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    msg,
    pubkey::Pubkey,
    program_pack::{Pack, IsInitialized},
    sysvar::{rent::Rent, Sysvar},
    program::invoke
};



use crate::{
    instruction::EscrowInstruction, error::EscrowError, state::Escrow
};

pub struct Processor;

impl Processor {

    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
        let instruction = EscrowInstruction::unpack(instruction_data)?;

        match instruction {
            EscrowInstruction::InitEscrow {amount} => {
                msg!("Instruction: InitEscrow");
                Self::process_init_escrow(accounts, amount, program_id)
            },
            EscrowInstruction::FinalizeEscrow {amount} => {
                msg!("Instruction: FinalizeEscrow");
                Self::process_finalize_escrow(accounts, amount, program_id)
            }
        }
    }

    fn process_init_escrow(accounts: &[AccountInfo], amount: u64, program_id: &Pubkey) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let initializer = next_account_info(account_info_iter)?;

        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let temp_token_account = next_account_info(account_info_iter)?;

        let token_to_receive_account = next_account_info(account_info_iter)?;
        if *token_to_receive_account.owner != spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let escrow_account = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

        if !rent.is_exempt(escrow_account.lamports(), escrow_account.data_len()) {
            return Err(EscrowError::NotRentExempt.into());
        }

        let mut escrow_info = Escrow::unpack_unchecked(&escrow_account.try_borrow_data()?)?;
        if escrow_info.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        
        escrow_info.is_initialized = true;
        escrow_info.initializer_pubkey = *initializer.key;
        escrow_info.temp_token_account_pubkey = *temp_token_account.key;
        escrow_info.initializer_token_to_receive_account_pubkey = *token_to_receive_account.key;
        escrow_info.expected_amount = amount;

        Escrow::pack(escrow_info, &mut escrow_account.try_borrow_mut_data()?)?;
        let (pda, _bump_seed) = Pubkey::find_program_address(&[b"escrow"], program_id);

        let token_program = next_account_info(account_info_iter)?;


        let owner_change_ix = spl_token::instruction::set_authority(
            token_program.key,
            temp_token_account.key,
            Some(&pda),
            spl_token::instruction::AuthorityType::AccountOwner,
            initializer.key,
            &[&initializer.key],
        )?;

        msg!("Calling the token program to transfer token account ownership...");
        invoke(
            &owner_change_ix,
            &[
                temp_token_account.clone(),
                initializer.clone(),
                token_program.clone(),
            ],
        )?;


        Ok(())
    }

    fn process_finalize_escrow(accounts: &[AccountInfo], amount: u64, program_id: &Pubkey) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let taker = next_account_info(account_info_iter)?;

        if !taker.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let taker_token_to_send_account_pubkey = next_account_info(account_info_iter)?;

        if *taker_token_to_send_account_pubkey.owner != spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let taker_token_to_receive_account_pubkey = next_account_info(account_info_iter)?;

        if *taker_token_to_receive_account_pubkey.owner != spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }

        let token_temp_account = next_account_info(account_info_iter)?;
        let token_temp_account_info =
        TokenAccount::unpack(&token_temp_account.try_borrow_data()?)?;

        let initializer_pubkey = next_account_info(account_info_iter)?;

        let initializer_token_to_receive_account_pubkey = next_account_info(account_info_iter)?;

        let escrow_account = next_account_info(account_info_iter)?;

        let mut escrow_info = Escrow::unpack_unchecked(&escrow_account.try_borrow_data()?)?;
        
        if !escrow_info.is_initialized() {
            return Err(ProgramError::IncorrectProgramId);
        }
        if escrow_info.initializer_token_to_receive_account_pubkey != initializer_token_to_receive_account_pubkey {
            return Err(ProgramError::IncorrectProgramId);
        }


        let token_program = next_account_info(account_info_iter)?;

        let (pda, _bump_seed) = Pubkey::find_program_address(&[b"escrow"], program_id);

        // add rest of validations here

        msg!("Calling the transfer instruction to transfer tokens from src to dst");

        let transfer_ix = spl::token::transfer(
            token_program.key,
            escrow_info.temp_token_account_pubkey.key, // src
            taker_token_to_receive_account_pubkey.key, //dst
            &pda,
            &[&pda],
            token_temp_account_info.amount
        )

        








    }
}