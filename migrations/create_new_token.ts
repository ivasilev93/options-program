import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OptionsProgram } from "../target/types/options_program";
import { Keypair, PublicKey, Connection, SystemProgram } from "@solana/web3.js";
import { createInitializeMintInstruction, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";


/*In order to run this ix, ANCHOR_PROVIDER_URL and ANCHOR_WALLET must be set.


Full command example:

ANCHOR_PROVIDER_URL=http://127.0.0.1:8899 ANCHOR_WALLET=$HOME/.config/solana/id.json npx ts-node migrations/create_new_token.ts 
*/


(async () => {
    try
    {
      console.log("Create new token scrip started...");
     
    const provider = anchor.AnchorProvider.env();
    // console.log('provider', provider)
    anchor.setProvider(provider);
  
    // const program = anchor.workspace.OptionsProgram as Program<OptionsProgram>;
  
    // const args = process.argv.slice(2);
    // if (args.length < 2) {
    //   console.error("Usage: ts-node create_market.ts <price_feed_address> <fee_basis_points>");
    //   process.exit(1);
    // }
  
    const decimals = 6; //Like JUP token
    const mintKeypair = anchor.web3.Keypair.generate();
    console.log('mint keypair: ', mintKeypair.publicKey.toBase58())

    const tx = new anchor.web3.Transaction();

    // Create mint account and initialize mint
    const lamportsForMint = await provider.connection.getMinimumBalanceForRentExemption(82);
    tx.add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: provider.wallet.publicKey,
        newAccountPubkey: mintKeypair.publicKey,
        space: 82,
        lamports: lamportsForMint,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        mintKeypair.publicKey,
        decimals,
        provider.wallet.publicKey,
        null, // freeze authority
        TOKEN_2022_PROGRAM_ID
      )
    );

    console.log('send');

    const createMintSign = await provider.sendAndConfirm(tx, [mintKeypair]);
    console.log('Mint created signature: ', createMintSign);
    console.log(" Mint created:", mintKeypair.publicKey.toBase58());

    // Create ATA for '9ffQciKRaK2ZiQMQ1NMBiBceqDi2iUucoYcqW9MyHX9L'
    const ata = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mintKeypair.publicKey,
      new PublicKey('9ffQciKRaK2ZiQMQ1NMBiBceqDi2iUucoYcqW9MyHX9L'),
      true,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    console.log("✅ ATA created:", ata.address.toBase58());

    // Mint test tokens to ATA
    await mintTo(
      provider.connection,
      provider.wallet.payer,
      mintKeypair.publicKey,
      ata.address,
      provider.wallet.publicKey,
      1_000_000_000, // 1000 tokens (with 6 decimals)
      [],
      null,
      TOKEN_2022_PROGRAM_ID
    );

     // Create ATA for 'D8Rua4Vc7GC861wpEew9PRo2T2KdAUtVvrSpFTKPBMBX'
     const ata2 = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mintKeypair.publicKey,
      new PublicKey('D8Rua4Vc7GC861wpEew9PRo2T2KdAUtVvrSpFTKPBMBX'),
      true,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    console.log("✅ ATA2 created:", ata2.address.toBase58());

    // Mint test tokens to ATA
    await mintTo(
      provider.connection,
      provider.wallet.payer,
      mintKeypair.publicKey,
      ata2.address,
      provider.wallet.publicKey,
      1_000_000_000, // 1000 tokens (with 6 decimals)
      [],
      null,
      TOKEN_2022_PROGRAM_ID
    );

     // Create ATA for '8T5U9PFa5bBz4vhW5DEHM8NFJrT5XcFJGT6ZZ2TceZ7q'
     const ata3 = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mintKeypair.publicKey,
      new PublicKey('8T5U9PFa5bBz4vhW5DEHM8NFJrT5XcFJGT6ZZ2TceZ7q'),
      true,
      undefined,
      undefined,
      TOKEN_2022_PROGRAM_ID
    );

    console.log("✅ ATA3 created:", ata3.address.toBase58());

    // Mint test tokens to ATA
    await mintTo(
      provider.connection,
      provider.wallet.payer,
      mintKeypair.publicKey,
      ata3.address,
      provider.wallet.publicKey,
      1_000_000_000, // 1000 tokens (with 6 decimals)
      [],
      null,
      TOKEN_2022_PROGRAM_ID
    );


    const bal = await provider.connection.getTokenAccountBalance(ata.address);
    console.log('balance: ', bal)

  } catch (err) {
    console.log('err: ', err)
  }

    
  })();

