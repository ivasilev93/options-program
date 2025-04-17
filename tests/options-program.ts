import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OptionsProgram } from "../target/types/options_program";
import { AccountInfo, Connection, LAMPORTS_PER_SOL, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js'
import { assert, expect } from "chai";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, createAssociatedTokenAccountInstruction, createSyncNativeInstruction, getAssociatedTokenAddress, NATIVE_MINT } from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { PythSolanaReceiver } from "@pythnetwork/pyth-solana-receiver";

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

describe("options-program", async () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.options_program as Program<OptionsProgram>;
  const wallet = provider.wallet as anchor.Wallet;
  console.log('Using Local Wallet: ', wallet.publicKey); 

  const SOL_USD_PRICE_FEED_ID = '0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d';
  const marketIx = 10;    
  console.log('programid: ', program.programId);

  const FIVE_MINS_FROM_NOW = new anchor.BN(Math.floor(Date.now() / 1000) + 5 * 60);

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

  console.log('EEE: ', marketPDA, marketVaultPDA, lpMintPDA, protocolFeesVault)

  it("Non-admin cannot create new market", async () => {
    const nonAdmin = anchor.web3.Keypair.generate();
    const airdropSignature = await provider.connection.requestAirdrop(
      nonAdmin.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    let latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropSignature,
    });

    try {
      const createMarketSignature = await program.methods.createMarket(
        new anchor.BN(50),
        'wSOL market',
        2222,
        SOL_USD_PRICE_FEED_ID,
        8000) //Pyth SOL/USD feed
        .accountsStrict({
          market: marketPDA,
          marketVault: marketVaultPDA,
          lpMint: lpMintPDA,
          assetMint: NATIVE_MINT,
          protocolFeesVault: protocolFeesVault,
          tokenProgram: TOKEN_PROGRAM_ID,
          signer: nonAdmin.publicKey,
          systemProgram: SYSTEM_PROGRAM_ID
      })
      .signers([nonAdmin])
      .rpc();
  
      latestBlockHash = await provider.connection.getLatestBlockhash();
      await provider.connection.confirmTransaction({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: createMarketSignature,
      });
    } catch (err) {
      assert.strictEqual(err.error.errorCode.code, "Unauthorized");
      assert.strictEqual(
        err.error.errorMessage,
        "Unauthorized",
      );
    }
  })

  it("Admin can create new market", async () => {

    const createMarketSignature = await program.methods.createMarket(
      new anchor.BN(50),
      'wSOL market',
      marketIx,
      SOL_USD_PRICE_FEED_ID,
      8000) 
      .accountsStrict({
        market: marketPDA,
        marketVault: marketVaultPDA,
        protocolFeesVault: protocolFeesVault,
        lpMint: lpMintPDA,
        assetMint: NATIVE_MINT,
        tokenProgram: TOKEN_PROGRAM_ID,
        signer: wallet.publicKey,
        systemProgram: SYSTEM_PROGRAM_ID
    })
    .rpc();
  
    const latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: createMarketSignature,
    });

    console.log('Admin create market tx: ', createMarketSignature);
  })

  // it("Alice can deposit into pool", async () => {
  //   //Setup LPs - alice and bob
  //   const alice = anchor.web3.Keypair.generate();
  //   let latestBlockHash = await provider.connection.getLatestBlockhash();

  //   //alice aidrop
  //   const alice_airdropSignature = await provider.connection.requestAirdrop(
  //     alice.publicKey,
  //     10000 * LAMPORTS_PER_SOL
  //   );
  //   await provider.connection.confirmTransaction({
  //     blockhash: latestBlockHash.blockhash,
  //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
  //     signature: alice_airdropSignature,
  //   });

  //   //wrap sol
  //   const alice_wSolTokenAcc = await wrapSol(provider.connection, alice, 5000 * LAMPORTS_PER_SOL);

  //   const alice_lpTokenAcc = await getAssociatedTokenAddress(
  //     lpMintPDA,
  //     alice.publicKey
  //   );

  //   await createAssociatedTokenAccount(
  //     provider.connection,
  //     alice,         
  //     lpMintPDA,            
  //     alice.publicKey 
  //   );

  //   //Alice deposits
  //   await program.methods
  //     .marketDeposit({amount: new anchor.BN(1000 * LAMPORTS_PER_SOL), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
  //     .accountsStrict({
  //       signer: alice.publicKey,
  //       userAssetAta: alice_wSolTokenAcc,
  //       userLpAta: alice_lpTokenAcc,
  //       market: marketPDA,
  //       marketVault: marketVaultPDA,
  //       lpMint: lpMintPDA,
  //       assetMint: NATIVE_MINT,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       systemProgram:SYSTEM_PROGRAM_ID,
  //       associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //     })
  //     .signers([alice])
  //     .rpc();

  //     const aliceAtaBalance = await provider.connection.getTokenAccountBalance(alice_lpTokenAcc);
  //     console.log('Alice LP acc balance: ', aliceAtaBalance.value);
  //     assert.equal(Number(aliceAtaBalance.value.amount), 1000 * LAMPORTS_PER_SOL);    

  //     const marketVaultBalance = await provider.connection.getTokenAccountBalance(marketVaultPDA);
  //     console.log('Market vault balance: ', marketVaultBalance.value);

  //     const market = await program.account.market.fetch(marketPDA);
  //     console.log("Market Account:");
  //     console.log({
  //       id: market.id,
  //       name: market.name,
  //       fee_bps: market.feeBps.toString(),
  //       bump: market.bump,
  //       reserve_supply: market.reserveSupply.toString(),
  //       committed_reserve: market.committedReserve.toString(),
  //       premiums: market.premiums.toString(),
  //       lp_minted: market.lpMinted.toString(),
  //       volatility_bps: market.volatilityBps,
  //       price_feed: market.priceFeed,
  //       asset_decimals: market.assetDecimals,
  //     });
  // })

  // it("Takers can create account and buy option", async () => {
  //   // Generate a new Keypair for the account
  //   const john = anchor.web3.Keypair.generate();
  //   let latestBlockHash = await provider.connection.getLatestBlockhash();

  //   const john_ad_tx = await provider.connection.requestAirdrop(
  //     john.publicKey,
  //     10000 * LAMPORTS_PER_SOL
  //   );
  //   await provider.connection.confirmTransaction({
  //     blockhash: latestBlockHash.blockhash,
  //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
  //     signature: john_ad_tx,
  //   });

  //   // Derive the PDA (Program Derived Address)
  //   const [john_taker_accountPda, bump] = await anchor.web3.PublicKey.findProgramAddress(
  //     [Buffer.from("account"), john.publicKey.toBuffer()],
  //     program.programId
  //   );

  //   console.log("John (taker) acc PDA:", john_taker_accountPda.toBase58());

  //   // Send the transaction to create the account
  //   await program.methods
  //     .createAccount()
  //     .accountsStrict({
  //       signer: john.publicKey,
  //       account: john_taker_accountPda,
  //       systemProgram: anchor.web3.SystemProgram.programId,
  //     })
  //     .signers([john])
  //     .rpc();

  //   //Buy option
  //   //wrap sol
  //   const john_wSolTokenAcc = await wrapSol(provider.connection, john, 5000 * LAMPORTS_PER_SOL);

  //   const tx = await program.methods.buy({
  //     marketIx: marketIx,
  //     option: { call: {} },
  //     quantity: new anchor.BN(10),
  //     expiryStamp: FIVE_MINS_FROM_NOW,
  //     strikePriceUsd: new anchor.BN(140000000)
  //   }).accountsStrict({
  //     account: john_taker_accountPda,
  //     assetMint: NATIVE_MINT,
  //     market: marketPDA,
  //     marketVault: marketVaultPDA,
  //     protocolFeesVault: protocolFeesVault,
  //     tokenProgram: TOKEN_PROGRAM_ID,
  //     userTokenAcc: john_wSolTokenAcc,
  //     priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
  //     signer: john.publicKey
  //   })
  //   .signers([john])
  //   .rpc();

  //   console.log("John buys option Tx Signature:", tx);

  //   const john_accountData = await program.account.userAccount.fetch(john_taker_accountPda);
  //   console.log("First Option:");
  //   const opt = john_accountData.options[0];
  //   console.log({
  //     marketIx: opt.marketIx,
  //     optionType: opt.optionType,
  //     strikePrice: opt.strikePrice.toString(),
  //     expiry: new Date(opt.expiry.toNumber() * 1000).toISOString(),
  //     premium: opt.premium.toString(),
  //   });

  //   const marketVaultBalance = await provider.connection.getTokenAccountBalance(marketVaultPDA);
  //   console.log('Market vault balance: ', marketVaultBalance.value);

  //   const market = await program.account.market.fetch(marketPDA);
  //   console.log("Market Account:");
  //     console.log({
  //       id: market.id,
  //       name: market.name,
  //       fee_bps: market.feeBps.toString(),
  //       bump: market.bump,
  //       reserve_supply: market.reserveSupply.toString(),
  //       committed_reserve: market.committedReserve.toString(),
  //       premiums: market.premiums.toString(),
  //       lp_minted: market.lpMinted.toString(),
  //       volatility_bps: market.volatilityBps,
  //       price_feed: market.priceFeed,
  //       asset_decimals: market.assetDecimals,
  //     });

  // });

  // it("Bob deposits after premium are accumulated and should receive less shares than Alice", async () => {
  //   const bob = anchor.web3.Keypair.generate();
  //   let latestBlockHash = await provider.connection.getLatestBlockhash();

  //    //bob airdrop
  //    const bob_AirdropSignature = await provider.connection.requestAirdrop(
  //     bob.publicKey,
  //     10000 * LAMPORTS_PER_SOL
  //   );
  //   await provider.connection.confirmTransaction({
  //     blockhash: latestBlockHash.blockhash,
  //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
  //     signature: bob_AirdropSignature,
  //   });

  //   //wrap wSol
  //   const bob_wSolTokenAcc = await wrapSol(provider.connection, bob, 5000 * LAMPORTS_PER_SOL);

  //   const bob_lpTokenAcc = await getAssociatedTokenAddress(
  //     lpMintPDA,
  //     bob.publicKey
  //   );
  //   await createAssociatedTokenAccount(
  //     provider.connection,
  //     bob,         
  //     lpMintPDA,            
  //     bob.publicKey 
  //   );

  //   //LP2 deposits
  //   await program.methods
  //   .marketDeposit({amount: new anchor.BN(1000 * LAMPORTS_PER_SOL), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
  //   .accountsStrict({
  //     signer: bob.publicKey,
  //     userAssetAta: bob_wSolTokenAcc,
  //     userLpAta: bob_lpTokenAcc,
  //     market: marketPDA,
  //     marketVault: marketVaultPDA,
  //     lpMint: lpMintPDA,
  //     assetMint: NATIVE_MINT,
  //     tokenProgram: TOKEN_PROGRAM_ID,
  //     systemProgram:SYSTEM_PROGRAM_ID,
  //     associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //   })
  //   .signers([bob])
  //   .rpc();

  //   const lp2AtaBalance = await provider.connection.getTokenAccountBalance(bob_lpTokenAcc);
  //   console.log('LP2 LP token amount: ', lp2AtaBalance.value);
  //   assert.equal(Number(lp2AtaBalance.value.amount), 909_090_909_000);
  // });

  it("Check proper balances after deposit, LP token minting, and premium collection", async () => {
    // Setup - create market and user accounts
    const alice = anchor.web3.Keypair.generate();
    const bob = anchor.web3.Keypair.generate();
    
    // Airdrop SOL to alice and bob
    let latestBlockHash = await provider.connection.getLatestBlockhash();
    const alice_airdrop = await provider.connection.requestAirdrop(
      alice.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: alice_airdrop,
    });
    
    const bob_airdrop = await provider.connection.requestAirdrop(
      bob.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: bob_airdrop,
    });
    
    // Wrap SOL for both users
    const alice_wSolTokenAcc = await wrapSol(provider.connection, alice, 6000 * LAMPORTS_PER_SOL);
    const bob_wSolTokenAcc = await wrapSol(provider.connection, bob, 6000 * LAMPORTS_PER_SOL);
    
    // Create LP token accounts for both users
    const alice_lpTokenAcc = await getAssociatedTokenAddress(
      lpMintPDA,
      alice.publicKey
    );
    
    await createAssociatedTokenAccount(
      provider.connection,
      alice,         
      lpMintPDA,            
      alice.publicKey 
    );
    
    const bob_lpTokenAcc = await getAssociatedTokenAddress(
      lpMintPDA,
      bob.publicKey
    );
    
    await createAssociatedTokenAccount(
      provider.connection,
      bob,         
      lpMintPDA,            
      bob.publicKey 
    );
    
    // Record initial state
    const initialMarketVaultBalance = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    console.log('Initial market vault balance:', initialMarketVaultBalance.value.amount);
    
    // 1. Alice deposits 1000 SOL
    const aliceDepositAmount = 5000 * LAMPORTS_PER_SOL;
    await program.methods
      .marketDeposit({amount: new anchor.BN(aliceDepositAmount), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
      .accountsStrict({
        signer: alice.publicKey,
        userAssetAta: alice_wSolTokenAcc,
        userLpAta: alice_lpTokenAcc,
        market: marketPDA,
        marketVault: marketVaultPDA,
        lpMint: lpMintPDA,
        assetMint: NATIVE_MINT,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([alice])
      .rpc();
      
    // Verify Alice's LP tokens equals her deposit (first depositor)
    const aliceLpBalance = await provider.connection.getTokenAccountBalance(alice_lpTokenAcc);
    console.log('Alice LP tokens after deposit:', aliceLpBalance.value.amount);
    assert.equal(Number(aliceLpBalance.value.amount), aliceDepositAmount);
    
    // Verify market vault balance increased by Alice's deposit
    const marketVaultBalanceAfterAlice = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    console.log('Market vault after Alice deposit:', marketVaultBalanceAfterAlice.value.amount);
    assert.equal(
      Number(marketVaultBalanceAfterAlice.value.amount) - Number(initialMarketVaultBalance.value.amount),
      aliceDepositAmount
    );
    
    // Verify market account state
    let market = await program.account.market.fetch(marketPDA);
    console.log("Market state after Alice deposit:", {
      reserve_supply: market.reserveSupply.toString(),
      lp_minted: market.lpMinted.toString()
    });
    assert.equal(market.reserveSupply.toString(), aliceDepositAmount.toString());
    assert.equal(market.lpMinted.toString(), aliceDepositAmount.toString());
    
    // 2. Create a user account for a trader and buy an option
    const trader = anchor.web3.Keypair.generate();
    const trader_airdrop = await provider.connection.requestAirdrop(
      trader.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: trader_airdrop,
    });
    
    // Wrap SOL for trader
    const trader_wSolTokenAcc = await wrapSol(provider.connection, trader, 6000 * LAMPORTS_PER_SOL);
    
    // Create trader account
    const [trader_accountPda] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("account"), trader.publicKey.toBuffer()],
      program.programId
    );
    
    await program.methods
      .createAccount()
      .accountsStrict({
        signer: trader.publicKey,
        account: trader_accountPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([trader])
      .rpc();
    
    // Record market state before option purchase
    const marketBeforeOption = await program.account.market.fetch(marketPDA);
    const marketVaultBeforeOption = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    const protocolFeesBeforeOption = await provider.connection.getTokenAccountBalance(protocolFeesVault);
    
    // 3. Trader buys an option
    const optionQuantity = 400;
    const strikePrice = 140000000; // $140
    
    await program.methods.buy({
      marketIx: marketIx,
      option: { call: {} },
      quantity: new anchor.BN(optionQuantity),
      expiryStamp: FIVE_MINS_FROM_NOW,
      strikePriceUsd: new anchor.BN(strikePrice)
    }).accountsStrict({
      account: trader_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      protocolFeesVault: protocolFeesVault,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: trader_wSolTokenAcc,
      priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
      signer: trader.publicKey
    })
    .signers([trader])
    .rpc();
    
    // Get option details
    const traderAccount = await program.account.userAccount.fetch(trader_accountPda);
    const optionDetails = traderAccount.options[0];
    const premiumPaid = optionDetails.premium;
    
    console.log("Option purchased with premium:", premiumPaid.toString());
    
    // Verify market state after option purchase
    const marketAfterOption = await program.account.market.fetch(marketPDA);
    console.log("Market state after option purchase:", {
      reserve_supply: marketAfterOption.reserveSupply.toString(),
      committed_reserve: marketAfterOption.committedReserve.toString(),
      premiums: marketAfterOption.premiums.toString(),
      lp_minted: marketAfterOption.lpMinted.toString()
    });
    
    // Verify protocol fees
    const protocolFeesAfterOption = await provider.connection.getTokenAccountBalance(protocolFeesVault);
    const protocolFeeAmount = Number(protocolFeesAfterOption.value.amount) - Number(protocolFeesBeforeOption.value.amount);
    console.log("Protocol fees collected:", protocolFeeAmount);
    
    // Verify market vault balance changes
    const marketVaultAfterOption = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    const expectedVaultChange = Number(premiumPaid) - protocolFeeAmount;
    const actualVaultChange = Number(marketVaultAfterOption.value.amount) - Number(marketVaultBeforeOption.value.amount);
    
    console.log("Market vault change:", actualVaultChange);
    console.log("Expected market vault change:", expectedVaultChange);
    assert.approximately(actualVaultChange, expectedVaultChange, 5); // Allow small rounding errors
    
    // Verify premiums accounting
    const expectedPremiumIncrease = Number(premiumPaid) * (1 - Number(marketBeforeOption.feeBps) / 10000);
    const actualPremiumIncrease = Number(marketAfterOption.premiums) - Number(marketBeforeOption.premiums);
    
    console.log("Expected premium increase:", expectedPremiumIncrease);
    console.log("Actual premium increase:", actualPremiumIncrease);
    assert.approximately(actualPremiumIncrease, expectedPremiumIncrease, 5); // Allow small rounding errors
    
    // 4. Bob deposits after options were purchased (should get fewer LP tokens)
    const bobDepositAmount = 5000 * LAMPORTS_PER_SOL;
    await program.methods
      .marketDeposit({amount: new anchor.BN(bobDepositAmount), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
      .accountsStrict({
        signer: bob.publicKey,
        userAssetAta: bob_wSolTokenAcc,
        userLpAta: bob_lpTokenAcc,
        market: marketPDA,
        marketVault: marketVaultPDA,
        lpMint: lpMintPDA,
        assetMint: NATIVE_MINT,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([bob])
      .rpc();
    
    // Verify Bob's LP tokens (should be less than deposit due to premiums)
    const bobLpBalance = await provider.connection.getTokenAccountBalance(bob_lpTokenAcc);
    console.log('Bob LP tokens after deposit:', bobLpBalance.value.amount);
    
    // Bob should get fewer LP tokens than his deposit amount due to premiums in the pool
    assert.isBelow(Number(bobLpBalance.value.amount), bobDepositAmount);
    
    // Verify total LP tokens outstanding matches the expected amount
    const totalLpExpected = Number(aliceLpBalance.value.amount) + Number(bobLpBalance.value.amount);
    const marketFinal = await program.account.market.fetch(marketPDA);
    
    console.log("Total LP tokens expected:", totalLpExpected);
    console.log("Total LP tokens in market:", marketFinal.lpMinted.toString());
    assert.equal(marketFinal.lpMinted.toString(), totalLpExpected.toString());
    
    // Verify total assets match expectations
    const marketVaultFinal = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    const totalAssetsInVault = Number(marketVaultFinal.value.amount);
    const expectedTotalAssets = aliceDepositAmount + bobDepositAmount + actualPremiumIncrease;
    
    console.log("Total assets in vault:", totalAssetsInVault);
    console.log("Expected total assets:", expectedTotalAssets);
    assert.approximately(totalAssetsInVault, expectedTotalAssets, 5); // Allow small rounding errors
  });
  
  it("Verify proper accounting when multiple options are purchased", async () => {
    // Setup user
    const trader = anchor.web3.Keypair.generate();
    const airdropSignature = await provider.connection.requestAirdrop(
      trader.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    let latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropSignature,
    });
    
    // Wrap SOL
    const trader_wSolTokenAcc = await wrapSol(provider.connection, trader, 5000 * LAMPORTS_PER_SOL);
    
    // Create user account
    const [trader_accountPda] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("account"), trader.publicKey.toBuffer()],
      program.programId
    );
    
    await program.methods
      .createAccount()
      .accountsStrict({
        signer: trader.publicKey,
        account: trader_accountPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([trader])
      .rpc();
    
    // Get initial state
    const initialMarket = await program.account.market.fetch(marketPDA);
    const initialCommittedReserve = initialMarket.committedReserve;
    const initialPremiums = initialMarket.premiums;
    
    // Buy first option (CALL option)
    const buy1tx = await program.methods.buy({
      marketIx: marketIx,
      option: { call: {} },
      quantity: new anchor.BN(5),
      expiryStamp: FIVE_MINS_FROM_NOW,
      strikePriceUsd: new anchor.BN(140000000) // $140
    }).accountsStrict({
      account: trader_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      protocolFeesVault: protocolFeesVault,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: trader_wSolTokenAcc,
      priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
      signer: trader.publicKey
    })
    .signers([trader])
    .rpc();

    console.log('First buy tx: ', buy1tx)
    
    // Get market state after first option
    const marketAfterFirstOption = await program.account.market.fetch(marketPDA);
    const firstOptionCommitted = marketAfterFirstOption.committedReserve.sub(initialCommittedReserve);
    const firstOptionPremium = marketAfterFirstOption.premiums.sub(initialPremiums);
    
    console.log("First option committed reserve:", firstOptionCommitted.toString());
    console.log("First option premium:", firstOptionPremium.toString());
    
    // Buy second option (PUT option)
    const buy2tx = await  await program.methods.buy({
      marketIx: marketIx,
      option: { call: {} },
      quantity: new anchor.BN(200),
      expiryStamp: FIVE_MINS_FROM_NOW,
      strikePriceUsd: new anchor.BN(150000000) // $120
    }).accountsStrict({
      account: trader_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      protocolFeesVault: protocolFeesVault,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: trader_wSolTokenAcc,
      priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
      signer: trader.publicKey
    })
    .signers([trader])
    .rpc();
    
    console.log('First 2 tx: ', buy2tx)

    // Get market state after second option
    const marketAfterSecondOption = await program.account.market.fetch(marketPDA);
    const secondOptionCommitted = marketAfterSecondOption.committedReserve.sub(marketAfterFirstOption.committedReserve);
    const secondOptionPremium = marketAfterSecondOption.premiums.sub(marketAfterFirstOption.premiums);
    
    console.log("Second option committed reserve:", secondOptionCommitted.toString());
    console.log("Second option premium:", secondOptionPremium.toString());
    
    // Verify user account has both options
    const traderAccount = await program.account.userAccount.fetch(trader_accountPda);
    assert.equal(traderAccount.options.filter(opt => opt.strikePrice.toNumber() > 0).length, 2);
    
    // Verify total committed reserve
    const totalCommittedIncrease = marketAfterSecondOption.committedReserve.sub(initialCommittedReserve);
    const expectedCommittedIncrease = firstOptionCommitted.add(secondOptionCommitted);
    
    console.log("Total committed reserve increase:", totalCommittedIncrease.toString());
    console.log("Expected committed reserve increase:", expectedCommittedIncrease.toString());
    assert.equal(totalCommittedIncrease.toString(), expectedCommittedIncrease.toString());
    
    // Verify total premiums
    const totalPremiumIncrease = marketAfterSecondOption.premiums.sub(initialPremiums);
    const expectedPremiumIncrease = firstOptionPremium.add(secondOptionPremium);
    
    console.log("Total premium increase:", totalPremiumIncrease.toString());
    console.log("Expected premium increase:", expectedPremiumIncrease.toString());
    assert.equal(totalPremiumIncrease.toString(), expectedPremiumIncrease.toString());
  });
  
  it("Reject buy order when insufficient collateral", async () => {
    // This test tries to buy options that would require more collateral than available
    
    // Setup user
    const trader = anchor.web3.Keypair.generate();
    const airdropSignature = await provider.connection.requestAirdrop(
      trader.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    let latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropSignature,
    });
    
    // Wrap SOL
    const trader_wSolTokenAcc = await wrapSol(provider.connection, trader, 5000 * LAMPORTS_PER_SOL);
    
    // Create user account
    const [trader_accountPda] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("account"), trader.publicKey.toBuffer()],
      program.programId
    );
    
    await program.methods
      .createAccount()
      .accountsStrict({
        signer: trader.publicKey,
        account: trader_accountPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([trader])
      .rpc();
    
    // Get initial market state
    const initialMarket = await program.account.market.fetch(marketPDA);
    // console.log("Initial market state:", {
    //   reserve_supply: initialMarket.reserveSupply.toString(),
    //   committed_reserve: initialMarket.committedReserve.toString(),
    //   available_collateral: initialMarket.reserveSupply.sub(initialMarket.committedReserve).toString()
    // });
    
    // Try to buy an option that would require more collateral than available
    // First, determine the current SOL price from Pyth
    // For test purposes, we'll use a very large quantity to ensure it exceeds available collateral
    const extremely_large_quantity = 100000000; 
    
    try {
      await program.methods.buy({
        marketIx: marketIx,
        option: { call: {} },
        quantity: new anchor.BN(extremely_large_quantity),
        expiryStamp: FIVE_MINS_FROM_NOW,
        strikePriceUsd: new anchor.BN(140000000) // $140
      }).accountsStrict({
        account: trader_accountPda,
        assetMint: NATIVE_MINT,
        market: marketPDA,
        marketVault: marketVaultPDA,
        protocolFeesVault: protocolFeesVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        userTokenAcc: trader_wSolTokenAcc,
        priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
        signer: trader.publicKey
      })
      .signers([trader])
      .rpc();
      
      // If we reach here, the transaction succeeded, which is unexpected
      assert.fail("Expected transaction to fail due to insufficient collateral");
    } catch (err) {
      // Verify the error is due to insufficient collateral
      console.log("Error message:", err.error?.errorMessage);
      assert.isTrue(
        err.error?.errorMessage?.includes("InsufficientColateral") || 
        err.error?.errorCode?.code === "InsufficientColateral"
      );
    }
    
    // Verify market state hasn't changed
    const finalMarket = await program.account.market.fetch(marketPDA);
    // console.log("Final market state:", {
    //   reserve_supply: finalMarket.reserveSupply.toString(),
    //   committed_reserve: finalMarket.committedReserve.toString()
    // });
    
    assert.equal(
      finalMarket.committedReserve.toString(), 
      initialMarket.committedReserve.toString(),
      "Committed reserve should not change after failed transaction"
    );
  });
  
  // it("Verify deposit/withdrawal accounting with multiple LPs and option premiums", async () => {
  //   // Set up multiple LPs
  //   const lp1 = anchor.web3.Keypair.generate();
  //   const lp2 = anchor.web3.Keypair.generate();
  //   const lp3 = anchor.web3.Keypair.generate();
    
  //   // Airdrop SOL to LPs
  //   let latestBlockHash = await provider.connection.getLatestBlockhash();
    
  //   for (const lp of [lp1, lp2, lp3]) {
  //     const airdropSignature = await provider.connection.requestAirdrop(
  //       lp.publicKey,
  //       10000 * LAMPORTS_PER_SOL
  //     );
  //     await provider.connection.confirmTransaction({
  //       blockhash: latestBlockHash.blockhash,
  //       lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
  //       signature: airdropSignature,
  //     });
  //   }
    
  //   // Wrap SOL and create LP token accounts
  //   const lp1_wSolTokenAcc = await wrapSol(provider.connection, lp1, 5000 * LAMPORTS_PER_SOL);
  //   const lp2_wSolTokenAcc = await wrapSol(provider.connection, lp2, 5000 * LAMPORTS_PER_SOL);
  //   const lp3_wSolTokenAcc = await wrapSol(provider.connection, lp3, 5000 * LAMPORTS_PER_SOL);
    
  //   const lp1_lpTokenAcc = await getAssociatedTokenAddress(lpMintPDA, lp1.publicKey);
  //   const lp2_lpTokenAcc = await getAssociatedTokenAddress(lpMintPDA, lp2.publicKey);
  //   const lp3_lpTokenAcc = await getAssociatedTokenAddress(lpMintPDA, lp3.publicKey);

  //   await createAssociatedTokenAccount(
  //     provider.connection,
  //     lp1,         
  //     lpMintPDA,            
  //     lp1.publicKey 
  //   );
  //   await createAssociatedTokenAccount(
  //     provider.connection,
  //     lp2,         
  //     lpMintPDA,            
  //     lp2.publicKey 
  //   );
  //   await createAssociatedTokenAccount(
  //     provider.connection,
  //     lp3,         
  //     lpMintPDA,            
  //     lp3.publicKey 
  //   );
    
  //   // for (const [lp, lpTokenAcc] of [[lp1, lp1_lpTokenAcc], [lp2, lp2_lpTokenAcc], [lp3, lp3_lpTokenAcc]]) {
  //   //   await createAssociatedTokenAccount(
  //   //     provider.connection,
  //   //     lp,         
  //   //     lpMintPDA,            
  //   //     lp.publicKey 
  //   //   );
  //   // }
    
  //   // Get initial market state
  //   const initialMarket = await program.account.market.fetch(marketPDA);
  //   const initialLpMinted = initialMarket.lpMinted;
  //   const initialReserveSupply = initialMarket.reserveSupply;
    
  //   console.log("Initial market state:", {
  //     lp_minted: initialLpMinted.toString(),
  //     reserve_supply: initialReserveSupply.toString()
  //   });
    
  //   // 1. LP1 deposits
  //   const lp1DepositAmount = 1000 * LAMPORTS_PER_SOL;
  //   await program.methods
  //     .marketDeposit({amount: new anchor.BN(lp1DepositAmount), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
  //     .accountsStrict({
  //       signer: lp1.publicKey,
  //       userAssetAta: lp1_wSolTokenAcc,
  //       userLpAta: lp1_lpTokenAcc,
  //       market: marketPDA,
  //       marketVault: marketVaultPDA,
  //       lpMint: lpMintPDA,
  //       assetMint: NATIVE_MINT,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       systemProgram: SYSTEM_PROGRAM_ID,
  //       associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //     })
  //     .signers([lp1])
  //     .rpc();
    
  //   // Get LP1's LP tokens
  //   const lp1LpBalance = await provider.connection.getTokenAccountBalance(lp1_lpTokenAcc);
  //   console.log('LP1 tokens received:', lp1LpBalance.value.amount);
    
  //   // Get market state after LP1 deposit
  //   const marketAfterLp1 = await program.account.market.fetch(marketPDA);
  //   console.log("Market after LP1 deposit:", {
  //     lp_minted: marketAfterLp1.lpMinted.toString(),
  //     reserve_supply: marketAfterLp1.reserveSupply.toString()
  //   });
    
  //   // Verify LP1's share is correctly calculated
  //   // If there were no premiums, LP1 should get tokens equal to deposit
  //   assert.equal(Number(lp1LpBalance.value.amount), lp1DepositAmount);
    
  //   // 2. Create user account for options trader
  //   const trader = anchor.web3.Keypair.generate();
  //   const traderAirdrop = await provider.connection.requestAirdrop(
  //     trader.publicKey,
  //     10000 * LAMPORTS_PER_SOL
  //   );
  //   await provider.connection.confirmTransaction({
  //     blockhash: latestBlockHash.blockhash,
  //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
  //     signature: traderAirdrop,
  //   });
    
  //   // Wrap SOL for trader
  //   const trader_wSolTokenAcc = await wrapSol(provider.connection, trader, 5000 * LAMPORTS_PER_SOL);
    
  //   // Create trader account
  //   const [trader_accountPda] = await anchor.web3.PublicKey.findProgramAddress(
  //     [Buffer.from("account"), trader.publicKey.toBuffer()],
  //     program.programId
  //   );
    
  //   await program.methods
  //     .createAccount()
  //     .accountsStrict({
  //       signer: trader.publicKey,
  //       account: trader_accountPda,
  //       systemProgram: anchor.web3.SystemProgram.programId,
  //     })
  //     .signers([trader])
  //     .rpc();
    
  //   // 3. Trader buys an option
  //   await program.methods.buy({
  //     marketIx: marketIx,
  //     option: { call: {} },
  //     quantity: new anchor.BN(20),
  //     expiryStamp: FIVE_MINS_FROM_NOW,
  //     strikePriceUsd: new anchor.BN(140000000) // $140
  //   }).accountsStrict({
  //     account: trader_accountPda,
  //     assetMint: NATIVE_MINT,
  //     market: marketPDA,
  //     marketVault: marketVaultPDA,
  //     protocolFeesVault: protocolFeesVault,
  //     tokenProgram: TOKEN_PROGRAM_ID,
  //     userTokenAcc: trader_wSolTokenAcc,
  //     priceUpdate: new PublicKey("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE"),
  //     signer: trader.publicKey
  //   })
  //   .signers([trader])
  //   .rpc();
    
  //   // Get market state after option purchase
  //   const marketAfterOption = await program.account.market.fetch(marketPDA);
  //   console.log("Market after option purchase:", {
  //     premiums: marketAfterOption.premiums.toString(),
  //     committed_reserve: marketAfterOption.committedReserve.toString()
  //   });
    
  //   const premiumCollected = marketAfterOption.premiums;
    
  //   // 4. LP2 deposits after premium collection
  //   const lp2DepositAmount = 1000 * LAMPORTS_PER_SOL;
  //   await program.methods
  //     .marketDeposit({amount: new anchor.BN(lp2DepositAmount), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
  //     .accountsStrict({
  //       signer: lp2.publicKey,
  //       userAssetAta: lp2_wSolTokenAcc,
  //       userLpAta: lp2_lpTokenAcc,
  //       market: marketPDA,
  //       marketVault: marketVaultPDA,
  //       lpMint: lpMintPDA,
  //       assetMint: NATIVE_MINT,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       systemProgram: SYSTEM_PROGRAM_ID,
  //       associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //     })
  //     .signers([lp2])
  //     .rpc();
    
  //   // Get LP2's LP tokens
  //   const lp2LpBalance = await provider.connection.getTokenAccountBalance(lp2_lpTokenAcc);
  //   console.log('LP2 tokens received:', lp2LpBalance.value.amount);
  // });
  
});