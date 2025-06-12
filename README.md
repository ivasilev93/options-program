# Solana Options Trading Demo Program

### This repository contains a simplified options trading program built on the Solana blockchain using the Anchor framework. It is designed as a demonstration/portfolio project.

---
**Important notes**
- Not Audited: This program has not undergone any professional security audit and is intended solely for demonstration purposes.
- Simplified Economic Model: The economic model is significantly simplified and does not reflect real-world financial complexities.
- Black-Scholes Model: The premium calculation uses a simplified Black-Scholes model with compromises, such as assuming a 0% risk-free rate, using approximations for computational efficiency and manually fed volatility.
---
 
## Architecture Overview
This program implements a basic options trading platform on Solana, allowing

1. Admins to create markets for trading options.
Each market has:
- A dedicated LP mint (for issuing LP tokens)
- A vault to hold asset liquidity
- A protocol fee vault

```rust
 #[account(
        init,
        payer = signer,
        seeds = [
            MARKET_LP_MINT_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref(),
        ],
        mint::decimals = asset_mint.decimals,
        mint::authority = lp_mint.key(),
        mint::freeze_authority = lp_mint.key(),
        bump
    )]
    pub lp_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = signer,
        seeds = [
            MARKET_SEED.as_bytes(),
            params.ix.to_le_bytes().as_ref()
        ],
        bump,
        space = 8 + Market::INIT_SPACE
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = signer,
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
        init,
        payer = signer,
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
```

2. Market Makers to deposit asset (receiving LP tokens) and withdraw by burning LP tokens
- LPs deposit underlying tokens into the market -> Receive proportional LP tokens (based on existing reserve and supply)
- Vault is updated, LP token supply grows
- Can later burn LP tokens to withdraw their share of the market's reserve
- LP token calculation: lp_tokens_to_mint = (base_asset_amount * lp_minted) / market_tvl (scaled for precision).

3. Options: Takers
Buying options:
- Takers specify option typ [CALL; PUT], strike price (USD), expiry timestamp, and quantity
- Current asset price is fetched from Pyth oracle
- Premium is calculated via Black-Scholes, and split:
    - protocol_fee to fee vault
    - lp_share into the reserve
- Option is stored in users's account:
```rust
#[account(zero_copy)]
#[derive(InitSpace, PartialEq, Eq)]
pub struct UserAccount {
    pub options: [OptionOrder; 32]
}

#[derive(PartialEq, Eq,InitSpace)]
#[zero_copy]
#[repr(C)]
pub struct OptionOrder {
    pub strike_price: u64,  //scaled by 10^6
    pub expiry: i64,
    pub premium: u64,
    pub quantity: u64,
    pub max_potential_payout_in_tokens: u64,
    pub market_ix: u16,
    pub option_type: u8,
    pub ix: u8,
    pub is_used: u8,
    pub padding: [u8; 3]
}
```
- Takers can exercise options at expiry, receiving payouts based on the asset's price (via Pyth oracle) and the option's strike price.

## To build and test
In constants.rs file set the admin key to your local wallet public key:
```rust
//the public key of your local wallet
pub const ADMIN_KEY: &str = "FARXLJJbSwZyELTe8TXihES7o26B2d5NKkvCkETP7Gnz"; 
```
or comment the constraint in market_create.rs, market_close.rs, market_update_vol.rs, withdraw_fees.rs like:
```rust
#[account(
        mut,
        //constraint = signer.key() == Pubkey::from_str(ADMIN_KEY).unwrap() @ CustomError::Unauthorized
    )]
    pub signer: Signer<'info>,
```

### Run test suite
```bash
anchor build
anchor test
```

### To run function tests
```bash
cargo test -- --test-threads=1 --nocapture
```

### You can also run a separate validator and a front end app for interaction:

Run separate validator, cloning the pyth SOL/USD feed
``` bash
solana-test-validator --bind-address 0.0.0.0 --url https://api.mainnet-beta.solana.com --ledger .anchor/test-ledger --rpc-port 8899 --clone 7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE --clone 7dbob1psH1iZBS7qPsm3Kwbf5DzSXK8Jyg31CTgTnxH5 --reset
anchor deploy
```

```bash
git clone https://github.com/ivasilev93/options-program-front-end.git
npm i
npm run dev
```

