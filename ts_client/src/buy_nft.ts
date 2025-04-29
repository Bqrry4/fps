import "dotenv/config"
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Nft } from "./types/nft";
import { clusterApiUrl, Connection, Keypair, PublicKey } from "@solana/web3.js";
import { getExplorerLink, getKeypairFromEnvironment } from "@solana-developers/helpers";
import idl from "./idl/nft.json";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import Wallet from "@coral-xyz/anchor/dist/esm/nodewallet.js"
import {
    fetchMasterEditionFromSeeds,
    findEditionMarkerPda,
} from "@metaplex-foundation/mpl-token-metadata";
import { fromWeb3JsPublicKey } from "@metaplex-foundation/umi-web3js-adapters";

const keypair = getKeypairFromEnvironment("WS_2");
const cluster = clusterApiUrl("devnet");
const connection = new Connection(cluster, "confirmed");

const wallet = new Wallet(keypair);

const provider = new anchor.AnchorProvider(connection, wallet);
anchor.setProvider(provider)

const program = new Program(idl as Nft, provider);
const umi = createUmi(cluster);

async function buy_nft(mint: PublicKey) {

    console.log("Buying NFT...");
    console.log(`Mint: ${mint.toBase58()}`);

    const editionMintKeypair = Keypair.generate();
    const editionMint = editionMintKeypair.publicKey;

    const masterEdition = await fetchMasterEditionFromSeeds(
        umi,
        { mint: fromWeb3JsPublicKey(mint) },
    );

    const editionMarkerIx = Math.floor(Number(masterEdition.supply) / 248);
    const editionMarker = await findEditionMarkerPda(
        umi,
        {
            mint: fromWeb3JsPublicKey(mint),
            editionMarker: editionMarkerIx.toString()
        },
    );

    const tx = await program.methods.buyNft()
    .accounts({
        buyer: keypair.publicKey,
        masterMint: mint,
        newMint: editionMint,
        editionMarker: editionMarker[0],
    })
    .signers([editionMintKeypair, keypair])
    .rpc({ skipPreflight: true });

    const sigLink = getExplorerLink("transaction", tx, "devnet");
    console.log(`New edition created: TxID ${sigLink}`);
}

let mint = new PublicKey("DnUXUFJqnL1EHWZjQK9UAkkWz79q75Pm2w6WtoPN3Zc8");
buy_nft(mint);