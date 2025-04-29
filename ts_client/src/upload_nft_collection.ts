import "dotenv/config"
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BN } from "bn.js";

import { Nft } from "./types/nft";
import { clusterApiUrl, Connection, Keypair, PublicKey } from "@solana/web3.js";
import { getExplorerLink, getKeypairFromEnvironment } from "@solana-developers/helpers";
import idl from "./idl/nft.json";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { createGenericFile, createSignerFromKeypair, signerIdentity } from "@metaplex-foundation/umi"
import { irysUploader } from "@metaplex-foundation/umi-uploader-irys"
import { readFile } from "fs/promises";
import Wallet from "@coral-xyz/anchor/dist/esm/nodewallet.js"

const keypair = getKeypairFromEnvironment("WS_1");

const cluster = clusterApiUrl("devnet");
const connection = new Connection(cluster, "confirmed");

const wallet = new Wallet(keypair);

const provider = new anchor.AnchorProvider(connection, wallet);
anchor.setProvider(provider)

const program = new Program(idl as Nft, provider);

const umi = createUmi(cluster);
const umi_kp = umi.eddsa.createKeypairFromSecretKey(keypair.secretKey);
const signer = createSignerFromKeypair(umi, umi_kp);
umi.use(irysUploader());
umi.use(signerIdentity(signer));

async function create_collection(creator: PublicKey, name: string, symbol: string) {
    console.log("Creating collection NFT...");
    console.log(`Creator: ${creator.toBase58()} \n Name: ${name} \n Symbol: ${symbol}`);

    const collectionKeypair = Keypair.generate();
    const collectionMint = collectionKeypair.publicKey;

    const tx = await program.methods.createCollection(
        name,
        symbol,
    ).accounts({
        creator: creator,
        mint: collectionMint,
    })
    .signers([collectionKeypair])
    .rpc({
        skipPreflight: true,
    });

    const sigLink = getExplorerLink("transaction", tx, "devnet");
    console.log(`Collection NFT created: TxID ${sigLink}`);

    return collectionMint;
}

async function upload_nft_to_collection(creator: PublicKey, collectionMint: PublicKey, name: string, symbol: string, uri: string) {
    console.log("Creating NFT...");
    console.log(`Collection Mint: ${collectionMint.toBase58()} \n Name: ${name} \n Symbol: ${symbol} \n URI: ${uri}`);

    const nftMintKeypair = Keypair.generate();
    const nftMint = nftMintKeypair.publicKey;

    console.log("\nMint", nftMint.toBase58());

    const tx = await program.methods.createNft(
        name,
        symbol,
        uri,
        new BN(1),
    )
    .accounts({
        creator: creator,
        mint: nftMint,
        collectionMint,
    })
    .signers([nftMintKeypair])
    .rpc({
        skipPreflight: true,
    });

    const sigLink = getExplorerLink("transaction", tx, "devnet");
    console.log(`Master edition nft created!: TxID ${sigLink}`);

    return nftMint;
}

async function upload_metadata(name: String, symbol: String) 
{
    console.log("Uploading metadata...");
    const assets = {
        "p": "../../skins/ak_1/ak_p.png", // preview
        "a": "../../skins/ak_1/ak_a.png",
        "r": "../../skins/ak_1/ak_r.png",
        "n": "../../skins/ak_1/ak_n.png",
        "m": "../../skins/ak_1/ak_m.png",
        "ao": "../../skins/ak_1/ak_ao.png",
    }

    const attributes = {};
    for(const tex in assets)
    {
        console.log(`Uploading texture ${tex}...`);
        const img = await readFile(assets[tex]);
        const imgConverted = createGenericFile(new Uint8Array(img), "image/png");

        const [uri] = await umi.uploader.upload([imgConverted]);
        attributes[tex] = uri;
    }

    const IMG_URI = attributes["p"];
    delete attributes["p"];

    const metadata = {
        name,
        symbol,
        description: "",
        image: IMG_URI,
        attributes: [],
        properties: {
            files: [{type: "image/png", uri: IMG_URI}]
        },
        textures : attributes,
    };

    const metadataUri = await umi.uploader.uploadJson(metadata);
    return metadataUri;
}

async function upload_nft() {

    const n_name = "DEF_Ak";
    const n_symbol = "DFAK";
    const c_name = "Default";
    const c_symbol = "DEF";

    let uri = await upload_metadata(n_name, n_symbol);

    let collectionMint = await create_collection(keypair.publicKey, c_name, c_symbol);
    let nftMint = await upload_nft_to_collection(keypair.publicKey, collectionMint, "DEF_Ak", "DFAK", uri);

    console.log(`Nft uploaded ${nftMint}`);
}

upload_nft();

