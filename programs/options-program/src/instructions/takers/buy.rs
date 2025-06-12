use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use anchor_spl::token_interface::{self, *};
use crate::{common::*, errors::CustomError, math::premium::*, state::{event::OptionBought, market::*, user_account::*}};

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct BuyOptionParams {
    pub market_ix: u16,
    pub option: OptionType,
    pub strike_price_usd: u64,      //strike price in usd e.g. 120_000_000 for $120.00; 10^8
    pub expiry_setting: Expiry,
    pub quantity: u64
}

#[derive(Accounts)]
#[instruction(params: BuyOptionParams)]
pub struct BuyOption<'info> {
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
            params.market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [
            MARKET_VAULT_SEED.as_bytes(),
            params.market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [
            PROTOCOL_FEES_VAULT_SEED.as_bytes(),
            params.market_ix.to_le_bytes().as_ref()
        ],
        bump,
    )]
    pub protocol_fees_vault: InterfaceAccount<'info, TokenAccount>,

    #[account()]
    pub asset_mint: InterfaceAccount<'info, Mint>,

    #[account()]
    pub price_update: Account<'info, PriceUpdateV2>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl BuyOption<'_> {
    pub fn handle(ctx: Context<BuyOption>, params: BuyOptionParams) -> Result<()> {
        let user_account = &mut ctx.accounts.account.load_mut()?;
        let market = &mut ctx.accounts.market;

        //Check avaiable slots in array
        let slot_ix = user_account.get_available_slot()
            .ok_or(CustomError::OrdersLimitExceeded)?;
        let clock = Clock::get()?;
        let option_expiry = clock.unix_timestamp + params.expiry_setting.to_seconds().unwrap() as i64;

        //Get asset price from oracle in usd, scaled by 10^8 (Pyth)
        let price_update = &mut ctx.accounts.price_update;

        // Using increased, suboptimal, maximum age, because we are working with cloned pyth account w stale updated price 
        // let maximum_age: u64 = 100 * 60;
        let maximum_age: u64 = 90; //90 sec for mainnet

        let feed_id = get_feed_id_from_hex(market.price_feed.as_str())?;
        let price = price_update.get_price_no_older_than(&clock, maximum_age, &feed_id)?;

        //In serious production settings this should be checked for freshness
        // require!(market.vol_last_updated + 120 >= clock.unix_timestamp, CustomError::VolatilityStaled);

        let (_, total_collateral_tokens) = calculate_collateral(
            params.strike_price_usd as u128,
            price.price as u128,
            &params.option,
            &market,
            params.expiry_setting,
            params.quantity
        )?;

        let available_collateral = market.reserve_supply - market.committed_reserve;
        require!(available_collateral > total_collateral_tokens, CustomError::InsufficientColateral);  
        
        //Premium
        let (premium_usd, premium_tokens, fee_tokens) = calculate_option_premium(
            params.strike_price_usd as u128,
            price.price as u128,
            params.expiry_setting,
            market, 
            &params.option,
            params.quantity)?;        

        let lp_share = premium_tokens - fee_tokens;

        msg!("Attemp to transfer {}", premium_tokens);

        //Transfer premium to market vault
        token_interface::transfer_checked(
            CpiContext::new(ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.user_token_acc.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.signer.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info()
            }),
            premium_tokens as u64,
            ctx.accounts.asset_mint.decimals)?;

        let market_ix_bytes = params.market_ix.to_le_bytes();
        //Transfer protocol fees to fee vault
        let signer_seeds: &[&[&[u8]]] = &[&[
            MARKET_VAULT_SEED.as_bytes(),
            market_ix_bytes.as_ref(),
            &[ctx.bumps.market_vault]]];

        token_interface::transfer_checked(
            CpiContext::new(ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.market_vault.to_account_info(),
                to: ctx.accounts.protocol_fees_vault.to_account_info(),
                authority: ctx.accounts.market_vault.to_account_info(),
                mint: ctx.accounts.asset_mint.to_account_info()
            }).with_signer(signer_seeds),
            fee_tokens,
            ctx.accounts.asset_mint.decimals)?;

        market.premiums = market.premiums
            .checked_add(lp_share).ok_or(CustomError::Overflow)?;
        market.committed_reserve = market.committed_reserve
            .checked_add(total_collateral_tokens).ok_or(CustomError::Overflow)?;

        //Save user option
        user_account.options[slot_ix] = OptionOrder {
            strike_price: params.strike_price_usd,
            expiry: option_expiry,
            premium: premium_tokens,
            premium_in_usd: premium_usd,
            quantity: params.quantity,
            max_potential_payout_in_tokens: total_collateral_tokens,
            market_ix: params.market_ix,
            option_type: u8::from(params.option),
            ix: slot_ix as u8,
            is_used: 1,
            padding: [0_u8; 3]
        };

        msg!("Option has been bought: 
        option ix {} 
        market: {}
        quantity: {}
        premium in tokens: {} 
        premium in usd: {}
        bought_at_price_usd: {}
        strike_price_usd: {}
        option: {:?}
        user: {}
        ",
        slot_ix,
        params.market_ix,
        params.quantity,
        premium_tokens,
        premium_usd,
        price.price,
        params.strike_price_usd,
        params.option.clone(),
        ctx.accounts.signer.key());

        //Off-chain proccess could listen for those events and schedule authorized exercise at expiry time on user(taker)'s behalf for convenience...
        emit!(OptionBought {
            market: params.market_ix,
            expiry_stamp: option_expiry,
            max_potential_payout_in_tokens: total_collateral_tokens,
            quantity: params.quantity,
            strike_price_usd: params.strike_price_usd as u64, //TODO u64::tryinto() better approach
            bought_at_price_usd: price.price as u64, 
            option: params.option.clone(),
            user: ctx.accounts.signer.key(),
            option_ix: slot_ix as u8
        });

        Ok(())
    }
}