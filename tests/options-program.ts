import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OptionsProgram } from "../target/types/options_program";
import { AccountInfo, Connection, LAMPORTS_PER_SOL, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js'
import { assert, expect } from "chai";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { amountToUiAmount, ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, createAssociatedTokenAccountInstruction, createSyncNativeInstruction, getAssociatedTokenAddress, getOrCreateAssociatedTokenAccount, NATIVE_MINT, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";

describe("options-program test suite", async () => {
  // Client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.options_program as Program<OptionsProgram>;
  const wallet = provider.wallet as anchor.Wallet;
  console.log('Using Local Wallet: ', wallet.publicKey); 
  console.log('The options program uses FARXLJJbSwZyELTe8TXihES7o26B2d5NKkvCkETP7Gnz as admin authority.'); 
  console.log('Program Id: ', program.programId);

  // --- ACCOUNTS --- //
  const admin = anchor.web3.Keypair.generate();
  const alice = anchor.web3.Keypair.generate();
  const bob = anchor.web3.Keypair.generate();
  const john = anchor.web3.Keypair.generate();

  // let admin_wsol_acc: PublicKey;
  let alice_wsol_acc: PublicKey;
  let bob_wsol_acc: PublicKey;
  let john_wsol_acc: PublicKey;

  let alice_lp_token_acc: PublicKey;
  let bob_lp_token_acc: PublicKey;


  // --- TEST CONSTANTS --- //
  const SOL_USD_PRICE_FEED_ID = '0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d';
  const marketIx = 1;    
  const SECONDS_IN_A_WEEK = 7 * 24 * 60 * 60;

  const START_SOL_BALANCE = 1001 * LAMPORTS_PER_SOL;
  const DEPOSIT_AMOUNT = 1000 * LAMPORTS_PER_SOL;
  const ADMIN_START_BALANCE = 1 * LAMPORTS_PER_SOL;

  // --- TEST PDAs --- //
  const [marketPDA,] = await anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from('market'),
      Buffer.from(new Uint16Array([marketIx]).buffer)
    ],
    program.programId
  );
  const [marketVaultPDA,] = await anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from('market_vault'),
      Buffer.from(new Uint16Array([marketIx]).buffer)
    ],
    program.programId
  );
  const [lpMintPDA,] = await anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from('market_lp_mint'),
      Buffer.from(new Uint16Array([marketIx]).buffer)
    ],
    program.programId
  );
  const [protocolFeesVault,] = await anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from('protocol_fees_vault'),
      Buffer.from(new Uint16Array([marketIx]).buffer)
    ],
    program.programId
  );

  before("Airdrop to wallets and sync wSOL", async () => {
    await airdropToWallets(
      [alice, bob, john], 
      START_SOL_BALANCE,
      provider.connection
    );

    await airdropToWallets(
      [admin], 
      ADMIN_START_BALANCE,
      provider.connection
    );

    [alice_wsol_acc, bob_wsol_acc, john_wsol_acc] = await wrapwSolAccounts(
      [alice, bob, john], 
      DEPOSIT_AMOUNT, 
      provider.connection
    );

    alice_lp_token_acc = await getAssociatedTokenAddress(
      lpMintPDA,
      alice.publicKey
    );

    bob_lp_token_acc = await getAssociatedTokenAddress(
      lpMintPDA,
      bob.publicKey
    );
  })

  // it("Can marketPDA be DoS-ed?", async () => {
  //   console.log('market PDA', marketPDA.toBase58())
  //   console.log('Alice pk', alice.publicKey.toBase58())

  //   const MARKET_SPACE = 8 + 230; 
  //   const lamports = await provider.connection.getMinimumBalanceForRentExemption(MARKET_SPACE);
  //   const tx = new anchor.web3.Transaction();
  //   tx.add(
  //     anchor.web3.SystemProgram.createAccount({
  //       fromPubkey: alice.publicKey,
  //       newAccountPubkey: marketPDA,
  //       lamports,
  //       programId: program.programId,
  //       space: MARKET_SPACE
  //     })
  //   )

  //   const txSign = await provider.sendAndConfirm(tx, [alice]); 
  //   console.log('Market PDA creation transaction signature:', txSign);

  //   // 3. (Optional) Confirm it exists
  //   const acc = await provider.connection.getAccountInfo(marketPDA);
  //   console.log('dosed acc', acc);

  // })

  // it("Ensure non-admin cannot create new market", async () => {
  //   const nonAdmin = anchor.web3.Keypair.generate();
  //   const airdropSignature = await provider.connection.requestAirdrop(
  //     nonAdmin.publicKey,
  //     10 * LAMPORTS_PER_SOL
  //   );
  //   let latestBlockHash = await provider.connection.getLatestBlockhash();
  //   await provider.connection.confirmTransaction({
  //     blockhash: latestBlockHash.blockhash,
  //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
  //     signature: airdropSignature,
  //   });

  //   try {
  //     const createMarketSignature = await program.methods.createMarket({
  //       fee: new anchor.BN(50),
  //       name: 'wSOL market',
  //       ix: marketIx,
  //       priceFeed: SOL_USD_PRICE_FEED_ID,
  //       hour1VolatilityBps: 10000,
  //       hour4VolatilityBps: 10000,
  //       day1VolatilityBps: 10000,
  //       day3VolatilityBps: 10000,
  //       weekVolatilityBps: 10000,
  //     })
  //       .accountsStrict({
  //         market: marketPDA,
  //         marketVault: marketVaultPDA,
  //         lpMint: lpMintPDA,
  //         assetMint: NATIVE_MINT,
  //         protocolFeesVault: protocolFeesVault,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         signer: nonAdmin.publicKey,
  //         systemProgram: SYSTEM_PROGRAM_ID
  //     })
  //     .signers([nonAdmin])
  //     .rpc();
  
  //     latestBlockHash = await provider.connection.getLatestBlockhash();
  //     await provider.connection.confirmTransaction({
  //       blockhash: latestBlockHash.blockhash,
  //       lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
  //       signature: createMarketSignature,
  //     });
  //   } catch (err) {
  //     assert.strictEqual(err.error.errorCode.code, "Unauthorized");
  //     assert.strictEqual(
  //       err.error.errorMessage,
  //       "Unauthorized",
  //     );
  //   }
  // })

  it("Admin can create new market", async () => {
    const createMarketSignature = await program.methods.createMarket({
      fee: new anchor.BN(50),
      name: 'wSOL market',
      ix: marketIx,
      priceFeed: SOL_USD_PRICE_FEED_ID,
      hour1VolatilityBps: 6000,
      hour4VolatilityBps: 7000,
      day1VolatilityBps: 9000,
      day3VolatilityBps: 8000,
      weekVolatilityBps: 5000,
    }) 
      .accountsStrict({
        market: marketPDA,
        marketVault: marketVaultPDA,
        protocolFeesVault: protocolFeesVault,
        lpMint: lpMintPDA,
        assetMint: NATIVE_MINT,
        tokenProgram: TOKEN_PROGRAM_ID,
        signer: admin.publicKey,
        systemProgram: SYSTEM_PROGRAM_ID
    })
    .signers([admin])
    .rpc();
  
    const latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: createMarketSignature,
    });

    console.log('Admin create market tx: ', createMarketSignature);
  })

  it("Alice can deposit into pool", async () => {
    await createAssociatedTokenAccount(
      provider.connection,
      alice,         
      lpMintPDA,            
      alice.publicKey 
    );    

    //Alice deposits
    //Min amount left to 1 for simplicity. To be estimated in frontend when using the app as a whole.
    await program.methods
      .marketDeposit({amount: new anchor.BN(DEPOSIT_AMOUNT), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
      .accountsStrict({
        signer: alice.publicKey,
        userAssetAta: alice_wsol_acc,
        userLpAta: alice_lp_token_acc,
        market: marketPDA,
        marketVault: marketVaultPDA,
        lpMint: lpMintPDA,
        assetMint: NATIVE_MINT,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram:SYSTEM_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([alice])
      .rpc();

      //Initial LP tokens should be equal to deposit amount
      const aliceLpBalance = await provider.connection.getTokenAccountBalance(alice_lp_token_acc);
      console.log('Alice LP acc balance: ', aliceLpBalance.value);
      assert.equal(Number(aliceLpBalance.value.amount), DEPOSIT_AMOUNT * 1000, "Initial LP tokens should be 1000 more than deposit amount");    

      const marketVaultBalance = await provider.connection.getTokenAccountBalance(marketVaultPDA);
      assert.equal(DEPOSIT_AMOUNT, Number(marketVaultBalance.value.amount), "Market balance should be equal to deposit amount");    
      console.log('Market vault balance: ', marketVaultBalance.value);

      const market = await program.account.market.fetch(marketPDA);
      console.log("Market Account:");
      console.log({
        id: market.id,
        name: market.name,
        fee_bps: market.feeBps.toString(),
        bump: market.bump,
        reserve_supply: market.reserveSupply.toString(),
        committed_reserve: market.committedReserve.toString(),
        premiums: market.premiums.toString(),
        lp_minted: market.lpMinted.toString(),
        hour1VolatilityBps: market.hour1VolatilityBps,
        hour4VolatilityBps: market.hour4VolatilityBps,
        day1VolatilityBps: market.day1VolatilityBps,
        day3VolatilityBps: market.day3VolatilityBps,
        weekVolatilityBps: market.weekVolatilityBps,
        price_feed: market.priceFeed,
        asset_decimals: market.assetDecimals,
      });
  })

  it("Takers (John) can create account and buy option", async () => {

    // Derive the PDA (Program Derived Address)
    const [john_taker_accountPda, bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("account"), john.publicKey.toBuffer()],
      program.programId
    );

    console.log("John (taker) acc PDA:", john_taker_accountPda.toBase58());

    // Send the transaction to create the account
    await program.methods
      .createAccount()
      .accountsStrict({
        signer: john.publicKey,
        account: john_taker_accountPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([john])
      .rpc();

    const tx = await program.methods.buy({
      marketIx: marketIx,
      option: { call: {} },
      quantity: new anchor.BN(1000),
      expirySetting: { hour4: {}},
      spotDeviation: { n10: {} } // new anchor.BN(14000000000)
    }).accountsStrict({
      account: john_taker_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      protocolFeesVault: protocolFeesVault,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: john_wsol_acc,
      priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
      signer: john.publicKey
    })
    .signers([john])
    .rpc();

    console.log("John buys option Tx Signature:", tx);

    const john_accountData = await program.account.userAccount.fetch(john_taker_accountPda);
    console.log("First Option:");
    const opt = john_accountData.options[0];
    console.log({
      marketIx: opt.marketIx,
      optionType: opt.optionType,
      strikePrice: opt.strikePrice.toString(),
      expiry: new Date(opt.expiry.toNumber() * 1000).toISOString(),
      premium: opt.premium.toString(),
    });

    const marketVaultBalance = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    console.log('Market vault balance: ', marketVaultBalance.value);

    const market = await program.account.market.fetch(marketPDA);
    console.log("Market Account:");
    console.log({
        id: market.id,
        name: market.name,
        fee_bps: market.feeBps.toString(),
        bump: market.bump,
        reserve_supply: market.reserveSupply.toString(),
        committed_reserve: market.committedReserve.toString(),
        premiums: market.premiums.toString(),
        lp_minted: market.lpMinted.toString(),
        price_feed: market.priceFeed,
        asset_decimals: market.assetDecimals,
      });

  });

  it("Takers (John) can exercise his option. Market(pool) accrues premiums", async () => {
    // Derive the PDA (Program Derived Address)
    const [john_taker_accountPda, bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("account"), john.publicKey.toBuffer()],
      program.programId
    );

    await program.methods.exercise({
      marketIx: marketIx,
      optionId: 0
    }).accountsStrict({
      account: john_taker_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
      signer: john.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: john_wsol_acc,
    })
    .signers([john]).rpc();

    const john_accountData = await program.account.userAccount.fetch(john_taker_accountPda);
    const opt = john_accountData.options[0];
    assert(opt.strikePrice.toNumber() === 0 && opt.expiry.toNumber() === 0 && opt.quantity.toNumber() === 0, "Option not in cleared state");

    const market = await program.account.market.fetch(marketPDA);
    assert(market.committedReserve.toNumber() === 0, "Market should have no collateral");
  });

  it("Bob deposits after premium are accumulated and should receive less shares than Alice", async () => {
    const market = await program.account.market.fetch(marketPDA);
    console.log("b0b - Market Account:");
    console.log({
        id: market.id,
        name: market.name,
        fee_bps: market.feeBps.toString(),
        bump: market.bump,
        reserve_supply: market.reserveSupply.toString(),
        committed_reserve: market.committedReserve.toString(),
        premiums: market.premiums.toString(),
        lp_minted: market.lpMinted.toString(),
        // volatility_bps: market.volatilityBps,
        price_feed: market.priceFeed,
        asset_decimals: market.assetDecimals,
      });

    await createAssociatedTokenAccount(
      provider.connection,
      bob,         
      lpMintPDA,            
      bob.publicKey 
    );

    //Bob deposits
    await program.methods
    .marketDeposit({amount: new anchor.BN(DEPOSIT_AMOUNT), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
    .accountsStrict({
      signer: bob.publicKey,
      userAssetAta: bob_wsol_acc,
      userLpAta: bob_lp_token_acc,
      market: marketPDA,
      marketVault: marketVaultPDA,
      lpMint: lpMintPDA,
      assetMint: NATIVE_MINT,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram:SYSTEM_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .signers([bob])
    .rpc();

    const aliceLpBalance = await provider.connection.getTokenAccountBalance(alice_lp_token_acc);
    
    const bobLpBalance = await provider.connection.getTokenAccountBalance(bob_lp_token_acc);
    console.log('Alice LP token amount: ', aliceLpBalance.value);
    console.log('Bob LP token amount: ', bobLpBalance.value);
    assert(Number(bobLpBalance.value.amount) < Number(aliceLpBalance.value.amount), "Bob should have less LP minted than Alice");
  });

  it("Market accrues more fees (traders buy/exercise options)", async () => {

    const [john_taker_accountPda,] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("account"), john.publicKey.toBuffer()],
      program.programId
    );

    //Buy option
    const tx = await program.methods.buy({
      marketIx: marketIx,
      option: { call: {} },
      quantity: new anchor.BN(1000),
      expirySetting: { day1: {} },
      spotDeviation: { n10: {} } // new anchor.BN(14000000000)
    }).accountsStrict({
      account: john_taker_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      protocolFeesVault: protocolFeesVault,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: john_wsol_acc,
      priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
      signer: john.publicKey
    })
    .signers([john])
    .rpc();

    //Exercise
    await program.methods.exercise({
      marketIx: marketIx,
      optionId: 0
    }).accountsStrict({
      account: john_taker_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
      signer: john.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: john_wsol_acc,
    })
    .signers([john]).rpc();

    const market = await program.account.market.fetch(marketPDA);
    console.log("Market Account:");
    console.log({
        id: market.id,
        name: market.name,
        fee_bps: market.feeBps.toString(),
        bump: market.bump,
        reserve_supply: market.reserveSupply.toString(),
        committed_reserve: market.committedReserve.toString(),
        premiums: market.premiums.toString(),
        lp_minted: market.lpMinted.toString(),
        // volatility_bps: market.volatilityBps,
        price_feed: market.priceFeed,
        asset_decimals: market.assetDecimals,
      });
  })

  it("Depositors (Bob and Alice) withdraw", async () => {
    let market = await program.account.market.fetch(marketPDA);
    console.log("f - Market Account:");
    console.log({
        id: market.id,
        name: market.name,
        fee_bps: market.feeBps.toString(),
        bump: market.bump,
        reserve_supply: market.reserveSupply.toString(),
        committed_reserve: market.committedReserve.toString(),
        premiums: market.premiums.toString(),
        lp_minted: market.lpMinted.toString(),
        // volatility_bps: market.volatilityBps,
        price_feed: market.priceFeed,
        asset_decimals: market.assetDecimals,
      });

    //Alice withdraws
    const aliceLpBalance = await provider.connection.getTokenAccountBalance(alice_lp_token_acc);
    const aliceLpTokenAmount = new anchor.BN(aliceLpBalance.value.amount);
    const estAliceWithdrawAmnt = calcWithdrawAmountFromLpShares(
      aliceLpTokenAmount, market.lpMinted, market.premiums, market.reserveSupply, market.committedReserve);

    await program.methods.marketWithdraw({
      ix: marketIx,
      lpTokensToBurn: aliceLpTokenAmount,
      minAmountOut: estAliceWithdrawAmnt.withdrawableAmount,
    }).accountsStrict({
      signer: alice.publicKey,
      userAssetAta: alice_wsol_acc,
      userLpAta: alice_lp_token_acc,
      market: marketPDA,
      marketVault: marketVaultPDA,
      lpMint: lpMintPDA,
      assetMint: NATIVE_MINT,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram:SYSTEM_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .signers([alice]).rpc();

    //Bob withdraws
    market = await program.account.market.fetch(marketPDA);
    const bobLpBalance = await provider.connection.getTokenAccountBalance(bob_lp_token_acc);
    const bobLpTokenAmount = new anchor.BN(bobLpBalance.value.amount);
    const aliceMinTokenAmount = calcWithdrawAmountFromLpShares(
      bobLpTokenAmount, market.lpMinted, market.premiums, market.reserveSupply, market.committedReserve);

    await program.methods.marketWithdraw({
      ix: marketIx,
      lpTokensToBurn: bobLpTokenAmount,
      minAmountOut:  aliceMinTokenAmount.withdrawableAmount,
    }).accountsStrict({
      signer: bob.publicKey,
      userAssetAta: bob_wsol_acc,
      userLpAta: bob_lp_token_acc,
      market: marketPDA,
      marketVault: marketVaultPDA,
      lpMint: lpMintPDA,
      assetMint: NATIVE_MINT,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram:SYSTEM_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .signers([bob]).rpc();

    const alicewSOLbalance = await provider.connection.getTokenAccountBalance(alice_wsol_acc);
    console.log('alice wsol: ', alicewSOLbalance.value.amount, alicewSOLbalance.value.uiAmount)
    const bobwSOLbalance = await provider.connection.getTokenAccountBalance(bob_wsol_acc);
    console.log('bob wsol: ', bobwSOLbalance.value.amount, bobwSOLbalance.value.uiAmount)

    assert(Number(alicewSOLbalance.value.amount) > Number(bobwSOLbalance.value.amount), "Alice should have more asset tokens that Bob");
    assert(Number(alicewSOLbalance.value.amount) > DEPOSIT_AMOUNT, "Alice should have more than 1000 wSOL tokens");
    assert(Number(bobwSOLbalance.value.amount) > DEPOSIT_AMOUNT, "Bob should have more than 1000 wSOL tokens");
  })

  it("Admin can close market. Should receive protocol fees. Associated market PDAs must be closed", async () => {
    let market = await program.account.market.fetch(marketPDA);
    let marketVault = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    let protocolFees = await provider.connection.getTokenAccountBalance(protocolFeesVault);
    console.log('admin publick key', admin.publicKey)
    console.log('marketVault', marketVault)
    console.log('protocolFees', protocolFees)

    assert(market != null, "Market pda should exist");
    assert(Number(protocolFees.value.amount) > 0, "Protocol PDA should have accumulated fees");

    const adminAta = await getAssociatedTokenAddress(NATIVE_MINT, admin.publicKey, false);
    const adminAtaAccInfo = await provider.connection.getAccountInfo(adminAta);
    if (!adminAtaAccInfo) {
      console.log('Creating admin ata...', adminAta)
      const ix = createAssociatedTokenAccountInstruction(
        admin.publicKey, 
        adminAta,             
        admin.publicKey,
        NATIVE_MINT
      );

      const tx = new Transaction().add(ix);
      const sig = await provider.sendAndConfirm(tx, [admin]);

      const latestBlockHash = await provider.connection.getLatestBlockhash();
      await provider.connection.confirmTransaction({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: sig,
      });

    let adminAtaBalance = await provider.connection.getTokenAccountBalance(adminAta);
    console.log('Admin ata balance', adminAtaBalance)

    }

    const res = await program.methods
      .closeMarket({ix: marketIx})
      .accountsStrict({
        admin: admin.publicKey,
        assetMint: NATIVE_MINT,
        lpMint: lpMintPDA,
        market: marketPDA,
        marketVault: marketVaultPDA,
        protocolFeesVault: protocolFeesVault,
        systemProgram: SYSTEM_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        adminAssetAta: adminAta
      })
      .signers([admin])
      .rpc();

      console.log('Close ix signature: ', res)

    const marketPDAInfo = await provider.connection.getAccountInfo(marketPDA);
    const marketVaultPDAInfo = await provider.connection.getAccountInfo(marketVaultPDA);
    const protocolFeesVaultInfo = await provider.connection.getAccountInfo(protocolFeesVault);
    const adminAtaBalance = await provider.connection.getTokenAccountBalance(adminAta);
    const adminLamports = await provider.connection.getBalance(admin.publicKey);

    // marketVault = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    // protocolFees = await provider.connection.getTokenAccountBalance(protocolFeesVault);

    assert(marketPDAInfo == null, "Market PDA is not closed");
    assert(marketVaultPDAInfo == null, "Market Vault PDA is not closed");
    assert(protocolFeesVaultInfo == null, "Protocol fees PDA is not closed");
    console.log('marketPDAInfo', marketPDAInfo)
    console.log('marketVaultPDAInfo', marketVaultPDAInfo)
    console.log('protocolFeesVaultInfo', protocolFeesVaultInfo)
    console.log('adminAtaBalance', adminAtaBalance)
    console.log('adminLamports', adminLamports)
  })
});

// --- HELPER FUNCTIONS --- //
async function wrapSol(connection: anchor.web3.Connection, wallet: anchor.web3.Keypair, lamports: number): Promise<PublicKey> {
  const associatedTokenAccount = await getAssociatedTokenAddress(
      NATIVE_MINT,
      wallet.publicKey
  );

  const wrapTransaction = new Transaction().add(
      createAssociatedTokenAccountInstruction(
          wallet.publicKey,
          associatedTokenAccount,
          wallet.publicKey,
          NATIVE_MINT
      ),
      anchor.web3.SystemProgram.transfer({
          fromPubkey: wallet.publicKey,
          toPubkey: associatedTokenAccount,
          lamports: lamports,
      }),
      createSyncNativeInstruction(associatedTokenAccount)
  );
  await sendAndConfirmTransaction(connection, wrapTransaction, [wallet]);
  return associatedTokenAccount;
}
async function airdropToWallets(
  wallets: anchor.web3.Keypair[],
  amount: number,
  connection: anchor.web3.Connection
) {
  const latestBlockHash = await connection.getLatestBlockhash();

  await Promise.all(wallets.map(async (w) => {
    const airdropSignature = await connection.requestAirdrop(
      w.publicKey,
      amount
    );

    await connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropSignature,
    });
  }));
}
async function wrapwSolAccounts(
  wallets: anchor.web3.Keypair[],
  depositAmount: number,
  connection: anchor.web3.Connection
): Promise<PublicKey[]> {

  const wsolAccs = await Promise.all(wallets.map(async w => {
    const wSolAcc = await wrapSol(connection, w, depositAmount);
    return wSolAcc;
  }));

  return wsolAccs;
}

function calcWithdrawAmountFromLpShares(
  lpTokensToBurn: anchor.BN,
  lpMinted: anchor.BN,
  premiums: anchor.BN,
  reserveSupply: anchor.BN,
  committedReserve: anchor.BN,
  
) {
  if (lpTokensToBurn <= new anchor.BN(0)) throw new Error("InvalidAmount");

  if (lpMinted < lpTokensToBurn) {
    throw new Error("InsufficientShares");
  }

  const SCALE = new anchor.BN(1000000000);

  // % ownership = lp_to_burn / lp_total
  const ownershipRatio = lpTokensToBurn.mul(SCALE).div(lpMinted);

  const marketTVL = reserveSupply.add(premiums);
  if (marketTVL <= new anchor.BN(0)) throw new Error("InvalidState");

  // expected proportional withdrawal
  const potentialWithdrawAmount = ownershipRatio.mul(marketTVL).div(SCALE);

  const uncommittedReserve = reserveSupply.sub(committedReserve);
  const maxWithdrawable = uncommittedReserve.add(premiums);

  const withdrawableAmount = potentialWithdrawAmount <= maxWithdrawable
    ? potentialWithdrawAmount
    : maxWithdrawable;

  if (withdrawableAmount < new anchor.BN(1)) throw new Error("CannotWithdraw");

  const actualLpTokensToBurn = withdrawableAmount < potentialWithdrawAmount
    ? withdrawableAmount.mul(lpMinted).div(marketTVL)
    : lpTokensToBurn;

  if (actualLpTokensToBurn <= new anchor.BN(0)) throw new Error("InvalidAmount");

  return {
    withdrawableAmount,
    actualLpTokensToBurn,
  };
}