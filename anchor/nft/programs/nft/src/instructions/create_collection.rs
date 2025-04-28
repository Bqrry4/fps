use anchor_lang::prelude::*;
use anchor_spl::metadata::mpl_token_metadata::{
    self, instructions::{
        CreateMasterEditionV3Cpi, CreateMasterEditionV3CpiAccounts,
        CreateMasterEditionV3InstructionArgs, CreateMetadataAccountV3Cpi,
        CreateMetadataAccountV3CpiAccounts, CreateMetadataAccountV3InstructionArgs,
    }, types::{CollectionDetails, Creator, DataV2}
};
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::Metadata,
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        mint::decimals = 0,
        mint::authority = mint_authority,
        mint::freeze_authority = mint_authority,
    )]
    mint: Account<'info, Mint>,

    /// CHECK: address
    #[account(
        seeds = [b"authority"],
        bump,
    )]
    pub mint_authority: UncheckedAccount<'info>,
    /// CHECK: address
    #[account(
        mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            mint.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::ID,
        bump
    )]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: address
    #[account(mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            mint.key().as_ref(),
            b"edition",
        ],
        seeds::program = mpl_token_metadata::ID,
        bump
    )]
    pub master_edition: UncheckedAccount<'info>,
    #[account(
        init,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = creator
    )]
    destination: Account<'info, TokenAccount>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    token_metadata_program: Program<'info, Metadata>,
}

pub fn create_collection(ctx: &Context<CreateCollection>, name: String, symbol: String) -> Result<()> {
    let metadata = &ctx.accounts.metadata.to_account_info();
    let master_edition = &ctx.accounts.master_edition.to_account_info();
    let mint = &ctx.accounts.mint.to_account_info();
    let authority = &ctx.accounts.mint_authority.to_account_info();
    let payer = &ctx.accounts.creator.to_account_info();
    let system_program = &ctx.accounts.system_program.to_account_info();
    let spl_token_program = &ctx.accounts.token_program.to_account_info();
    let spl_metadata_program = &ctx.accounts.token_metadata_program.to_account_info();

    let seeds = &[&b"authority"[..], &[ctx.bumps.mint_authority]];
    let signer_seeds = &[&seeds[..]];

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.destination.to_account_info(),
        authority: ctx.accounts.mint_authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    mint_to(cpi_ctx, 1)?;
    msg!("Collection NFT minted!");

    let creator = vec![Creator {
        address: ctx.accounts.mint_authority.key().clone(),
        verified: true,
        share: 100,
    }];

    let metadata_account = CreateMetadataAccountV3Cpi::new(
        spl_metadata_program,
        CreateMetadataAccountV3CpiAccounts {
            metadata,
            mint,
            mint_authority: authority,
            payer,
            update_authority: (authority, true),
            system_program,
            rent: None,
        },
        CreateMetadataAccountV3InstructionArgs {
            data: DataV2 {
                name: name,
                symbol: symbol,
                uri: "".to_owned(),
                seller_fee_basis_points: 0,
                creators: Some(creator),
                collection: None,
                uses: None,
            },
            is_mutable: true,
            collection_details: Some(CollectionDetails::V1 { size: 0 }),
        },
    );
    metadata_account.invoke_signed(signer_seeds)?;
    msg!("Metadata Account created!");

    let master_edition_account = CreateMasterEditionV3Cpi::new(
        spl_metadata_program,
        CreateMasterEditionV3CpiAccounts {
            edition: master_edition,
            update_authority: authority,
            mint_authority: authority,
            mint,
            payer,
            metadata,
            token_program: spl_token_program,
            system_program,
            rent: None,
        },
        CreateMasterEditionV3InstructionArgs {
            max_supply: Some(0),
        },
    );
    master_edition_account.invoke_signed(signer_seeds)?;
    msg!("Master Edition Account created");

    Ok(())
}
