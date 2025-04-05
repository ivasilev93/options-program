use anchor_lang::prelude::*;
// use crate::{errors::*, state::market};
use crate::state::market::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{ self, Mint, TokenAccount, TokenInterface, TransferChecked, MintTo };

#[derive(Accounts)]
#[instruction(
    ix: u64
)]
pub struct MarketDeposit<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::token_program = token_program,
        token::authority = signer
    )]
    pub user_asset_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = lp_mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    pub user_lp_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [
            MARKET_SEED.as_bytes(),
            ix.to_le_bytes().as_ref(),
            signer.key().as_ref()
        ],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        token::mint = asset_mint,
        seeds = [
            MARKET_VAULT_SEED.as_bytes(),
            ix.to_le_bytes().as_ref(),
            signer.key().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [
            MARKET_LP_MINT_SEED.as_bytes(),
            ix.to_le_bytes().as_ref(),
        ],
        bump
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,
    
    pub asset_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl MarketDeposit<'_> {
    pub fn handle(ctx: Context<MarketDeposit>, ix: u64, amount: u64) -> Result<()> {

        //Calc lp tokens(share) to mint
        let market = &mut ctx.accounts.market;
        let market_tvl = market.premiums + market.reserve_supply;

        let lp_tokens_to_mint = if market.lp_minted == 0 {
            amount
        } else {
            (amount + market.lp_minted) / market_tvl
        };

        //Update market minted tokens
        market.lp_minted = market.lp_minted
            .checked_add(lp_tokens_to_mint).unwrap();

        //Update market total reserve
        market.reserve_supply = market.reserve_supply
            .checked_add(amount).unwrap();

        //Transfer from user to vaul
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.user_asset_ata.to_account_info(),
            to: ctx.accounts.market_vault.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
            mint: ctx.accounts.asset_mint.to_account_info()
        };

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
        token_interface::transfer_checked(cpi_context, amount, ctx.accounts.asset_mint.decimals)?;

        //Mint LP tokens
        let mint_cpi_accounts = MintTo {
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.user_lp_ata.to_account_info(),
            authority: ctx.accounts.lp_mint.to_account_info()
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new(cpi_program, mint_cpi_accounts);
        token_interface::mint_to(cpi_context, lp_tokens_to_mint)?;

        Ok(())
    }
}