use {
    anchor_spl::token::spl_token::{
        self,
        state::{Account as SplTokenAccount, AccountState, Mint as SplMint},
    },
    litesvm::LiteSVM,
    solana_account::Account,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_program_option::COption,
    solana_program_pack::Pack,
    solana_signer::Signer,
    spl_associated_token_account_interface::address::get_associated_token_address,
};

pub fn create_mint(svm: &mut LiteSVM, authority: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let mint_pubkey = mint.pubkey();

    let mint_data = SplMint {
        mint_authority: COption::Some(*authority),
        supply: u64::MAX,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };

    let mut data = vec![0u8; SplMint::LEN];
    SplMint::pack(mint_data, &mut data).unwrap();

    svm.set_account(
        mint_pubkey,
        Account { lamports: 1_000_000_000, data, owner: spl_token::ID, executable: false, rent_epoch: 0 },
    )
    .unwrap();

    mint_pubkey
}

pub fn create_token_account(svm: &mut LiteSVM, mint: &Pubkey, owner: &Pubkey, amount: u64) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);

    let token_data = SplTokenAccount {
        mint: *mint,
        owner: *owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };

    let mut data = vec![0u8; SplTokenAccount::LEN];
    SplTokenAccount::pack(token_data, &mut data).unwrap();

    svm.set_account(
        ata,
        Account { lamports: 1_000_000_000, data, owner: spl_token::ID, executable: false, rent_epoch: 0 },
    )
    .unwrap();

    ata
}

pub fn setup_svm() -> (LiteSVM, Keypair, Keypair) {
    let program_id = liqour_defi::ID;
    let authority = Keypair::new();
    let user = Keypair::new();

    let mut svm = LiteSVM::new();
    svm.add_program(program_id, include_bytes!("../../../../target/deploy/liqour_defi.so"))
        .unwrap();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();

    (svm, authority, user)
}
