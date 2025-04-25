use anchor_lang::prelude::*;
use crate::errors::CustomError;
use crate::state::market::*;
use crate::state::event::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{ self, Mint, TokenAccount, TokenInterface, TransferChecked, Burn };

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawParams {    
    pub lp_tokens_to_burn: u64,            
    pub min_amount_out: u64,    
    pub ix: u16,
}

#[derive(Accounts)]
#[instruction(params: WithdrawParams)]
pub struct MarketWithdraw<'info> {
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
        mut,
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

impl MarketWithdraw<'_> {
    pub fn handle(ctx: Context<MarketWithdraw>, params: WithdrawParams) -> Result<()> {
        let market = &mut ctx.accounts.market;

        let (withdraw_amount, lp_tokens_to_burn) = calc_withdraw_amount_from_lp_shares(params.lp_tokens_to_burn, &market)?;
        require!(params.min_amount_out >= withdraw_amount, CustomError::SlippageExceeded);

        let market_tvl = market.reserve_supply.checked_add(market.premiums).unwrap();
        let uncomitted_reserve = market.reserve_supply.checked_sub(market.committed_reserve).unwrap();

        let reserve_share = if market_tvl > 0 {
            (withdraw_amount as u128)
                .checked_mul(uncomitted_reserve as u128).unwrap()
                .checked_div(market_tvl as u128).unwrap() as u64
        } else {
            0
        };

        let premium_share = withdraw_amount.checked_sub(reserve_share).unwrap();

        let reserve_before = market.reserve_supply;
        let premiums_before = market.premiums;
        let lp_tokens_before = market.lp_minted;

        market.reserve_supply = market.reserve_supply.checked_sub(reserve_share).unwrap();
        market.premiums = market.premiums.checked_sub(premium_share).unwrap();
        market.lp_minted = market.lp_minted.checked_sub(params.lp_tokens_to_burn).unwrap();

        //Market vault signer seeds
        let ix_bytes = params.ix.to_le_bytes();
        let ix_bytes_ref = ix_bytes.as_ref();
        let seeds = &[MARKET_VAULT_SEED.as_bytes(), ix_bytes_ref, &[ctx.bumps.market_vault]];
        let signer_seeds = &[&seeds[..]];

        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked { 
                    from: ctx.accounts.market_vault.to_account_info(),
                    mint: ctx.accounts.asset_mint.to_account_info(), 
                    to: ctx.accounts.user_asset_ata.to_account_info(), 
                    authority: ctx.accounts.market_vault.to_account_info() 
                },
            signer_seeds),
            withdraw_amount,
            ctx.accounts.asset_mint.decimals
        )?;

         //Get signer seeds for burning
         let ix_bytes = params.ix.to_le_bytes();
         let ix_bytes_ref = ix_bytes.as_ref();
         let seeds = &[MARKET_LP_MINT_SEED.as_bytes(), ix_bytes_ref, &[ctx.bumps.lp_mint]];
         let signer_seeds = &[&seeds[..]];
        token_interface::burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                Burn {
                    from: ctx.accounts.user_lp_ata.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                    mint: ctx.accounts.lp_mint.to_account_info()
                }, 
                signer_seeds), 
                lp_tokens_to_burn)?;

        msg!("Market before: reserve - {}, premiums - {}, lp minted - {},", 
            reserve_before, premiums_before, lp_tokens_before);

        msg!("Market after: reserve - {}, premiums - {}, lp minted - {},", 
            market.reserve_supply, market.premiums, market.lp_minted);

        msg!("Burned lp tokens - {}", lp_tokens_to_burn);
        msg!("Receiven asset tokens - {}", withdraw_amount);

        emit!(MakerWithdrawEvent {
            user: ctx.accounts.signer.key(),
            market: market.id,
            market_name: market.name.clone(),
            market_asset_mint: ctx.accounts.asset_mint.key(),
            reserve_before,
            reserve_after: market.reserve_supply,
            premiums_before,
            premiums_after: market.premiums,
            lp_tokens_before: lp_tokens_before,
            lp_tokens_after: market.lp_minted,
            tokens_withdrawn: withdraw_amount,
        });        

        Ok(())
    }
    
}