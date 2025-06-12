use std::str::FromStr;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{ self, Mint, TokenAccount, TokenInterface, TransferChecked, CloseAccount };
use crate::errors::*;
use crate::state::market::*;
use crate::constants::ADMIN_KEY;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CloseMarketParams {
    pub ix: u16, 
}

#[derive(Accounts)]
#[instruction(params: CloseMarketParams)]
pub struct CloseMarket<'info> {
    #[account(
        mut,
        constraint = admin.key() == Pubkey::from_str(ADMIN_KEY).unwrap() @ CustomError::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account()]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::token_program = token_program,
        token::authority = admin
    )]
    pub admin_asset_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [
            MARKET_LP_MINT_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref(),
        ],
        mint::decimals = asset_mint.decimals,
        mint::authority = lp_mint.key(),
        mint::freeze_authority = lp_mint.key(),
        bump,
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [
            MARKET_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
        close = admin
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = market_vault,
        token::token_program = token_program,
        seeds = [
            MARKET_VAULT_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
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

impl CloseMarket<'_> {
    pub fn handle(ctx: Context<CloseMarket>, params: CloseMarketParams) -> Result<()> {
        let market_vault = &mut ctx.accounts.market_vault;
        let fees_vault = &mut ctx.accounts.protocol_fees_vault;
        let market_ix_bytes = params.ix.to_le_bytes();
        
        let vault_signer_seeds: &[&[&[u8]]] = &[&[
                MARKET_VAULT_SEED.as_bytes(),
                market_ix_bytes.as_ref(),
                &[ctx.bumps.market_vault]]];

        if market_vault.amount > 0 {
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.to_account_info(),
                    TransferChecked {
                    from: market_vault.to_account_info(),
                    to: ctx.accounts.admin_asset_ata.to_account_info(),
                    authority: market_vault.to_account_info(),
                    mint: ctx.accounts.asset_mint.to_account_info()
                }).with_signer(vault_signer_seeds),
                market_vault.amount,
                ctx.accounts.asset_mint.decimals)?;
        }

        //Close market vault
        token_interface::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: market_vault.to_account_info(),
                destination: ctx.accounts.admin.to_account_info(),
                authority: market_vault.to_account_info(),
            },
            vault_signer_seeds
        ))?;


        let fees_signer_seeds: &[&[&[u8]]] = &[&[
            PROTOCOL_FEES_VAULT_SEED.as_bytes(),
            market_ix_bytes.as_ref(),
            &[ctx.bumps.protocol_fees_vault]]];

        if fees_vault.amount > 0 {
            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.to_account_info(),
                    TransferChecked {
                    from: fees_vault.to_account_info(),
                    to: ctx.accounts.admin_asset_ata.to_account_info(),
                    authority: fees_vault.to_account_info(),
                    mint: ctx.accounts.asset_mint.to_account_info()
                }).with_signer(fees_signer_seeds),
                fees_vault.amount,
                ctx.accounts.asset_mint.decimals)?;
        }

        //Close fees vault
        token_interface::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: fees_vault.to_account_info(),
                destination: ctx.accounts.admin.to_account_info(),
                authority: fees_vault.to_account_info(),
            },
            fees_signer_seeds
        ))?;
    

        Ok(())
    }
}

