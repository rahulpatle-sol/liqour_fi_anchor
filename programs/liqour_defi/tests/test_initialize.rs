use {
    anchor_lang::{
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::token::spl_token::{
        self,
        state::{Account as SplTokenAccount, AccountState, Mint as SplMint},
    },
    litesvm::LiteSVM,
    solana_account::Account,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program_option::COption,
    solana_program_pack::Pack,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_sysvar::rent,
    solana_transaction::versioned::VersionedTransaction,
    spl_associated_token_account_interface::address::get_associated_token_address,
    liqour_defi::state::{UserVault, VaultConfig},
};

fn create_mint(svm: &mut LiteSVM, authority: &Pubkey, decimals: u8) -> Pubkey {
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

fn create_token_account(svm: &mut LiteSVM, mint: &Pubkey, owner: &Pubkey, amount: u64) -> Pubkey {
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

#[test]
fn test_full_flow() {
    let program_id = liqour_defi::ID;
    let authority = Keypair::new();
    let user = Keypair::new();

    let mut svm = LiteSVM::new();
    svm.add_program(program_id, include_bytes!("../../../target/deploy/liqour_defi.so"))
        .unwrap();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&user.pubkey(), 10_000_000_000).unwrap();

    // ── setup mint & user token account ──────────────────────────────
    let usdc_mint = create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc =
        create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);

    // ── PDA derivation ───────────────────────────────────────────────
    let (vault_config, _bump) =
        Pubkey::find_program_address(&[b"vault_config"], &program_id);

    let vault_token_account = Keypair::new();

    // ────────────────────── INITIALIZE ────────────────────────────────
    let init_accounts = liqour_defi::accounts::Initialize {
        authority:            authority.pubkey(),
        usdc_mint,
        vault_token_account:  vault_token_account.pubkey(),
        vault_config,
        token_program:        spl_token::ID,
        system_program:       system_program::ID,
        rent:                 rent::ID,
    };

    let init_ix = Instruction::new_with_bytes(
        program_id,
        &liqour_defi::instruction::Initialize {}.data(),
        init_accounts.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[init_ix], Some(&authority.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&authority, &vault_token_account])
        .unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Initialize failed: {res:?}");

    // verify vault_config
    let raw = svm.get_account(&vault_config).unwrap();
    let cfg = VaultConfig::try_deserialize(&mut &raw.data[..]).unwrap();
    assert_eq!(cfg.authority, authority.pubkey());
    assert_eq!(cfg.usdc_mint, usdc_mint);
    assert_eq!(cfg.total_deposited, 0);

    // ──────────────────────── DEPOSIT ─────────────────────────────────
    let (user_vault, _uv_bump) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &program_id);

    let deposit_amount: u64 = 100_000_000; // 100 USDC
    let deposit_accounts = liqour_defi::accounts::Deposit {
        user:                 user.pubkey(),
        user_usdc,
        vault_token_account:  vault_token_account.pubkey(),
        vault_config,
        user_vault,
        token_program:        spl_token::ID,
        system_program:       system_program::ID,
        rent:                 rent::ID,
    };

    let deposit_ix = Instruction::new_with_bytes(
        program_id,
        &liqour_defi::instruction::Deposit { amount: deposit_amount }.data(),
        deposit_accounts.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[deposit_ix], Some(&user.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&user]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Deposit failed: {res:?}");

    let cfg = VaultConfig::try_deserialize(&mut &svm.get_account(&vault_config).unwrap().data[..])
        .unwrap();
    assert_eq!(cfg.total_deposited, deposit_amount);

    let uv = UserVault::try_deserialize(&mut &svm.get_account(&user_vault).unwrap().data[..])
        .unwrap();
    assert_eq!(uv.owner, user.pubkey());
    assert_eq!(uv.deposited, deposit_amount);

    // verify vault token balance
    let vault_tokens =
        SplTokenAccount::unpack(&svm.get_account(&vault_token_account.pubkey()).unwrap().data)
            .unwrap();
    assert_eq!(vault_tokens.amount, deposit_amount);

    // ─────────────────────── WITHDRAW ─────────────────────────────────
    let withdraw_amount: u64 = 50_000_000; // 50 USDC
    let withdraw_accounts = liqour_defi::accounts::Withdraw {
        authority:            authority.pubkey(),
        user_usdc,
        vault_token_account:  vault_token_account.pubkey(),
        vault_config,
        user_vault,
        token_program:        spl_token::ID,
    };

    let withdraw_ix = Instruction::new_with_bytes(
        program_id,
        &liqour_defi::instruction::Withdraw { amount: withdraw_amount }.data(),
        withdraw_accounts.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[withdraw_ix], Some(&authority.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&authority]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Withdraw failed: {res:?}");

    let cfg = VaultConfig::try_deserialize(&mut &svm.get_account(&vault_config).unwrap().data[..])
        .unwrap();
    assert_eq!(cfg.total_deposited, deposit_amount - withdraw_amount);

    let uv = UserVault::try_deserialize(&mut &svm.get_account(&user_vault).unwrap().data[..])
        .unwrap();
    assert_eq!(uv.withdrawn, withdraw_amount);

    let user_tokens_after =
        SplTokenAccount::unpack(&svm.get_account(&user_usdc).unwrap().data).unwrap();
    assert_eq!(user_tokens_after.amount, 1_000_000_000_000 - deposit_amount + withdraw_amount);
}
