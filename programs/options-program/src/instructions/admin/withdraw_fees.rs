use std::str::FromStr;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{ self, * };
use crate::errors::*;
use crate::state::market::*;
use crate::constants::ADMIN_KEY;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawFeesParams {
    pub ix: u16,
}

#[derive(Accounts)]
#[instruction(params: WithdrawFeesParams)]
pub struct WithdrawFees<'info> {
    #[account(
        mut,
        // constraint = signer.key() == Pubkey::from_str(ADMIN_KEY).unwrap() @ CustomError::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = admin
    )]
    pub admin_token_acc: InterfaceAccount<'info, TokenAccount>,

    #[account()]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        token::mint = asset_mint,
        token::authority = protocol_fees_vault,
        token::token_program = token_program,
        seeds = [
            PROTOCOL_FEES_VAULT_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub protocol_fees_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>
}

impl WithdrawFees<'_> {
    pub fn handle(ctx: Context<WithdrawFees>, params: WithdrawFeesParams) -> Result<()> {
        let fees_vault = &mut ctx.accounts.protocol_fees_vault;
        let amount = fees_vault.amount;
        
        let market_ix_bytes = params.ix.to_le_bytes();
        
        let signer_seeds: &[&[&[u8]]] = &[&[
            PROTOCOL_FEES_VAULT_SEED.as_bytes(),
            market_ix_bytes.as_ref(),
            &[ctx.bumps.protocol_fees_vault]]];

        msg!("Transfering {} tokens to admin account {}", amount, ctx.accounts.admin.key());

        token_interface::transfer_checked(
            CpiContext::new(ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.protocol_fees_vault.to_account_info(),
                to: ctx.accounts.admin_token_acc.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info()
            }).with_signer(signer_seeds),
            amount,
            ctx.accounts.asset_mint.decimals)?;

        Ok(())
    }
}

