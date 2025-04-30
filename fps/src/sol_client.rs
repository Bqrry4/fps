use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;

use anchor_client::anchor_lang::AnchorDeserialize;
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_request::TokenAccountsFilter;
use anchor_client::solana_sdk::pubkey::Pubkey;
use dashmap::DashSet;
use dashmap::DashMap;
use mpl_token_metadata::accounts::Metadata;

use raylib::texture::{Image, WeakTexture2D};
use raylib::{RaylibHandle, RaylibThread};
use solana_account_decoder::UiAccountData;

use reqwest::blocking::get;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct SkinMetadata {
    pub name:   String,
    pub symbol: String,
    pub identifier: String,
    pub textures: TextureField,
}

#[derive(Deserialize, Debug)]
pub struct TextureField {
    pub a : String,
    pub r : String,
    pub n : String,
    pub m : String,
    pub ao : String,
}

/**
 * Arc on fields to copy the object in the async tasks
 * Dashmap for concurrent access
 */
#[derive(Clone)]
pub struct SolanaClient {
    sol_client: Arc<RpcClient>,
    token_program_id: Pubkey,
    // to avoid duplicate concurrent fetches
    in_flight: Arc<DashSet<String>>, 
    //Just a hack to cache the texture requests that are on each frame.. yeah 
    //Mint -> Map(texture_identifier -> texture)
    skin_map: Arc<DashMap<String, Arc<HashMap<String, WeakTexture2D>>>>,
    // skin_imgs_map: Arc<DashMap<String, Arc<HashMap<String, Image>>>>
    raw_bytes: Arc<DashMap<String, HashMap<String, Vec<u8>>>>
}

impl SolanaClient {
    pub fn new() -> Self {
        let rpc_url = "https://api.devnet.solana.com"; // Changed to Devnet
        let sol_client = RpcClient::new(rpc_url.to_string());

        let token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();

        Self {
            sol_client: Arc::new(sol_client),
            token_program_id,
            skin_map: Arc::new(DashMap::new()),
            raw_bytes: Arc::new(DashMap::new()),
            in_flight: Arc::new(DashSet::new()),
        }
    }

    //General blocking fetch at the initialization
    pub fn fetch_skins(&self, public_key: Pubkey) -> Result<Vec<(Pubkey, SkinMetadata)>, String> {
    
        let token_accounts = self.sol_client
            .get_token_accounts_by_owner(
                &public_key,
                TokenAccountsFilter::ProgramId(self.token_program_id),
            )
            .unwrap();
    
        let skins: Vec<(Pubkey, SkinMetadata)> = token_accounts
            .iter()
            //Get mints from ATAs
            .map(|account| match &account.account.data {
                UiAccountData::Json(parsed_account) => {
                    Some(
                        parsed_account.parsed["info"]["mint"].as_str().unwrap()
                    )
                }
                _ => {
                    None
                }
            })
            //Filter out None values
            .filter_map(|mint| mint)
            //Filter the nft mints
            .filter_map(|mint|{
                let pk = Pubkey::from_str(mint).unwrap();
                let decimals = self.sol_client.get_token_supply(&pk).unwrap().decimals;
                if decimals == 0 {
                    return Some(pk);
                }
                None
            })
            //Fetch metadata uri's
            .map(|mint| {
                //let mint: solana_program::pubkey::Pubkey = solana_program::pubkey::Pubkey::new_from_array(mint.to_bytes());
                
                let metadata_pda = Pubkey::find_program_address(
                    &[
                        b"metadata",
                        mpl_token_metadata::ID.as_ref(),
                        mint.as_ref(),
                    ],
                    &mpl_token_metadata::ID,
                ).0;
    
                let data = self.sol_client.get_account_data(&metadata_pda).unwrap();
                let metadata: Metadata = Metadata::deserialize(&mut data.as_slice()).unwrap();
    
                (mint, metadata.uri)
    
            })
            .filter_map(|nft| {
                let uri = nft.1.as_str();
                let response = get(uri).unwrap();
                if !response.status().is_success()
                {
                    return None;
                }

                return match response.json::<SkinMetadata>() {
                    Ok(skin_data) => {
                        if skin_data.identifier == "fps+bq"
                        {
                            Some((nft.0, skin_data))
                        }
                        else {
                            None
                        }
                    },
                    _ => {None}
                };

            })
            .collect();
    
        Ok(skins)
    }

    //Fetches at runtime so should not block, the current implementation is for invoking this function at each frame
    pub fn fetch_skin(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, mint: &String) -> Option<HashMap<String, WeakTexture2D>> {

        let mint = Pubkey::from_str(mint).unwrap();
        let key = mint.to_string();
        //Return if it is available
        if let Some(entry) = self.skin_map.get(&key) {
            return Some((**entry).clone());
        }

        // if its not available check if it has fullfiled the image bytes load
        // This needs to be done in the main thread as OpenGL only allows texture loading from the thread it's context was created
        // The Image objects are wrapping a C pointer so can't pass them in a closure, game dev in rust is a joke
        if let Some(entry) = self.raw_bytes.get(&key) {
            let textures = SolanaClient::fetch_textures(rl, thread, (*entry).clone()).unwrap();
            self.skin_map.insert(key.clone(), Arc::new(textures));
            return Some((**self.skin_map.get(&key).unwrap()).clone());
        }

        //if there is no loaded images for it spawn a thread to fetch it
        if self.in_flight.insert(key.clone()) {
            let loader = self.clone();
            
            thread::spawn(move || {
                let metadata_pda = Pubkey::find_program_address(
                    &[
                        b"metadata",
                        mpl_token_metadata::ID.as_ref(),
                        mint.as_ref(),
                    ],
                    &mpl_token_metadata::ID,
                ).0;
        
                let data = loader.sol_client.get_account_data(&metadata_pda).unwrap();
                let metadata: Metadata = Metadata::deserialize(&mut data.as_slice()).unwrap();
        
                let uri = metadata.uri;
                let response = get(uri).unwrap();
                if response.status().is_success()
                {
                    let skin_md = response.json::<SkinMetadata>().unwrap();
                    let imgs = SolanaClient::fetch_images_bytes(&skin_md.textures).unwrap();
                    loader.raw_bytes.insert(key.clone(), imgs);
                }

                // Remove from in-flight
                loader.in_flight.remove(&key);
            });
        }

        None
       
    }


    pub fn fetch_images_bytes(tf: &TextureField) -> Result<HashMap<String, Vec<u8>>, reqwest::Error>
    {
        let mut map: HashMap<String, Vec<u8>> = HashMap::new();

        for (key, url) in &[
            ("a",  &tf.a),
            ("r",  &tf.r),
            ("n",  &tf.n),
            ("m",  &tf.m),
            ("ao", &tf.ao),
        ] {
            let resp = get(*url)?;
            let bytes = resp.bytes()?;

            map.insert((*key).to_string(), bytes.to_vec());
        }

        Ok(map)
    }

    //-! Weak textures need to be unloaded manually
    pub fn fetch_textures(rl: &mut RaylibHandle, thread: &RaylibThread, imgs: HashMap<String, Vec<u8>>) -> Result<HashMap<String, WeakTexture2D>, String>
    {
        let mut map: HashMap<String, WeakTexture2D> = HashMap::new();

        for(key, bytes) in &imgs {

            let img = Image::load_image_from_mem(".png", &bytes)
            .expect("Failed to load image from memory");

            let texture = unsafe {
                rl.load_texture_from_image(thread, &img)
                .expect("Failed to load image to texture")
                .make_weak()
            };

            map.insert((*key).to_string(), texture);
        }

        Ok(map)
    }

    //Textures must be cleared manually
    pub fn clear(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread,) {
        self.skin_map.iter().for_each(|entry| {
            entry.value().iter().for_each(|(_, texture)| {
                unsafe { rl.unload_texture(&thread, texture.to_owned()) };
            });
        });
    }

}