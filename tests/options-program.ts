import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OptionsProgram } from "../target/types/options_program";
import { expect } from "chai";

describe("options-program", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.options_program as Program<OptionsProgram>;
  const wallet = provider.wallet as anchor.Wallet;

  it("Create account", async () => {
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

    console.log("Fetched UserAccount Data:", accountData);

    // Validate the initial state
    console.log("Balance:", accountData.balance.toString());
    console.log("Options Length:", accountData.options.length);
    console.log("First Option (if exists):", accountData.options[0]);

    // Assert expected values (if using Jest)
    expect(accountData.balance.toString()).eq("0");
    expect(accountData.options.length).eq(32);
  });
});