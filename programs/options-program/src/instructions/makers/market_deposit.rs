use anchor_lang::prelude::*;
use crate::errors::CustomError;
use crate::state::market::*;
use crate::state::event::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{ self, Mint, TokenAccount, TokenInterface, TransferChecked, MintTo };

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositIx {
    pub amount: u64,
    pub min_amount_out: u64,
    pub ix: u16,
}

#[derive(Accounts)]
#[instruction(params: DepositIx)]
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
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        token::mint = asset_mint,
        seeds = [
            MARKET_VAULT_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [
            MARKET_LP_MINT_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref(),
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
    pub fn handle(ctx: Context<MarketDeposit>, amount: u64, min_amount_out: u64, ix: u16) -> Result<()> {

        //Calc lp tokens(share) to mint
        let market = &mut ctx.accounts.market;
        let lp_tokens_before = market.lp_minted;
        let market_reserve_before = market.reserve_supply;
        let lp_tokens_to_mint = calc_lp_shares(amount, min_amount_out, market)?;

        //Update market 
        market.lp_minted = market.lp_minted
            .checked_add(lp_tokens_to_mint).unwrap();
        market.reserve_supply = market.reserve_supply
            .checked_add(amount).unwrap();

        msg!("Market: {} {}. Reserve vefore: {}. Reserve after: {}",
            market.id,
            market.name,
            market_reserve_before,
            market.reserve_supply);

        msg!("Minting {} LP tokens. LP tokens before: {}. LP tokens after: {}",
            lp_tokens_to_mint,
            lp_tokens_before,
            market.lp_minted);

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

        let ix_bytes = ix.to_le_bytes();
        let ix_bytes_ref = ix_bytes.as_ref();
        let seeds = &[MARKET_LP_MINT_SEED.as_bytes(), ix_bytes_ref, &[ctx.bumps.lp_mint]];
        let signer_seeds = &[&seeds[..]];
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_context = CpiContext::new_with_signer(cpi_program, mint_cpi_accounts, signer_seeds);
        token_interface::mint_to(cpi_context, lp_tokens_to_mint)?;  

        //Emit event for indexers, bots, analytics services, ect...
        //emit_cpi not needed if this is not expected to be invoked by other program CPI
        emit!(MakerDepositEvent {
            user: ctx.accounts.signer.key(),
            market: market.id,
            market_name: market.name.clone(),
            market_asset_mint: ctx.accounts.asset_mint.key(),
            market_reserve_before: market_reserve_before,
            market_reserve_after: market.reserve_supply,
            lp_tokens_minted: lp_tokens_to_mint,
            tokens_deposited: amount
        });

        Ok(())
    }
}