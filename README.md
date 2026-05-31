# Liqour DeFi

A Solana on-chain vault program built with Anchor 1.0.2. Users deposit USDC into a vault and an authorised backend wallet can execute withdrawals on their behalf.

**Program ID:** `FGJS4S51o9rSvxeomGrqacdwPFnZbBuU6p9KzhRHUx3b`
**Devnet Explorer:** https://explorer.solana.com/address/FGJS4S51o9rSvxeomGrqacdwPFnZbBuU6p9KzhRHUx3b?cluster=devnet

---

## System Design

### Accounts

| Account | Type | Purpose |
|---|---|---|
| `VaultConfig` | PDA (`seeds=["vault_config"]`) | Global vault settings: authority, mint, vault token account, total deposited |
| `UserVault` | PDA (`seeds=["user_vault", user_key]`) | Per-user deposit/withdraw tracking |
| `vault_token_account` | Token account owned by `VaultConfig` PDA | Holds all pooled USDC |

### Instructions

**Initialize** – One-time setup by the authority.
- Creates the `VaultConfig` PDA.
- Creates the vault's USDC token account (authority = `VaultConfig` PDA).

**Deposit** – Called by any user.
- Transfers USDC from user → vault token account (SPL Token CPI).
- Creates or updates the user's `UserVault` PDA (`init_if_needed`).
- Updates `VaultConfig.total_deposited`.

**Withdraw** – Called only by the stored authority (backend wallet).
- Authority checks `authority.key() == vault_config.authority`.
- `VaultConfig` PDA signs to transfer USDC from vault → user's token account.
- Updates `UserVault.withdrawn` and `VaultConfig.total_deposited`.

### PDA Signing

The `VaultConfig` PDA signs the withdraw CPI via `CpiContext::new_with_signer` using seeds `[b"vault_config", &[bump]]`.

### Error Codes

| Code | Message |
|---|---|
| `ZeroAmount` | Amount must be greater than zero |
| `InsufficientVaultBalance` | Vault token account has insufficient balance |
| `Unauthorized` | Only the vault authority can withdraw |
| `Overflow` | Math overflow |

---

## How It Works

```
User                     Vault (Solana Program)            USDC Mint
 │                             │                               │
 ├── deposit(amount) ─────────▶│                               │
 │                             ├── CPI transfer(user→vault) ──▶│
 │                             │◀── OK ───────────────────────┤
 │                             │                               │
 │◀── OK ─────────────────────┤                               │
 │                             │                               │
Authority                  Vault (Solana Program)            USDC Mint
 │                             │                               │
 ├── withdraw(amount) ────────▶│                               │
 │                             ├── CPI transfer(vault→user) ──▶│
 │                             │◀── OK ───────────────────────┤
 │◀── OK ─────────────────────┤                               │
```

---

## Tests

### Integration Test (`test_full_flow`)

Uses **LiteSVM** (lightweight Solana VM) to run a full end-to-end scenario:

1. **Setup** – Deploy program, create USDC mint, create user's USDC ATA with `1_000_000_000_000` balance.
2. **Initialize** – Authority creates `VaultConfig` PDA and vault token account.
3. **Deposit** – User deposits 100 USDC (`100_000_000` micro-USDC). Verifies:
   - `VaultConfig.total_deposited == 100_000_000`
   - `UserVault.owner == user`, `UserVault.deposited == 100_000_000`
   - Vault token account balance == `100_000_000`
4. **Withdraw** – Authority withdraws 50 USDC (`50_000_000`). Verifies:
   - `VaultConfig.total_deposited == 50_000_000` (100 - 50)
   - `UserVault.withdrawn == 50_000_000`
   - User token account final balance == `1_000_000_000_000 - 100_000_000 + 50_000_000`

### Unit Tests (`test_*_ix_data`)

Verify the `InstructionData` trait compiles and produces non-empty serialized payloads for all three instructions.

### Running

```bash
anchor build
cargo test
```

---

## Tech Stack

- **Anchor 1.0.2** – Framework
- **Solana 3.x** – Runtime
- **anchor-spl** – Token/Token-2022 interface
- **LiteSVM 0.10** – In-process test validator
