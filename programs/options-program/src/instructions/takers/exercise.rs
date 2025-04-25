use core::cmp::{max, min};

use crate::common::OptionType;
use crate::constants::EXERCISE_INTERVAL_TOLERANCE;
use crate::constants::STRIKE_PRICE_DECIMALS;
use crate::errors::*;
use crate::state::event::*;
use crate::state::user_account::*;
use crate::state::market::*;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{ self, Mint, TokenAccount, TokenInterface, TransferChecked };
use pyth_solana_receiver_sdk::price_update::*;

#[derive(Accounts)]
#[instruction(
    market_ix: u16,
    option_id: u8
)]
pub struct ExerciseOption<'info> {

    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            USR_ACC_SEED.as_bytes(),
            signer.key().as_ref()
        ],
        bump
    )]
    pub account: AccountLoader<'info, UserAccount>,

    #[account(
        mut,
        token::mint = asset_mint,
        token::authority = signer
    )]
    pub user_token_acc: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [
            MARKET_SEED.as_bytes(),
            market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [
            MARKET_VAULT_SEED.as_bytes(),
            market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

    pub asset_mint: InterfaceAccount<'info, Mint>,
    pub price_update: Account<'info, PriceUpdateV2>,
    pub token_program: Interface<'info, TokenInterface>,
    
}

impl ExerciseOption<'_> {
    pub fn handle(ctx: Context<ExerciseOption>, market_ix: u16, option_id: u8) -> Result<()> {
        let user_account = &mut ctx.accounts.account.load_mut()?;
        let market = &mut ctx.accounts.market;
        let option = &mut user_account.options[option_id as usize];

        let mut user_payout_in_tokens = 0u64;

        let stamp_now = Clock::get()?.unix_timestamp;
        require!(stamp_now <= option.expiry + EXERCISE_INTERVAL_TOLERANCE, CustomError::ExerciseIsOverdue);

        //Get asset price from oracle in usd, scaled by 10^6 (Pyth)
        let price_update = &mut ctx.accounts.price_update;
        let maximum_age: u64 = 100* 60;
        let feed_id = get_feed_id_from_hex(market.price_feed.as_str())?;
        let price = price_update.get_price_no_older_than(&Clock::get()?, maximum_age, &feed_id)?;

        let pyth_decimals = price.exponent.abs() as u32; 
        
        let scaled_strike_price = if pyth_decimals >= STRIKE_PRICE_DECIMALS {
            option.strike_price * 10u64.pow(pyth_decimals - STRIKE_PRICE_DECIMALS)
        } else {
            option.strike_price / 10u64.pow(STRIKE_PRICE_DECIMALS - pyth_decimals)
        };        

        let profit_usd = match OptionType::try_from(option.option_type).unwrap() {
            OptionType::CALL => {
                (price.price as u64)
                    .saturating_sub(scaled_strike_price) 
                    .checked_mul(option.quantity).unwrap()
            },
            OptionType::PUT => {
                (scaled_strike_price as u64)
                    .saturating_sub(price.price as u64)
                    .checked_mul(option.quantity).unwrap()
            }
        };

        // If profit > 0 transfer tokens equivalent from vault
        if profit_usd > 0 {
            let token_scaling = 10_u64.pow(market.asset_decimals as u32);

            let profit_in_tokens = (profit_usd as u128)
                .checked_mul(token_scaling as u128).unwrap()
                .checked_div(price.price as u128).unwrap() as u64;

            //There is limit to payouts for solvency
            user_payout_in_tokens = min(profit_in_tokens, option.max_potential_payout_in_tokens);

            let market_ix_bytes = market_ix.to_le_bytes();

            let signer_seeds: &[&[&[u8]]] = &[&[
                MARKET_VAULT_SEED.as_bytes(),
                market_ix_bytes.as_ref(),
                &[ctx.bumps.market_vault]]];

            token_interface::transfer_checked(
                CpiContext::new(ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.market_vault.to_account_info(),
                    to: ctx.accounts.user_token_acc.to_account_info(),
                    authority: ctx.accounts.market_vault.to_account_info(),
                    mint: ctx.accounts.asset_mint.to_account_info()
                }).with_signer(signer_seeds),
                user_payout_in_tokens,
                ctx.accounts.asset_mint.decimals)?;            

            //Update market reserve n premiums data
            if user_payout_in_tokens <= market.premiums {
                market.premiums = market.premiums
                    .checked_sub(user_payout_in_tokens)
                    .ok_or(CustomError::Overflow)?;
            } else {
                let remainder = user_payout_in_tokens - market.premiums;
                market.premiums = 0;
                market.reserve_supply = market.reserve_supply
                    .checked_sub(remainder)
                    .ok_or(CustomError::Overflow)?;
            }
        } 

        //Release commited reserve
        market.committed_reserve = market.committed_reserve
                .checked_sub(option.max_potential_payout_in_tokens)
                .ok_or(CustomError::Overflow)?;

        //clear option slot         
        option.clear();        

        //log
        msg!("User {} exercised option {} ", ctx.accounts.signer.key().to_string(), option_id);
        msg!("Payout usd (in 10^8) {} ", profit_usd);
        msg!("Payout token amount {} ", user_payout_in_tokens);
        msg!("Option type {:?} ", option.option_type);
        msg!("Option ix {} ", option_id);

        //emit event
        emit!(OptionExercised {
            market: market_ix,
            quantity: option.quantity,
            option: OptionType::try_from(option.option_type).unwrap(),
            user: ctx.accounts.signer.key(),
            option_ix: option_id as u8,
            profit_usd: profit_usd,
            user_payout: user_payout_in_tokens,
            timestamp: stamp_now
        });

        Ok(())
    }
}