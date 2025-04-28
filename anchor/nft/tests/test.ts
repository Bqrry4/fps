import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Nft } from "../target/types/nft";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";

describe("mint-nft", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const wallet = provider.wallet as NodeWallet

  const program = anchor.workspace.Nft as Program<Nft>;

  const TOKEN_METADATA_PROGRAM_ID = new anchor.web3.PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

  const mintAuthority = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("authority")], program.programId)[0];

  const collectionKeypair = Keypair.generate();
  const collectionMint = collectionKeypair.publicKey;

  const mintKeypair = Keypair.generate();
  const mint = mintKeypair.publicKey;

  it("Create Collection NFT", async () => {
    console.log("\nCollection Mint Key: ", collectionMint.toBase58());

    const destination = getAssociatedTokenAddressSync(collectionMint, wallet.publicKey);
    console.log("Destination ATA = ", destination.toBase58());

    const tx = await program.methods.createCollection(
      "Collection NFT",
      "COLNFT",
    ).accounts({
      creator: wallet.publicKey,
      mint: collectionMint,
    })
      .signers([collectionKeypair])
      .rpc({
        skipPreflight: true,
      });
    console.log("\nCollection NFT minted: TxID - ", tx);
  })

  it("Create NFT", async () => {
    console.log("\nMint", mint.toBase58());

    const tx = await program.methods.createNft(
      "My NFT",
      "MNFT",
      "https://example.com/metadata.json",
      new anchor.BN(3)
    )
      .accounts({
        creator: wallet.publicKey,
        mint,
        collectionMint,
      })
      .signers([mintKeypair])
      .rpc({
        skipPreflight: true,
      });

    console.log("\nNFT Minted! Your transaction signature", tx);
  });

  it("Buy NFT", async () => {

    const editionMintKeypair = Keypair.generate();
    const editionMint        = editionMintKeypair.publicKey;

    console.log("\nEdition Mint Key: ", editionMint.toBase58());
    console.log("\nEdition Mint Key: ", mint.toBase58());

    const editionMarkerIx  = Math.floor(6 / 248);
    const [editionMarker]  = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        TOKEN_METADATA_PROGRAM_ID.toBuffer(),
        mint.toBuffer(),
        Buffer.from("edition"),
        Buffer.from(new anchor.BN(editionMarkerIx).toArray("le", 8)),
      ],
      TOKEN_METADATA_PROGRAM_ID
    );
    console.log("\nEdition Mint Key: ", editionMarker.toBase58());

    const tx = await program.methods.buyNft()
    .accounts({
      buyer: wallet.publicKey,
      masterMint: mint,
      newMint: editionMint,
      editionMarker,
    })
    .signers([editionMintKeypair, wallet.payer]).transaction();
    tx.feePayer = wallet.publicKey;
    const simulationResult = await provider.connection.simulateTransaction(tx);
    console.log('Simulation logs:', simulationResult.value.logs);
    
    try {
    const tx = await program.methods.buyNft()
      .accounts({
        buyer: wallet.publicKey,
        masterMint: mint,
        newMint: editionMint,
        editionMarker,
      })
      .signers([editionMintKeypair, wallet.payer])
      .rpc({ skipPreflight: true });

    console.log("BuyNft Tx:", tx);
  } catch (error) {
    if (error instanceof anchor.AnchorError) {
      console.error('AnchorError code:', error.error.errorCode);
      console.error('AnchorError message:', error.error.errorMessage);
      console.error('Program logs:', error.logs);
    } else {
      console.error('Unexpected error:', error);
    }
  }
  });

});