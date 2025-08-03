use solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::{Account as TokenAccount, AccountState};

/// Enhanced token account validation with comprehensive security checks
pub fn safe_unpack_and_validate_token_account(
    account: &AccountInfo,
    account_name: &str,
    expected_owner: Option<&Pubkey>,
    expected_mint: Option<&Pubkey>,
    check_delegate: bool,
) -> Result<TokenAccount, ProgramError> {
    // Check if account has data
    if account.data_len() == 0 {
        msg!("❌ {}: Account has no data (uninitialized)", account_name);
        return Err(ProgramError::UninitializedAccount);
    }
    
    // Check if account is owned by SPL Token program
    if account.owner != &spl_token::id() {
        msg!("❌ {}: Account is not owned by SPL Token program", account_name);
        msg!("   • Expected owner: {}", spl_token::id());
        msg!("   • Actual owner: {}", account.owner);
        return Err(ProgramError::IncorrectProgramId);
    }
    
    // Try to unpack the token account data
    let token_account = TokenAccount::unpack_from_slice(&account.data.borrow())
        .map_err(|e| {
            msg!("❌ {}: Failed to unpack token account data", account_name);
            msg!("   • Error: {:?}", e);
            ProgramError::InvalidAccountData
        })?;
    
    // 🔒 SECURITY: Check if account is frozen
    if token_account.state == AccountState::Frozen {
        msg!("❌ {}: Token account is FROZEN", account_name);
        msg!("   • Account cannot be used for transfers");
        msg!("   • Owner: {}", token_account.owner);
        msg!("   • Mint: {}", token_account.mint);
        return Err(ProgramError::InvalidAccountData);
    }
    
    // 🔒 SECURITY: Check delegate authority
    if check_delegate && token_account.delegate.is_some() {
        msg!("⚠️ {}: Token account has a delegate authority", account_name);
        msg!("   • Delegate: {:?}", token_account.delegate);
        msg!("   • Delegated amount: {}", token_account.delegated_amount);
        
        // For high-security operations, reject delegated accounts
        if token_account.delegated_amount > 0 {
            msg!("❌ {}: Account has active delegation - rejected for security", account_name);
            return Err(ProgramError::InvalidAccountData);
        }
    }
    
    // 🔒 SECURITY: Validate expected owner if provided
    if let Some(expected) = expected_owner {
        if token_account.owner != *expected {
            msg!("❌ {}: Token account owner mismatch", account_name);
            msg!("   • Expected owner: {}", expected);
            msg!("   • Actual owner: {}", token_account.owner);
            return Err(ProgramError::InvalidAccountData);
        }
    }
    
    // 🔒 SECURITY: Validate expected mint if provided
    if let Some(expected) = expected_mint {
        if token_account.mint != *expected {
            msg!("❌ {}: Token account mint mismatch", account_name);
            msg!("   • Expected mint: {}", expected);
            msg!("   • Actual mint: {}", token_account.mint);
            return Err(ProgramError::InvalidAccountData);
        }
    }
    
    // 🔒 SECURITY: Warn about close authority
    if token_account.close_authority.is_some() {
        msg!("⚠️ {}: Token account has a close authority", account_name);
        msg!("   • Close authority: {:?}", token_account.close_authority);
        // Note: We don't reject these, but log for monitoring
    }
    
    msg!("✅ {}: Token account validation passed", account_name);
    msg!("   • Mint: {}", token_account.mint);
    msg!("   • Owner: {}", token_account.owner);
    msg!("   • Balance: {}", token_account.amount);
    msg!("   • State: Active (not frozen)");
    msg!("   • Delegate: {}", if token_account.delegate.is_some() { "Present" } else { "None" });
    
    Ok(token_account)
}