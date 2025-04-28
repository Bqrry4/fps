pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("ADidMwkBx687QFpAFmYVJs3fqVLQz1BHNfb1dH4o5UgK");

#[program]
pub mod nft {
    use super::*;

    pub fn create_collection(
        ctx: Context<CreateCollection>,
        name: String,
        symbol: String,
    ) -> Result<()> {
        create_collection::create_collection(&ctx, name, symbol)
    }

    pub fn create_nft(
        ctx: Context<CreateNFT>,
        name: String,
        symbol: String,
        metadata_uri: String,
        supply: u64,
    ) -> Result<()> {
        create_nft::create_nft(&ctx, name, symbol, metadata_uri, supply)
    }

    pub fn buy_nft(
        ctx: Context<Buy>
    ) -> Result<()> {
        print_nft::buy(&ctx)
    }
}
