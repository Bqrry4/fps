use anchor_lang::{prelude::*, solana_program};
use anchor_spl::{metadata::{
    mpl_token_metadata::{
        self,
        instructions::{
            MintNewEditionFromMasterEditionViaTokenCpi,
            MintNewEditionFromMasterEditionViaTokenCpiAccounts,
            MintNewEditionFromMasterEditionViaTokenInstructionArgs,
        },
        types::MintNewEditionFromMasterEditionViaTokenArgs,
        EDITION_MARKER_BIT_SIZE,
    },
    MasterEditionAccount, Metadata, MetadataAccount,
}, token::{mint_to, MintTo}};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    pub master_mint: Box<Account<'info, Mint>>,

    /// CHECK: address
    #[account(
        seeds = [b"authority"],
        bump
    )]
    pub mint_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = master_mint,
        associated_token::authority = mint_authority
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    // Derive the metadata as it should exist
    #[account(
        mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            master_mint.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::ID,
        bump
    )]
    pub metadata: Box<Account<'info, MetadataAccount>>,

    // Derive the master edition as it should exist
    #[account(mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            master_mint.key().as_ref(),
            b"edition",
        ],
        seeds::program = mpl_token_metadata::ID,
        bump,
    )]
    pub master_edition: Box<Account<'info, MasterEditionAccount>>,

    // The new mint of the copy
    #[account(
        init,
        payer = buyer,
        mint::decimals = 0,
        mint::authority = buyer,
        mint::freeze_authority = buyer,
    )]
    pub new_mint: Box<Account<'info, Mint>>,

    // Account to where the new edition will be printed
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = new_mint,
        associated_token::authority = buyer
    )]
    pub edition_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: address
    #[account(
        mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            new_mint.key().as_ref()
        ],
        seeds::program = mpl_token_metadata::ID,
        bump
    )]
    pub new_metadata: UncheckedAccount<'info>,

    /// CHECK: address
    #[account(
        mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            new_mint.key().as_ref(),
            b"edition"
        ],
        seeds::program = mpl_token_metadata::ID,
        bump
    )]
    pub new_edition: UncheckedAccount<'info>,

    /// CHECK: address
    #[account(
        mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            master_mint.key().as_ref(),
            b"edition",
            (master_edition.supply / EDITION_MARKER_BIT_SIZE).to_string().as_bytes()
        ],
        seeds::program = mpl_token_metadata::ID,
        bump
    )]
    pub edition_marker: UncheckedAccount<'info>,
    // #[account(mut)]

    // /// CHECK: address
    // pub edition_marker: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metadata>,
}

pub fn buy(ctx: &Context<Buy>) -> Result<()> {
    let current_supply = ctx.accounts.master_edition.supply;
    let next_edition = current_supply.checked_add(1).unwrap();
    let marker_index = (next_edition - 1) / EDITION_MARKER_BIT_SIZE;
    let marker_bytes = marker_index.to_le_bytes();

    // let (marker_derived, marker_bump) =
    // Pubkey::find_program_address(
    //     &[
    //         b"metadata",
    //         mpl_token_metadata::ID.as_ref(),
    //         ctx.accounts.master_mint.key().as_ref(),
    //         b"edition",
    //         &marker_bytes,
    //     ],
    //     &mpl_token_metadata::ID,
    // );
    // solana_program::pubkey::Pubkey::find_program_address(
    //     &[
    //         "metadata".as_bytes(),
    //         mpl_token_metadata::ID.as_ref(),
    //         ctx.accounts.master_mint.key().as_ref(),
    //         "edition".as_bytes(),
    //         marker_index.to_string().as_ref(),
    //     ],
    //     &mpl_token_metadata::ID,
    // );
    // msg!("Derived edition marker PDA: {}", marker_derived);
    // msg!(
    //     "Provided edition marker key: {}",
    //     ctx.accounts.edition_marker.key()
    // );
    // require_keys_eq!(
    //     marker_derived,
    //     ctx.accounts.edition_marker.key(),
    //     ErrorCode::AccountNotProgramData
    // );

    let buyer = &ctx.accounts.buyer.to_account_info();
    let metadata = &ctx.accounts.metadata.to_account_info();
    let master_edition = &ctx.accounts.master_edition.to_account_info();
    let token_account = &ctx.accounts.vault.to_account_info();
    let new_mint = &ctx.accounts.new_mint.to_account_info();
    let new_metadata = &ctx.accounts.new_metadata.to_account_info();
    let new_edition = &ctx.accounts.new_edition.to_account_info();
    let edition_mark_pda = &ctx.accounts.edition_marker.to_account_info();
    let mint_authority = &ctx.accounts.mint_authority.to_account_info();
    let payer = &ctx.accounts.buyer.to_account_info();
    let system_program = &ctx.accounts.system_program.to_account_info();
    let token_program = &ctx.accounts.token_program.to_account_info();
    let spl_metadata_program = &ctx.accounts.token_metadata_program.to_account_info();

    let seeds = &[&b"authority"[..], &[ctx.bumps.mint_authority]];
    let signer_seeds = &[&seeds[..]];

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = MintTo {
        mint: ctx.accounts.new_mint.to_account_info(),
        to: ctx.accounts.edition_token_account.to_account_info(),
        authority: ctx.accounts.buyer.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    mint_to(cpi_ctx, 1)?;
    msg!("NFT minted!");

    let print = MintNewEditionFromMasterEditionViaTokenCpi::new(
        spl_metadata_program,
        MintNewEditionFromMasterEditionViaTokenCpiAccounts {
            metadata,
            master_edition,
            new_metadata,
            new_edition,
            new_metadata_update_authority: mint_authority,
            new_mint_authority: buyer,
            new_mint,
            token_account_owner: mint_authority,
            token_account,
            payer,
            token_program,
            system_program,
            edition_mark_pda,
            rent: None,
        },
        MintNewEditionFromMasterEditionViaTokenInstructionArgs {
            mint_new_edition_from_master_edition_via_token_args:
                MintNewEditionFromMasterEditionViaTokenArgs {
                    edition: ctx.accounts.master_edition.supply + 1,
                },
        },
    );
    print.invoke_signed(signer_seeds)?;

    Ok(())
}
