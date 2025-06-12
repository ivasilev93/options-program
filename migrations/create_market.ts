import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OptionsProgram } from "../target/types/options_program";
import { Keypair, PublicKey, Connection, SystemProgram, Transaction, ComputeBudgetProgram } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";


/*In order to run this ix, ANCHOR_PROVIDER_URL and ANCHOR_WALLET must be set.
Args:
<pyth-price-feed> - pyth oracle price feed 
<protocol-fees-bps> - protocol basis points  // 1bps = 0.01%
<market-ix> - market index
<asset-mint> - asset mint

Full command example:

ANCHOR_PROVIDER_URL=http://127.0.0.1:8899 ANCHOR_WALLET=$HOME/.config/solana/id.json npx ts-node migrations/create_market.ts <pyth-price-feed> <protocol-fees-bps> <market-ix> <asset-mint>
*/

// wSOL market - ANCHOR_PROVIDER_URL=http://127.0.0.1:8899 ANCHOR_WALLET=$HOME/.config/solana/id.json npx ts-node migrations/create_market.ts 0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d 50 1 So11111111111111111111111111111111111111112

//JUP market - ANCHOR_PROVIDER_URL=http://127.0.0.1:8899 ANCHOR_WALLET=$HOME/.config/solana/id.json npx ts-node migrations/create_market.ts 0x0a0408d619e9380abad35060f9192039ed5042fa6f82301d0e48bb52be830996 100 2 JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN

async function detectTokenProgram(connection: Connection, mint: PublicKey): Promise<PublicKey> {
  const mintAccount = await connection.getAccountInfo(mint);
  if (!mintAccount) throw new Error("Mint not found");
  console.log('Mint owner info: ', mintAccount.owner.toBase58())

  const owner = mintAccount.owner;
  if (owner.equals(TOKEN_PROGRAM_ID)) return TOKEN_PROGRAM_ID;
  if (owner.equals(TOKEN_2022_PROGRAM_ID)) return TOKEN_2022_PROGRAM_ID;

  throw new Error(`Unknown token program: ${owner.toBase58()}`);
}

(async () => {
    console.log("Create market script started");
  
    const provider = anchor.AnchorProvider.env();
    // console.log('provider', provider)
    anchor.setProvider(provider);
  
    const program = anchor.workspace.OptionsProgram as Program<OptionsProgram>;
  
    const args = process.argv.slice(2);
    if (args.length < 4) {
      console.error("Usage: ts-node create_market.ts <price_feed_address> <fee_basis_points>");
      process.exit(1);
    }
  
    console.log("Arguments received: ", args[0], args[1], args[2], args[3]);
    const pythFeed = args[0];
    const protocolFeeBps = Number(args[1]);
    const marketIx = Number(args[2]);
    const assetMint = args[3];

    console.log('Parsed values: ', pythFeed, protocolFeeBps, marketIx, assetMint);

    const admin = provider.wallet as anchor.Wallet;
    const token_program_id = await detectTokenProgram(provider.connection, new PublicKey(assetMint));
  
   //---Derive PDAs---//
   //Could extreact common account derivation logic into utils class...too much code duplication
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

     const transaction = new Transaction();    
     // Add priority fee instructions
     transaction.add(
       ComputeBudgetProgram.setComputeUnitPrice({ 
         microLamports: 10 // Higher priority for admin operations
       })
     );

    const createMarketIx = await program.methods.createMarket({
          fee: new anchor.BN(protocolFeeBps),
          name: '--', //todo remove later
          ix: marketIx,
          priceFeed: pythFeed,
          hour1VolatilityBps: 8000,
          hour4VolatilityBps: 9000,
          day1VolatilityBps: 8000,
          day3VolatilityBps: 7500,
          weekVolatilityBps: 7000,
        }) 
          .accountsStrict({
            market: marketPDA,
            marketVault: marketVaultPDA,
            protocolFeesVault: protocolFeesVault,
            lpMint: lpMintPDA,
            assetMint: new PublicKey(assetMint),
            tokenProgram: token_program_id,
            signer: admin.publicKey,
            systemProgram: SYSTEM_PROGRAM_ID
        })
        .signers([admin.payer])
        .instruction();

        transaction.add(createMarketIx);
      
        const createMarketSignature = await provider.sendAndConfirm(transaction, [admin.payer]);
        // const latestBlockHash = await provider.connection.getLatestBlockhash();
        // await provider.connection.confirmTransaction({
        //   blockhash: latestBlockHash.blockhash,
        //   lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        //   signature: createMarketSignature,
        // });
  
        console.log('Market created: ', createMarketSignature);
  })();

