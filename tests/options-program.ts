import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OptionsProgram } from "../target/types/options_program";
import { LAMPORTS_PER_SOL, PublicKey, sendAndConfirmTransaction, Transaction } from '@solana/web3.js'
import { assert, expect } from "chai";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, createAssociatedTokenAccountInstruction, createSyncNativeInstruction, getAssociatedTokenAddress, NATIVE_MINT } from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";

function u16ToLEBytes(n: number): Buffer {
  const buf = Buffer.alloc(2);
  buf.writeUInt16LE(n, 0);
  return buf;
}

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

  const marketIx = 1;    
  console.log('programid: ', program.programId);
  const [marketPDA,] = await anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from('market'),
      Buffer.from(new Uint16Array([marketIx]).buffer)
      // new anchor.BN(marketIx).toBuffer("le", 2),
    ],
    program.programId
  );

  console.log('market PDA', marketPDA.toBase58());
  console.log('Market PDA seeds:', [
    Buffer.from('market'),
    Buffer.from(new Uint16Array([marketIx]).buffer)
    // new anchor.BN(marketIx).toBuffer("le", 2)
  ]);

  const [marketVaultPDA,] = await anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from('market_vault'),
      Buffer.from(new Uint16Array([marketIx]).buffer)
      // new anchor.BN(marketIx).toBuffer("le", 2),
    ],
    program.programId
  );

  const [lpMintPDA,] = await anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from('market_lp_mint'),
      Buffer.from(new Uint16Array([marketIx]).buffer)
      // new anchor.BN(marketIx).toBuffer("le", 2),
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
        new anchor.BN(2),
        'wSOL market',
        marketIx)
        .accountsStrict({
          market: marketPDA,
          marketVault: marketVaultPDA,
          lpMint: lpMintPDA,
          assetMint: NATIVE_MINT,
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
      new anchor.BN(2),
      'wSOL market',
      Number(marketIx))
      .accountsStrict({
        market: marketPDA,
        marketVault: marketVaultPDA,
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

  it("LPs deposit into pool", async () => {
    //Setup LP
    const liqProvider1 = anchor.web3.Keypair.generate();
    const liqProvider2 = anchor.web3.Keypair.generate();
    let latestBlockHash = await provider.connection.getLatestBlockhash();

    //lp1 aidrop
    const lp1AirdropSignature = await provider.connection.requestAirdrop(
      liqProvider1.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: lp1AirdropSignature,
    });

    //lp2 airdrop
    const lp2AirdropSignature = await provider.connection.requestAirdrop(
      liqProvider2.publicKey,
      10000 * LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: lp2AirdropSignature,
    });

    //wrap sol
    const lp1AssociatedTokenAcc = await wrapSol(provider.connection, liqProvider1, 5000 * LAMPORTS_PER_SOL);
    const lp2AssociatedTokenAcc = await wrapSol(provider.connection, liqProvider2, 5000 * LAMPORTS_PER_SOL);

    const lp1lpTokenAcc = await getAssociatedTokenAddress(
      lpMintPDA,
      liqProvider1.publicKey
    );

    await createAssociatedTokenAccount(
      provider.connection,
      liqProvider1,         
      lpMintPDA,            
      liqProvider1.publicKey 
    );

    const lp2lpTokenAcc = await getAssociatedTokenAddress(
      lpMintPDA,
      liqProvider2.publicKey
    );
    await createAssociatedTokenAccount(
      provider.connection,
      liqProvider2,         
      lpMintPDA,            
      liqProvider2.publicKey 
    );

    //LP1 deposits
    await program.methods
      .marketDeposit({amount: new anchor.BN(1000 * LAMPORTS_PER_SOL), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
      .accountsStrict({
        signer: liqProvider1.publicKey,
        userAssetAta: lp1AssociatedTokenAcc,
        userLpAta: lp1lpTokenAcc,
        market: marketPDA,
        marketVault: marketVaultPDA,
        lpMint: lpMintPDA,
        assetMint: NATIVE_MINT,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram:SYSTEM_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([liqProvider1])
      .rpc();

      const lp1AtaBalance = await provider.connection.getTokenAccountBalance(lp1lpTokenAcc);
      console.log('LP1 LP acc balance: ', lp1AtaBalance.value);
      assert.equal(Number(lp1AtaBalance.value.amount), 1000 * LAMPORTS_PER_SOL);

    //TODO: Need some premiums to accumulate 

    //LP2 deposits
    await program.methods
    .marketDeposit({amount: new anchor.BN(1000 * LAMPORTS_PER_SOL), minAmountOut: new anchor.BN(1), ix: Number(marketIx)})
    .accountsStrict({
      signer: liqProvider2.publicKey,
      userAssetAta: lp2AssociatedTokenAcc,
      userLpAta: lp2lpTokenAcc,
      market: marketPDA,
      marketVault: marketVaultPDA,
      lpMint: lpMintPDA,
      assetMint: NATIVE_MINT,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram:SYSTEM_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    })
    .signers([liqProvider2])
    .rpc();

    const lp2AtaBalance = await provider.connection.getTokenAccountBalance(lp2lpTokenAcc);
    console.log('LP2 LP token amount: ', lp2AtaBalance.value);
    assert.equal(Number(lp2AtaBalance.value.amount), 909_090_909_000);
  })

  it("Takers can create account", async () => {
    // Generate a new Keypair for the account
    const userAccount = anchor.web3.Keypair.generate();

    // Derive the PDA (Program Derived Address)
    const [userAccountPda, bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("account"), wallet.publicKey.toBuffer()],
      program.programId
    );

    console.log("User Account PDA:", userAccountPda.toBase58());

    // Send the transaction to create the account
    const tx = await program.methods
      .createAccount()
      .accountsStrict({
        signer: wallet.publicKey,
        account: userAccountPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([])
      .rpc();

    console.log("Transaction Signature:", tx);

    // Fetch the newly created account
    const accountData = await program.account.userAccount.fetch(userAccountPda);

    // console.log("Fetched UserAccount Data:", accountData);

    // Validate the initial state
    // console.log("Balance:", accountData.balance.toString());
    console.log("Options Length:", accountData.options.length);
    console.log("First Option (if exists):", accountData.options[0]);

    // Assert expected values (if using Jest)
    // expect(accountData.balance.toString()).eq("0");
    expect(accountData.options.length).eq(32);
  });
});