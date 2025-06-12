import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OptionsProgram } from "../target/types/options_program";
import { Keypair, PublicKey, Connection, SystemProgram, Transaction, ComputeBudgetProgram } from "@solana/web3.js";
import { createAssociatedTokenAccountInstruction, getAssociatedTokenAddress, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";


/*In order to run this ix, ANCHOR_PROVIDER_URL and ANCHOR_WALLET must be set.
Args:
<market-ix> - market index
<asset-mint> - asset mint

Full command example:
ANCHOR_PROVIDER_URL=http://127.0.0.1:8899 ANCHOR_WALLET=$HOME/.config/solana/id.json npx ts-node migrations/close_market.ts <market-ix> <asset-mint>
*/

async function detectTokenProgram(connection: Connection, mint: PublicKey): Promise<PublicKey> {
  const mintAccount = await connection.getAccountInfo(mint);
  if (!mintAccount) throw new Error("Mint not found");
  // console.log('Mint owner info: ', mintAccount.owner.toBase58())

  const owner = mintAccount.owner;
  if (owner.equals(TOKEN_PROGRAM_ID)) return TOKEN_PROGRAM_ID;
  if (owner.equals(TOKEN_2022_PROGRAM_ID)) return TOKEN_2022_PROGRAM_ID;

  throw new Error(`Unknown token program: ${owner.toBase58()}`);
}

(async () => {
    console.log("Close market script started...");
  
    const provider = anchor.AnchorProvider.env();
    // console.log('provider', provider)
    anchor.setProvider(provider);
  
    const program = anchor.workspace.OptionsProgram as Program<OptionsProgram>;
  
    const args = process.argv.slice(2);
    if (args.length < 2) {
      console.error("Error: Invalid arguments passed");
      process.exit(1);
    }
  
    console.log("Arguments received: ", args[0]);
    const marketIx = Number(args[0]);
    const assetMint = args[1];

    console.log(`Parsed values. Market index ${marketIx},  asset mint ${assetMint}`);

    const admin = provider.wallet as anchor.Wallet;
    const token_program_id = await detectTokenProgram(provider.connection, new PublicKey(assetMint));

    const adminAta = await getAssociatedTokenAddress(new PublicKey(assetMint), admin.publicKey, false, token_program_id);
    const adminAtaAccInfo = await provider.connection.getAccountInfo(adminAta);

    const tx = new Transaction();
    tx.add(
           ComputeBudgetProgram.setComputeUnitPrice({ 
             microLamports: 10 // Higher priority for admin operations
           })
         );

    let amdinAssetAmountBefore = 0;
    if (!adminAtaAccInfo) {
          console.log('Admin has no ATA. Creating admin ata...', adminAta)
          const create_ata_ix = createAssociatedTokenAccountInstruction(
            admin.publicKey, 
            adminAta,             
            admin.publicKey,
            new PublicKey(assetMint),
            token_program_id
          );            
        
          tx.add(create_ata_ix);
    } else {
      const adminBalanceInfo = await provider.connection.getTokenAccountBalance(adminAta);
      amdinAssetAmountBefore = adminBalanceInfo.value.uiAmount;
      console.log(`Admin balance amount before - ${amdinAssetAmountBefore} (${adminBalanceInfo.value.amount})`);
    }
  
   //---Derive PDAs---//
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

    let marketVault = await provider.connection.getTokenAccountBalance(marketVaultPDA);
    let protocolFees = await provider.connection.getTokenAccountBalance(protocolFeesVault);
    console.log('Amounts in vaults before close:')
    console.log(`Market vault - ${marketVault.value.uiAmount} (${marketVault.value.amount})`);
    console.log(`Protocol fees - ${protocolFees.value.uiAmount} (${protocolFees.value.amount})`);

    const closeMarketIx = await program.methods.closeMarket({
        ix: marketIx,
    }) 
    .accountsStrict({
        admin: admin.publicKey,
        assetMint: new PublicKey(assetMint),
        adminAssetAta: adminAta,
        lpMint: lpMintPDA,
        market: marketPDA,
        marketVault: marketVaultPDA,
        protocolFeesVault: protocolFeesVault,
        tokenProgram: token_program_id,
        systemProgram: SYSTEM_PROGRAM_ID,
    })
    .signers([admin.payer])
    .instruction();

    tx.add(closeMarketIx);
    const sig = await provider.sendAndConfirm(tx, [admin.payer]);    
      
    // const latestBlockHash = await provider.connection.getLatestBlockhash();
    // await provider.connection.confirmTransaction({
    //     blockhash: latestBlockHash.blockhash,
    //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
    //     signature: sig,
    // });
  
    console.log('Market closed. Transaction signature: ', sig);
    const adminBalanceInfo = await provider.connection.getTokenAccountBalance(adminAta);
    console.log(`Admin balance after closing market - ${adminBalanceInfo.value.uiAmount} (${adminBalanceInfo.value.amount})`);
  })();

