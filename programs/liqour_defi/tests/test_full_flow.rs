use {
    anchor_lang::{
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::token::spl_token::{
        self,
        state::{Account as SplTokenAccount, AccountState},
    },
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_program_pack::Pack,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_sysvar::rent,
    solana_transaction::versioned::VersionedTransaction,
    liqour_defi::state::{UserVault, VaultConfig},
};

mod common;

// ─── Helpers ──────────────────────────────────────────────────────────

fn init_program(svm: &mut litesvm::LiteSVM, authority: &Keypair, usdc_mint: Pubkey) -> (Pubkey, Keypair) {
    let (vault_config, _bump) =
        Pubkey::find_program_address(&[b"vault_config"], &liqour_defi::ID);
    let vault_token_account = Keypair::new();

    let accounts = liqour_defi::accounts::Initialize {
        authority: authority.pubkey(),
        usdc_mint,
        vault_token_account: vault_token_account.pubkey(),
        vault_config,
        token_program: spl_token::ID,
        system_program: system_program::ID,
        rent: rent::ID,
    };

    let ix = Instruction::new_with_bytes(
        liqour_defi::ID,
        &liqour_defi::instruction::Initialize {}.data(),
        accounts.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&authority.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[authority, &vault_token_account])
        .unwrap();
    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Init failed: {res:?}");
    (vault_config, vault_token_account)
}

fn deposit(
    svm: &mut litesvm::LiteSVM,
    user: &Keypair,
    user_usdc: Pubkey,
    vault_token_account: Pubkey,
    vault_config: Pubkey,
    user_vault: Pubkey,
    amount: u64,
) -> Result<(), ()> {
    let accounts = liqour_defi::accounts::Deposit {
        user: user.pubkey(),
        user_usdc,
        vault_token_account,
        vault_config,
        user_vault,
        token_program: spl_token::ID,
        system_program: system_program::ID,
        rent: rent::ID,
    };

    let ix = Instruction::new_with_bytes(
        liqour_defi::ID,
        &liqour_defi::instruction::Deposit { amount }.data(),
        accounts.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&user.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[user]).unwrap();
    svm.send_transaction(tx).map(|_| ()).map_err(|_| ())
}

fn withdraw(
    svm: &mut litesvm::LiteSVM,
    authority: &Keypair,
    user_usdc: Pubkey,
    vault_token_account: Pubkey,
    vault_config: Pubkey,
    user_vault: Pubkey,
    amount: u64,
) -> Result<(), ()> {
    let accounts = liqour_defi::accounts::Withdraw {
        authority: authority.pubkey(),
        user_usdc,
        vault_token_account,
        vault_config,
        user_vault,
        token_program: spl_token::ID,
    };

    let ix = Instruction::new_with_bytes(
        liqour_defi::ID,
        &liqour_defi::instruction::Withdraw { amount }.data(),
        accounts.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&authority.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[authority]).unwrap();
    svm.send_transaction(tx).map(|_| ()).map_err(|_| ())
}

// ─── Initialize Tests ────────────────────────────────────────────────

#[test]
fn test_initialize_success() {
    let (mut svm, authority, _user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let (vault_config, _) = init_program(&mut svm, &authority, usdc_mint);

    let raw = svm.get_account(&vault_config).unwrap();
    let cfg = VaultConfig::try_deserialize(&mut &raw.data[..]).unwrap();
    assert_eq!(cfg.authority, authority.pubkey());
    assert_eq!(cfg.usdc_mint, usdc_mint);
    assert_eq!(cfg.total_deposited, 0);
}

// ─── Deposit Tests ───────────────────────────────────────────────────

#[test]
fn test_deposit_success() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    let deposit_amount = 100_000_000;
    deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        deposit_amount,
    ).unwrap();

    let cfg = VaultConfig::try_deserialize(&mut &svm.get_account(&vault_config).unwrap().data[..]).unwrap();
    assert_eq!(cfg.total_deposited, deposit_amount);

    let uv = UserVault::try_deserialize(&mut &svm.get_account(&user_vault).unwrap().data[..]).unwrap();
    assert_eq!(uv.owner, user.pubkey());
    assert_eq!(uv.deposited, deposit_amount);
}

#[test]
fn test_deposit_updates_token_balances() {
    let initial_user_balance = 1_000_000_000_000u64;
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), initial_user_balance);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    let deposit_amount = 100_000_000;
    deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        deposit_amount,
    ).unwrap();

    let user_tokens = SplTokenAccount::unpack(&svm.get_account(&user_usdc).unwrap().data).unwrap();
    assert_eq!(user_tokens.amount, initial_user_balance - deposit_amount);

    let vault_tokens =
        SplTokenAccount::unpack(&svm.get_account(&vault_token_account.pubkey()).unwrap().data).unwrap();
    assert_eq!(vault_tokens.amount, deposit_amount);
}

#[test]
fn test_deposit_zero_amount_fails() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    let res = deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault, 0,
    );
    assert!(res.is_err(), "Expected deposit of 0 amount to fail");
}

#[test]
fn test_multiple_deposits_accumulate() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    deposit(&mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault, 100_000_000).unwrap();
    deposit(&mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault, 200_000_000).unwrap();
    deposit(&mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault, 300_000_000).unwrap();

    let cfg = VaultConfig::try_deserialize(&mut &svm.get_account(&vault_config).unwrap().data[..]).unwrap();
    assert_eq!(cfg.total_deposited, 600_000_000);

    let uv = UserVault::try_deserialize(&mut &svm.get_account(&user_vault).unwrap().data[..]).unwrap();
    assert_eq!(uv.deposited, 600_000_000);
}

// ─── Withdraw Tests ──────────────────────────────────────────────────

#[test]
fn test_withdraw_success() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    let deposit_amount = 100_000_000;
    deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        deposit_amount,
    ).unwrap();

    let withdraw_amount = 40_000_000;
    withdraw(
        &mut svm, &authority, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        withdraw_amount,
    ).unwrap();

    let cfg = VaultConfig::try_deserialize(&mut &svm.get_account(&vault_config).unwrap().data[..]).unwrap();
    assert_eq!(cfg.total_deposited, deposit_amount - withdraw_amount);

    let uv = UserVault::try_deserialize(&mut &svm.get_account(&user_vault).unwrap().data[..]).unwrap();
    assert_eq!(uv.withdrawn, withdraw_amount);

    let user_tokens = SplTokenAccount::unpack(&svm.get_account(&user_usdc).unwrap().data).unwrap();
    assert_eq!(user_tokens.amount, 1_000_000_000_000 - deposit_amount + withdraw_amount);
}

#[test]
fn test_withdraw_zero_amount_fails() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        100_000_000,
    ).unwrap();

    let res = withdraw(
        &mut svm, &authority, user_usdc, vault_token_account.pubkey(), vault_config, user_vault, 0,
    );
    assert!(res.is_err(), "Expected withdraw of 0 amount to fail");
}

#[test]
fn test_withdraw_insufficient_vault_balance_fails() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        50_000_000,
    ).unwrap();

    let res = withdraw(
        &mut svm, &authority, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        100_000_000,
    );
    assert!(res.is_err(), "Expected withdraw exceeding vault balance to fail");
}

#[test]
fn test_withdraw_unauthorized_fails() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        100_000_000,
    ).unwrap();

    // user (not authority) signs the withdraw tx — should fail Unauthorized
    let accounts = liqour_defi::accounts::Withdraw {
        authority: authority.pubkey(),
        user_usdc,
        vault_token_account: vault_token_account.pubkey(),
        vault_config,
        user_vault,
        token_program: spl_token::ID,
    };

    let ix = Instruction::new_with_bytes(
        liqour_defi::ID,
        &liqour_defi::instruction::Withdraw { amount: 10_000_000 }.data(),
        accounts.to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&user.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&user]).unwrap();
    let res = svm.send_transaction(tx);
    assert!(res.is_err(), "Expected unauthorized withdraw to fail");
}

// ─── Multi-User Tests ────────────────────────────────────────────────

#[test]
fn test_multiple_users_independent_vaults() {
    let (mut svm, authority, user1) = common::setup_svm();
    let user2 = Keypair::new();
    svm.airdrop(&user2.pubkey(), 10_000_000_000).unwrap();

    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user1_usdc = common::create_token_account(&mut svm, &usdc_mint, &user1.pubkey(), 1_000_000_000_000);
    let user2_usdc = common::create_token_account(&mut svm, &usdc_mint, &user2.pubkey(), 500_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);

    let (user1_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user1.pubkey().as_ref()], &liqour_defi::ID);
    let (user2_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user2.pubkey().as_ref()], &liqour_defi::ID);

    deposit(&mut svm, &user1, user1_usdc, vault_token_account.pubkey(), vault_config, user1_vault, 100_000_000).unwrap();
    deposit(&mut svm, &user2, user2_usdc, vault_token_account.pubkey(), vault_config, user2_vault, 250_000_000).unwrap();

    let cfg = VaultConfig::try_deserialize(&mut &svm.get_account(&vault_config).unwrap().data[..]).unwrap();
    assert_eq!(cfg.total_deposited, 350_000_000);

    let uv1 = UserVault::try_deserialize(&mut &svm.get_account(&user1_vault).unwrap().data[..]).unwrap();
    assert_eq!(uv1.deposited, 100_000_000);

    let uv2 = UserVault::try_deserialize(&mut &svm.get_account(&user2_vault).unwrap().data[..]).unwrap();
    assert_eq!(uv2.deposited, 250_000_000);
}

// ─── Full Round-Trip ─────────────────────────────────────────────────

#[test]
fn test_full_round_trip() {
    let (mut svm, authority, user) = common::setup_svm();
    let usdc_mint = common::create_mint(&mut svm, &authority.pubkey(), 6);
    let user_usdc = common::create_token_account(&mut svm, &usdc_mint, &user.pubkey(), 1_000_000_000_000);
    let (vault_config, vault_token_account) = init_program(&mut svm, &authority, usdc_mint);
    let (user_vault, _) =
        Pubkey::find_program_address(&[b"user_vault", user.pubkey().as_ref()], &liqour_defi::ID);

    let deposit_amount = 100_000_000;
    deposit(
        &mut svm, &user, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        deposit_amount,
    ).unwrap();

    let withdraw_amount = 100_000_000;
    withdraw(
        &mut svm, &authority, user_usdc, vault_token_account.pubkey(), vault_config, user_vault,
        withdraw_amount,
    ).unwrap();

    let cfg = VaultConfig::try_deserialize(&mut &svm.get_account(&vault_config).unwrap().data[..]).unwrap();
    assert_eq!(cfg.total_deposited, 0);

    let uv = UserVault::try_deserialize(&mut &svm.get_account(&user_vault).unwrap().data[..]).unwrap();
    assert_eq!(uv.deposited, deposit_amount);
    assert_eq!(uv.withdrawn, withdraw_amount);

    let user_tokens = SplTokenAccount::unpack(&svm.get_account(&user_usdc).unwrap().data).unwrap();
    assert_eq!(user_tokens.amount, 1_000_000_000_000);

    let vault_tokens =
        SplTokenAccount::unpack(&svm.get_account(&vault_token_account.pubkey()).unwrap().data).unwrap();
    assert_eq!(vault_tokens.amount, 0);
}
