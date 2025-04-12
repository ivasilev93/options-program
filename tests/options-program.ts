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
  const marketIx = 1;    
  console.log('programid: ', program.programId);

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
        marketIx,
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
      8000) //Pyth SOL/USD feed
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

  it("Alice can deposit into pool", async () => {
    //Setup LPs - alice and bob
    const alice = anchor.web3.Keypair.generate();
    let latestBlockHash = await provider.connection.getLatestBlockhash();

    //alice aidrop
    const alice_airdropSignature = await provider.connection.requestAirdrop(
      alice.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: alice_airdropSignature,
    });

    //wrap sol
    const alice_wSolTokenAcc = await wrapSol(provider.connection, alice, 5000 * LAMPORTS_PER_SOL);

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

    //Alice deposits
    await program.methods
      .marketDeposit({amount: new anchor.BN(1000 * LAMPORTS_PER_SOL), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
      .accountsStrict({
        signer: alice.publicKey,
        userAssetAta: alice_wSolTokenAcc,
        userLpAta: alice_lpTokenAcc,
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

      const aliceAtaBalance = await provider.connection.getTokenAccountBalance(alice_lpTokenAcc);
      console.log('Alice LP acc balance: ', aliceAtaBalance.value);
      assert.equal(Number(aliceAtaBalance.value.amount), 1000 * LAMPORTS_PER_SOL);    

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
        volatility_bps: market.volatilityBps,
        price_feed: market.priceFeed,
        asset_decimals: market.assetDecimals,
      });
  })

  it("Takers can create account and buy option", async () => {
    // Generate a new Keypair for the account
    const john = anchor.web3.Keypair.generate();
    let latestBlockHash = await provider.connection.getLatestBlockhash();

    const john_ad_tx = await provider.connection.requestAirdrop(
      john.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: john_ad_tx,
    });

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

    //Buy option
    //wrap sol
    const john_wSolTokenAcc = await wrapSol(provider.connection, john, 5000 * LAMPORTS_PER_SOL);

    const tx = await program.methods.buy({
      marketIx: marketIx,
      option: { call: {} },
      quantity: new anchor.BN(10),
      expiryStamp: new anchor.BN(Math.floor(Date.now() / 1000) + 5 * 60),
      strikePriceUsd: new anchor.BN(140000000)
    }).accountsStrict({
      account: john_taker_accountPda,
      assetMint: NATIVE_MINT,
      market: marketPDA,
      marketVault: marketVaultPDA,
      protocolFeesVault: protocolFeesVault,
      tokenProgram: TOKEN_PROGRAM_ID,
      userTokenAcc: john_wSolTokenAcc,
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
        volatility_bps: market.volatilityBps,
        price_feed: market.priceFeed,
        asset_decimals: market.assetDecimals,
      });

  });

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
});