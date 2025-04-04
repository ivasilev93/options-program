use anchor_lang::prelude::*;

declare_id!("Be2AgTUf5uVfdHaSXPpzifVkmwfkgRwtLToVywevfvrS");

#[program]
pub mod options_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
