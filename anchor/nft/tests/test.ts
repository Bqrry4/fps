import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Nft } from "../target/types/nft";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import {
	fetchMasterEditionFromSeeds,
	findMasterEditionPda,
	safeFetchMasterEditionFromSeeds,
} from "@metaplex-foundation/mpl-token-metadata";
import "dotenv/config"
import { clusterApiUrl } from "@solana/web3.js";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";

import { fromWeb3JsPublicKey } from "@metaplex-foundation/umi-web3js-adapters";


describe("mint-nft", () => {
	const provider = anchor.AnchorProvider.env();
	anchor.setProvider(provider);

	const wallet = provider.wallet as NodeWallet

	const program = anchor.workspace.Nft as Program<Nft>;

	const TOKEN_METADATA_PROGRAM_ID = new anchor.web3.PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

	// const umi = createUmi(clusterApiUrl('devnet'));
	const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
	const umi = createUmi(connection);

	const collectionKeypair = Keypair.generate();
	const collectionMint = collectionKeypair.publicKey;

	const mintKeypair = Keypair.generate();
	const mint = mintKeypair.publicKey;

	it("Create Collection NFT", async () => {
		console.log("\nCollection Mint Key: ", collectionMint.toBase58());

		// const destination = getAssociatedTokenAddressSync(collectionMint, wallet.publicKey);
		// console.log("Destination ATA = ", destination.toBase58());

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
			new anchor.BN(1)
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

	//Second test should fail
	for (let i = 0; i < 2; i++) {
		it("Buy NFT", async () => {

			const editionMintKeypair = Keypair.generate();
			const editionMint = editionMintKeypair.publicKey;

			console.log("\nEdition Mint Key: ", editionMint.toBase58());
			console.log("\nEdition Mint Key: ", mint.toBase58());

			// Fetch the Master Edition account
			const masterEdition = await fetchMasterEditionFromSeeds(
				umi,
				{ mint: fromWeb3JsPublicKey(mint) },
			);

			//fetchEditionMarkerFromSeeds
			const editionMarkerIx = Math.floor(Number(masterEdition.supply) / 248);
			const [editionMarker] = PublicKey.findProgramAddressSync(
				[
					Buffer.from("metadata"),
					TOKEN_METADATA_PROGRAM_ID.toBuffer(),
					mint.toBuffer(),
					Buffer.from("edition"),
					Buffer.from(editionMarkerIx.toString()),
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
	}
});