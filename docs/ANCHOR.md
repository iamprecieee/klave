# Anchor Framework — Complete Reference

## What is Anchor

Anchor is the primary development framework for building Solana programs (smart contracts). It wraps the low-level Solana SDK with Rust macros that eliminate boilerplate, enforce account validation, handle serialization, and guard against common security vulnerabilities automatically.

If you come from a backend Rust background: Anchor is to Solana what Actix or Axum are to HTTP — it does not change what the underlying runtime does, it gives you a structured, productive way to build on top of it with less friction and fewer footguns.

A Solana program is stateless code deployed on-chain. All state lives in separate **accounts** that are passed into program instructions at call time. Anchor's job is to make declaring, validating, and interacting with those accounts safe and ergonomic.

---

## Table of Contents

1. Environment Setup
2. Project Structure
3. Core Macros
4. Instruction Context
5. Account Types
6. Account Constraints
7. Account Space Calculation
8. Program Derived Addresses (PDAs)
9. Cross Program Invocations (CPIs)
10. Token Operations (SPL Tokens)
11. Token Extensions (Token 2022)
12. Custom Errors
13. Events
14. Zero-Copy Deserialization
15. Program IDL File
16. declare_program! Macro
17. Anchor.toml Configuration
18. Anchor CLI Reference
19. Anchor Version Manager (AVM)
20. Testing
21. Verifiable Builds
22. Common Pitfalls and Security Notes
23. Dependency Reference

---

## 1. Environment Setup

### Quick Install (Mac and Linux)

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://solana-install.solana.workers.dev | bash
```

This single command installs all required tools:

- Rust (via rustup)
- Solana CLI
- Anchor CLI
- Node.js
- Yarn

After it completes, restart your terminal. Verify:

```
Installed Versions:
Rust: rustc 1.85.0
Solana CLI: solana-cli 2.1.15
Anchor CLI: anchor-cli 0.32.1
Node.js: v23.9.0
Yarn: 1.22.1
```

### Windows

Install WSL (Windows Subsystem for Linux) first, then run the quick install command above inside the Ubuntu terminal.

---

### Manual Installation (Step by Step)

Follow this order. Each tool depends on the one before it.

#### Step 1 — Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
rustc --version
```

#### Step 2 — Solana CLI

```bash
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
```

Add to your PATH (Linux/WSL):

```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

Verify:

```bash
solana --version
# solana-cli 2.0.26
```

Update Solana CLI later:

```bash
agave-install update
```

#### Step 3 — Anchor CLI via AVM

AVM (Anchor Version Manager) is recommended. It lets you switch Anchor versions across projects.

```bash
# Install AVM
cargo install --git https://github.com/coral-xyz/anchor avm --force

# Verify AVM
avm --version

# Install latest Anchor
avm install latest
avm use latest

# Or install a specific version
avm install 0.32.1
avm use 0.32.1

# Verify Anchor
anchor --version
# anchor-cli 0.32.1
```

You must run `avm use` after installing. AVM will not activate a version automatically.

On Linux/WSL, if you see `error: could not exec the linker cc`, install the build dependencies for your distro (gcc, build-essential) then retry.

---

### Solana CLI Basics

These commands are used frequently during development and testing.

#### Configuration

```bash
# View current config
solana config get

# Output:
# Config File: /Users/you/.config/solana/cli/config.yml
# RPC URL: https://api.mainnet-beta.solana.com
# Keypair Path: /Users/you/.config/solana/id.json
# Commitment: confirmed

# Set cluster
solana config set --url mainnet-beta
solana config set --url devnet
solana config set --url localhost
solana config set --url testnet

# Shorthand versions
solana config set -um   # mainnet-beta
solana config set -ud   # devnet
solana config set -ul   # localhost
solana config set -ut   # testnet
```

Always check your cluster before deploying. Accidentally deploying to mainnet from a test config wastes real SOL.

#### Wallet Management

```bash
# Generate a new keypair at the default location
solana-keygen new

# Get your wallet address
solana address

# Check balance
solana balance
```

If a keypair already exists at `~/.config/solana/id.json`, the command will NOT overwrite it unless you pass `--force`.

#### Funding Your Wallet

```bash
# Switch to devnet first
solana config set -ud

# Request airdrop (currently limited to 5 SOL per request on devnet)
solana airdrop 2

# If rate limited, use the web faucet at https://faucet.solana.com
```

#### Local Validator

```bash
# Start a local validator in a separate terminal
solana-test-validator

# Switch CLI to local
solana config set -ul

# Airdrop on local (no limits)
solana airdrop 100
```

The local validator is the fastest way to iterate. `anchor test` starts and stops one for you automatically, but running it manually lets you inspect accounts and transaction logs between test runs.

---

## 2. Project Structure

### Creating a Project

```bash
anchor init my-project
cd my-project
```

### Default File Layout

```
my-project/
├── Anchor.toml                        # workspace and cluster config
├── Cargo.toml                         # Rust workspace manifest
├── package.json                       # JS/TS dependencies
├── tsconfig.json
├── .anchor/
│   └── program-logs/                  # transaction logs from last test run
├── app/                               # optional: frontend code
├── migrations/
│   └── deploy.js
├── programs/
│   └── my-project/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                 # module declarations + declare_id!
│           ├── constants.rs           # program-wide constants
│           ├── error.rs               # custom error codes
│           ├── instructions/
│           │   ├── mod.rs             # re-exports all instructions
│           │   └── initialize.rs      # one file per instruction
│           └── state/
│               └── mod.rs             # account data structs
├── target/
│   ├── deploy/
│   │   ├── my_project.so             # compiled program binary
│   │   └── my_project-keypair.json   # program keypair
│   ├── idl/
│   │   └── my_project.json           # generated IDL
│   └── types/
│       └── my_project.ts             # generated TypeScript types
└── tests/
    └── my-project.ts                  # TypeScript integration tests
```

### Template Options

```bash
# Default: modular structure (recommended for production)
anchor init my-project
anchor init --template multiple my-project

# Single file (easier for prototypes)
anchor init --template single my-project

# Rust test template instead of TypeScript
anchor init --test-template rust my-project

# Mollusk test library
anchor init --test-template mollusk my-project
```

The modular structure puts each instruction in its own file under `instructions/`, state definitions in `state/`, errors in `error.rs`, and constants in `constants.rs`. This is what you should use for anything beyond a quick experiment.

---

## 3. Core Macros

Anchor programs are built around four macros. Understanding what each one generates is important for debugging when things go wrong.

### `declare_id!`

Declares the on-chain address of the program. This must match the keypair in `target/deploy/your_program_name-keypair.json`.

```rust
use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");
```

On first `anchor build`, the actual program ID from the keypair is written here automatically.

When you clone a repository, the program ID in `declare_id!` will not match your locally generated keypair. Fix this with:

```bash
anchor keys sync
```

### `#[program]`

Marks the Rust module that contains all callable instruction handlers. Each public function inside is one instruction that clients can invoke.

```rust
#[program]
mod my_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, data: u64) -> Result<()> {
        ctx.accounts.new_account.data = data;
        msg!("Data set to: {}", data);
        Ok(())
    }

    pub fn update(ctx: Context<Update>, data: u64) -> Result<()> {
        ctx.accounts.existing_account.data = data;
        Ok(())
    }
}
```

Rules:

- Every instruction must return `Result<()>`
- The first parameter is always `Context<T>` where `T` is a struct with `#[derive(Accounts)]`
- Additional parameters after `ctx` are instruction arguments passed from the client
- `msg!()` writes to program logs (visible in transaction output)

### `#[derive(Accounts)]`

Applied to a struct to declare the accounts required by an instruction. Anchor validates all accounts automatically before executing the instruction body. Validation includes ownership checks, discriminator checks, signer checks, and all constraint expressions.

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    // Creates a new account on-chain, charges rent to signer
    #[account(init, payer = signer, space = 8 + 8)]
    pub new_account: Account<'info, NewAccount>,

    // Must have signed the transaction, will pay rent
    #[account(mut)]
    pub signer: Signer<'info>,

    // Required by the init constraint to create accounts
    pub system_program: Program<'info, System>,
}
```

Field naming is arbitrary but should be descriptive. Anchor generates a discriminator for each instruction (first 8 bytes of `sha256("global:<instruction_name>")`), which routes incoming transactions to the correct handler.

### `#[account]`

Applied to structs that define the shape of data stored in on-chain accounts owned by your program. Anchor automatically:

- Assigns your program as the owner when the account is created
- Writes an 8-byte discriminator as the first bytes of data (first 8 bytes of `sha256("account:<AccountName")`)
- Handles Borsh serialization and deserialization on every read/write

```rust
#[account]
pub struct NewAccount {
    pub data: u64,
}
```

The discriminator serves a critical security role: when Anchor deserializes an account, it checks the first 8 bytes against the expected discriminator. A mismatch means the wrong account type was passed in, and Anchor returns an error before your instruction logic runs. This prevents account substitution attacks.

#### Account Discriminator Details

```
Instruction discriminator: sha256("global:initialize")[0..8]
Account discriminator:     sha256("account:NewAccount")[0..8]
```

For example:

```
sha256("global:initialize") = af af 6d 1f 0d 98 9b ed ...
First 8 bytes as u8: [175, 175, 109, 31, 13, 152, 155, 237]
```

These are included in the IDL and handled automatically by the Anchor client.

---

## 4. Instruction Context

The `Context<T>` type is the first parameter of every instruction. It gives the instruction access to everything it needs beyond the explicit arguments.

```rust
pub struct Context<'a, 'b, 'c, 'info, T: Bumps> {
    pub program_id: &'a Pubkey,
    pub accounts: &'b mut T,
    pub remaining_accounts: &'c [AccountInfo<'info>],
    pub bumps: T::Bumps,
}
```

| Field                    | Description                                                |
| ------------------------ | ---------------------------------------------------------- |
| `ctx.accounts`           | The validated accounts struct (your `T`)                   |
| `ctx.program_id`         | This program's public key                                  |
| `ctx.bumps`              | Bump seeds for PDA accounts declared in the `T` struct     |
| `ctx.remaining_accounts` | Accounts passed in but not declared in `T` — use carefully |

Accessing fields:

```rust
pub fn my_instruction(ctx: Context<MyAccounts>, amount: u64) -> Result<()> {
    let account_data = &mut ctx.accounts.my_account;
    account_data.value = amount;

    let bump = ctx.bumps.pda_account;  // if pda_account uses seeds/bump constraints

    msg!("Program ID: {}", ctx.program_id);
    Ok(())
}
```

---

## 5. Account Types

These are the wrapper types used as field types in `#[derive(Accounts)]` structs.

### `Account<'info, T>`

The most common type. Checks that the account is owned by the current program and deserializes its data into `T`.

```rust
pub my_account: Account<'info, MyData>,
```

### `Signer<'info>`

Validates that this account signed the transaction. Does not deserialize any data.

```rust
pub authority: Signer<'info>,
```

### `SystemAccount<'info>`

Validates that the account is owned by the System Program. Used for wallets and uninitialized accounts.

```rust
pub recipient: SystemAccount<'info>,
```

### `Program<'info, T>`

Validates that the account is a specific program. Used when you need to include a program reference for CPIs.

```rust
use anchor_spl::token::Token;

pub system_program: Program<'info, System>,
pub token_program: Program<'info, Token>,
```

### `Interface<'info, T>`

Like `Program` but accepts any one of a set of programs. Used for Token Program vs Token Extensions Program compatibility.

```rust
use anchor_spl::token_interface::TokenInterface;

pub token_program: Interface<'info, TokenInterface>,
```

### `InterfaceAccount<'info, T>`

Like `Account` but accepts accounts owned by any program in the interface set. Used for mint and token accounts that may belong to either Token Program or Token Extensions Program.

```rust
use anchor_spl::token_interface::{Mint, TokenAccount};

pub mint: InterfaceAccount<'info, Mint>,
pub token_account: InterfaceAccount<'info, TokenAccount>,
```

### `AccountLoader<'info, T>`

Zero-copy deserialization. Used for large accounts. Does not copy data from the account into a heap struct — instead gives you a reference directly into the account bytes. See the Zero-Copy section for full details.

```rust
pub large_account: AccountLoader<'info, LargeData>,
```

### `UncheckedAccount<'info>`

No validation at all. You must document why it is safe with a `/// CHECK:` comment, or the compiler will reject it.

```rust
/// CHECK: This account is validated manually inside the instruction
pub arbitrary_account: UncheckedAccount<'info>,
```

Use this only when none of the other types fit, and always validate manually inside the instruction body.

### `Box<Account<'info, T>>`

Heap-allocates the account to save stack space. Solana's stack limit is 4KB. If you have many large accounts in one struct, boxing some of them prevents stack overflows.

```rust
pub large_account: Box<Account<'info, BigStruct>>,
```

### `Option<Account<'info, T>>`

Makes an account optional. If the client passes `None` (the system program address as a placeholder), the field will be `None` in the instruction body.

```rust
pub optional_account: Option<Account<'info, MyData>>,
```

Check it in the instruction:

```rust
if let Some(acct) = &ctx.accounts.optional_account {
    // use it
}
```

### `Sysvar<'info, T>`

Provides access to Solana system variables like the current clock time or rent information.

```rust
pub rent: Sysvar<'info, Rent>,
pub clock: Sysvar<'info, Clock>,
```

Access values:

```rust
let current_time = ctx.accounts.clock.unix_timestamp;
let rent_minimum = ctx.accounts.rent.minimum_balance(space);
```

### `Migration<'info, From, To>`

Handles schema migrations between account versions. On deserialization, the account must match the `From` type. On exit, it is serialized as `To`. Typically used with the `realloc` constraint.

```rust
#[account]
pub struct AccountV1 { pub data: u64 }

#[account]
pub struct AccountV2 { pub data: u64, pub new_field: u64 }

#[derive(Accounts)]
pub struct MigrateAccount<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        realloc = 8 + AccountV2::INIT_SPACE,
        realloc::payer = payer,
        realloc::zero = false
    )]
    pub my_account: Migration<'info, AccountV1, AccountV2>,
    pub system_program: Program<'info, System>,
}
```

Usage patterns:

```rust
// Explicit migration
let old_data = ctx.accounts.my_account.data;
ctx.accounts.my_account.migrate(AccountV2 {
    data: old_data,
    new_field: 42,
})?;

// Idempotent migration — migrates only if needed
let migrated = ctx.accounts.my_account.into_inner(AccountV2 {
    data: ctx.accounts.my_account.data,
    new_field: 0,
});

// Idempotent with mutable access
let migrated = ctx.accounts.my_account.into_inner_mut(AccountV2 {
    data: ctx.accounts.my_account.data,
    new_field: 0,
});
migrated.new_field = 42;
```

---

## 6. Account Constraints

Constraints are placed on account fields inside `#[derive(Accounts)]` structs using the `#[account(...)]` attribute. They define what conditions an account must meet to be valid for the instruction.

### Normal Constraints

#### `mut`

Marks the account as mutable. Anchor will persist state changes back to the account at the end of the instruction.

```rust
#[account(mut)]
pub my_account: Account<'info, MyData>,

// With custom error
#[account(mut @ MyError::AccountNotMutable)]
pub my_account: Account<'info, MyData>,
```

#### `signer`

Requires the account to have signed the transaction.

```rust
#[account(signer)]
pub authority: AccountInfo<'info>,
```

Usually you use `Signer<'info>` as the type instead, which implies the same check without needing the explicit constraint.

#### `init`

Creates a new account via CPI to the System Program. Allocates space, transfers ownership to this program, and sets the discriminator.

```rust
#[account(
    init,
    payer = signer,     // who pays for rent
    space = 8 + 64,     // total bytes (8 for discriminator + your data)
)]
pub new_account: Account<'info, MyData>,
```

`init` requires `System Program` to be present in the accounts struct.

#### `init_if_needed`

Same as `init` but skips creation if the account already exists. Requires enabling the `init-if-needed` feature in Cargo.toml.

```rust
#[account(
    init_if_needed,
    payer = signer,
    space = 8 + 64,
)]
pub account: Account<'info, MyData>,
```

```toml
[dependencies]
anchor-lang = { version = "0.32.1", features = ["init-if-needed"] }
```

#### `seeds` and `bump`

Validates that the account's address is a PDA derived from these seeds and the current program ID. Must be used together.

```rust
#[account(
    seeds = [b"vault", user.key().as_ref()],
    bump,
)]
pub vault: SystemAccount<'info>,
```

`seeds::program` overrides which program ID is used for derivation (only needed when checking PDAs from a different program):

```rust
#[account(
    seeds = [b"external"],
    bump,
    seeds::program = other_program.key(),
)]
pub external_pda: UncheckedAccount<'info>,
```

Providing a specific bump value instead of finding it:

```rust
#[account(
    seeds = [b"vault"],
    bump = stored_bump,  // stored_bump comes from another account field
)]
```

#### `has_one`

Checks that a field on the account struct matches the key of the corresponding field in the `Accounts` struct.

```rust
// This checks that escrow.authority == accounts.authority.key()
#[account(has_one = authority)]
pub escrow: Account<'info, EscrowAccount>,
pub authority: Signer<'info>,
```

With custom error:

```rust
#[account(has_one = authority @ MyError::WrongAuthority)]
```

#### `address`

Checks that the account's address matches a specific pubkey.

```rust
use anchor_lang::solana_program::sysvar;

#[account(address = sysvar::rent::ID)]
pub rent: AccountInfo<'info>,
```

#### `owner`

Checks that the account is owned by a specific program.

```rust
#[account(owner = token_program.key())]
pub token_account: UncheckedAccount<'info>,
```

#### `executable`

Checks that the account is a deployed program.

```rust
#[account(executable)]
pub some_program: UncheckedAccount<'info>,
```

#### `zero`

Checks that the account's discriminator has NOT been set (all zeros). Used as an alternative to `init` for accounts larger than 10240 bytes. The account must be created manually first via System Program before calling the instruction.

```rust
#[account(zero)]
pub large_account: AccountLoader<'info, LargeData>,
```

#### `close`

Closes the account by zeroing its data and sending all lamports to the target account. This reclaims rent SOL.

```rust
#[account(
    mut,
    close = signer,  // lamports go here
)]
pub account_to_close: Account<'info, MyData>,
```

#### `constraint`

A custom boolean expression. If it evaluates to false, the instruction fails.

```rust
#[account(
    constraint = some_account.value > 0,
)]
pub some_account: Account<'info, MyData>,

// With custom error
#[account(
    constraint = some_account.value > 0 @ MyError::ValueZero,
)]
```

#### `realloc`

Resizes an existing account's data. Useful for account migrations and adding new fields.

```rust
#[account(
    mut,
    realloc = 8 + 128,
    realloc::payer = signer,
    realloc::zero = true,   // zero out new bytes
)]
pub account: Account<'info, MyData>,
```

#### `dup`

By default, Anchor prevents passing the same mutable account twice to prevent accidental bugs. Use `dup` to explicitly allow it when intentional.

```rust
#[derive(Accounts)]
pub struct AllowDuplicate<'info> {
    #[account(mut)]
    pub account1: Account<'info, Counter>,
    #[account(mut, dup)]
    pub account2: Account<'info, Counter>,  // allowed to be same as account1
}
```

---

### SPL Token Constraints

These are shorthand constraints for creating and validating token-related accounts. They handle all the initialization details internally.

#### `mint::*`

```rust
#[account(
    init,
    payer = signer,
    mint::decimals = 6,
    mint::authority = signer.key(),
    mint::freeze_authority = signer.key(),  // optional
)]
pub mint: InterfaceAccount<'info, Mint>,
```

Add `seeds` and `bump` to create a PDA mint:

```rust
#[account(
    init,
    payer = signer,
    mint::decimals = 6,
    mint::authority = mint.key(),   // PDA is its own authority
    mint::freeze_authority = mint.key(),
    seeds = [b"mint"],
    bump,
)]
pub mint: InterfaceAccount<'info, Mint>,
```

#### `token::*`

For standard token accounts (non-ATA, or custom PDA token accounts):

```rust
// Token account at a keypair address
#[account(
    init,
    payer = signer,
    token::mint = mint,
    token::authority = signer,
    token::token_program = token_program,
)]
pub token_account: InterfaceAccount<'info, TokenAccount>,

// Token account at a PDA address
#[account(
    init,
    payer = signer,
    token::mint = mint,
    token::authority = token_account,  // PDA is its own authority
    token::token_program = token_program,
    seeds = [b"token"],
    bump,
)]
pub token_account: InterfaceAccount<'info, TokenAccount>,
```

When `token::authority` is a PDA (like the token account itself), your program can sign token transfers from it.

#### `associated_token::*`

For standard Associated Token Accounts (ATAs). Prefer this for user wallets.

```rust
#[account(
    init_if_needed,
    payer = signer,
    associated_token::mint = mint,
    associated_token::authority = signer,
    associated_token::token_program = token_program,
)]
pub token_account: InterfaceAccount<'info, TokenAccount>,
```

Also requires `AssociatedToken` program in accounts:

```rust
pub associated_token_program: Program<'info, AssociatedToken>,
```

#### Token Extensions Constraints

```rust
// Close authority extension
#[account(
    extensions::close_authority::authority = signer,
)]
pub mint: InterfaceAccount<'info, Mint>,

// Permanent delegate
#[account(
    extensions::permanent_delegate::delegate = delegate_account,
)]

// Transfer hook
#[account(
    extensions::transfer_hook::authority = signer,
    extensions::transfer_hook::program_id = hook_program,
)]

// Metadata pointer
#[account(
    extensions::metadata_pointer::authority = signer,
    extensions::metadata_pointer::metadata_address = mint,
)]

// Group pointer
#[account(
    extensions::group_pointer::authority = signer,
    extensions::group_pointer::group_address = group_account,
)]
```

---

### `#[instruction(...)]` Attribute

Makes instruction arguments available inside the accounts struct, useful for calculating dynamic space or seeding PDAs with instruction arguments.

```rust
#[program]
pub mod example {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, input: String) -> Result<()> {
        // ...
    }
}

#[derive(Accounts)]
#[instruction(input: String)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = signer,
        space = 8 + 4 + input.len(),  // dynamic space based on argument
    )]
    pub new_account: Account<'info, DataAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

Arguments must be listed in the same order as in the instruction handler. You may stop listing them once you no longer need them — but you cannot skip one in the middle.

---

## 7. Account Space Calculation

When using `init`, you must provide the total byte size of the account in the `space` constraint. **Always add 8 bytes for Anchor's discriminator.**

### Type Size Reference

| Rust Type       | Bytes                          | Notes                         |
| --------------- | ------------------------------ | ----------------------------- |
| `bool`          | 1                              |                               |
| `u8` / `i8`     | 1                              |                               |
| `u16` / `i16`   | 2                              |                               |
| `u32` / `i32`   | 4                              |                               |
| `u64` / `i64`   | 8                              |                               |
| `u128` / `i128` | 16                             |                               |
| `f32`           | 4                              | NaN will fail serialization   |
| `f64`           | 8                              | NaN will fail serialization   |
| `Pubkey`        | 32                             |                               |
| `bool`          | 1                              |                               |
| `[T; N]`        | `size_of::<T>() * N`           | fixed array                   |
| `Vec<T>`        | `4 + size_of::<T>() * max_len` | 4 bytes for the length prefix |
| `String`        | `4 + max_byte_length`          | 4 bytes for the length prefix |
| `Option<T>`     | `1 + size_of::<T>()`           | 1 byte for the Some/None tag  |
| `Enum`          | `1 + size of largest variant`  | 1 byte for the variant tag    |

Enum example:

```rust
// GameState::Won { winner: Pubkey } is the largest variant at 32 bytes
// Total: 1 + 32 = 33 bytes
pub enum GameState {
    Active,
    Tie,
    Won { winner: Pubkey },
}
```

### Manual Calculation Example

```rust
#[account]
pub struct PlayerState {
    pub owner: Pubkey,       // 32
    pub score: u64,          // 8
    pub level: u16,          // 2
    pub name: String,        // 4 + up to 50 bytes = 54
    pub items: Vec<u32>,     // 4 + up to 10 * 4 = 44
}

impl PlayerState {
    // 8 (discriminator) + 32 + 8 + 2 + 54 + 44 = 148
    pub const MAX_SIZE: usize = 32 + 8 + 2 + (4 + 50) + (4 + 10 * 4);
}

// In accounts struct:
#[account(init, payer = signer, space = 8 + PlayerState::MAX_SIZE)]
```

### InitSpace Macro (Recommended)

`#[derive(InitSpace)]` calculates `INIT_SPACE` automatically. It does not include the 8-byte discriminator, so you still add that manually.

```rust
#[account]
#[derive(InitSpace)]
pub struct PlayerState {
    pub owner: Pubkey,
    pub score: u64,
    pub level: u16,
    #[max_len(50)]
    pub name: String,
    #[max_len(10)]
    pub items: Vec<u32>,
}

// In accounts struct:
#[account(init, payer = signer, space = 8 + PlayerState::INIT_SPACE)]
```

The `#[max_len(N)]` attribute tells `InitSpace` the maximum element count for `Vec` and max byte length for `String`. For nested Vecs: `#[max_len(outer_len, inner_len)]`.

---

## 8. Program Derived Addresses (PDAs)

PDAs are deterministic addresses computed from seeds and a program ID. They have no corresponding private key, so only the program that owns them can "sign" for them during CPIs.

PDAs are the foundation of most non-trivial Solana program architecture. Every piece of program-owned state lives in an account whose address is typically a PDA.

### How PDA Derivation Works

```
PDA address = find_program_address(seeds, program_id)
```

The function hashes the seeds with the program ID and a bump value until it finds an address that falls off the Ed25519 curve (i.e., no valid private key exists for it). The bump is a single byte (255 down to 0) that makes this possible.

The "canonical bump" is the first valid bump found (starting from 255). Always use the canonical bump.

### Declaring PDAs in Account Structs

```rust
// No seeds (just the program ID determines the address)
#[account(seeds = [], bump)]
pub global_config: Account<'info, Config>,

// Static seed
#[account(
    seeds = [b"treasury"],
    bump,
)]
pub treasury: SystemAccount<'info>,

// Dynamic seed: include a user's pubkey to create per-user accounts
#[account(
    seeds = [b"user_profile", user.key().as_ref()],
    bump,
)]
pub user_profile: Account<'info, UserProfile>,

// Multiple seeds
#[account(
    seeds = [b"escrow", buyer.key().as_ref(), seller.key().as_ref()],
    bump,
)]
pub escrow: Account<'info, EscrowAccount>,
```

### Accessing the Bump

The bump for any PDA account declared with `seeds` + `bump` is available in `ctx.bumps`:

```rust
pub fn my_ix(ctx: Context<MyAccounts>) -> Result<()> {
    let bump = ctx.bumps.treasury;  // field name matches account field name
    // ...
}
```

### Storing the Bump

If you need to sign with a PDA later (e.g., in a separate instruction), store the bump in the account data on creation:

```rust
#[account]
pub struct VaultState {
    pub bump: u8,
    pub owner: Pubkey,
    pub amount: u64,
}

pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.bump = ctx.bumps.vault;
    vault.owner = ctx.accounts.owner.key();
    vault.amount = 0;
    Ok(())
}
```

### Deriving PDAs on the Client Side (TypeScript)

```typescript
const [pda, bump] = PublicKey.findProgramAddressSync(
  [Buffer.from("user_profile"), user.publicKey.toBuffer()],
  program.programId,
);
```

The seeds and their encoding must match exactly what the program uses. `b"user_profile"` in Rust is a UTF-8 byte slice, which corresponds to `Buffer.from("user_profile")` in JS.

### Creating PDA Accounts with `init`

```rust
#[account(
    init,
    payer = user,
    space = 8 + UserProfile::INIT_SPACE,
    seeds = [b"user_profile", user.key().as_ref()],
    bump,
)]
pub user_profile: Account<'info, UserProfile>,
```

This creates the account at the PDA address. Anchor derives the address, verifies it, creates the account via System Program CPI, and sets the discriminator — all automatically.

### PDA Seeds in the IDL

Seeds defined in constraints are included in the IDL. The Anchor TypeScript client can automatically resolve PDA addresses when you call instructions, so you often do not need to derive them manually on the client side.

---

## 9. Cross Program Invocations (CPIs)

CPIs allow your program to call instructions on other programs. This is how Solana composability works — programs interact by invoking each other's instructions, not by sharing state directly.

A CPI requires:

1. The program ID of the target program
2. The accounts required by the target instruction
3. Any instruction data / arguments

### `CpiContext`

Anchor's `CpiContext` bundles the program and accounts needed for a CPI call.

```rust
let cpi_context = CpiContext::new(
    target_program.to_account_info(),
    TargetAccounts {
        account_one: ctx.accounts.account_one.to_account_info(),
        account_two: ctx.accounts.account_two.to_account_info(),
    },
);
```

### Basic CPI: SOL Transfer via System Program

```rust
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("...");

#[program]
pub mod my_program {
    use super::*;

    pub fn send_sol(ctx: Context<SendSol>, amount: u64) -> Result<()> {
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.sender.to_account_info(),
                to: ctx.accounts.recipient.to_account_info(),
            },
        );
        transfer(cpi_context, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SendSol<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(mut)]
    pub recipient: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}
```

### CPI with PDA Signer

When the sender in a CPI is a PDA (not a wallet), the program must "sign" on its behalf using the PDA's seeds. Use `.with_signer()` on the `CpiContext`.

```rust
pub fn transfer_from_pda(ctx: Context<TransferFromPda>, amount: u64) -> Result<()> {
    // Build the signer seeds matching the PDA's derivation
    let seed = ctx.accounts.recipient.key();
    let bump_seed = ctx.bumps.pda_account;
    let signer_seeds: &[&[&[u8]]] = &[&[b"pda", seed.as_ref(), &[bump_seed]]];

    let cpi_context = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        Transfer {
            from: ctx.accounts.pda_account.to_account_info(),
            to: ctx.accounts.recipient.to_account_info(),
        },
    ).with_signer(signer_seeds);

    transfer(cpi_context, amount)?;
    Ok(())
}

#[derive(Accounts)]
pub struct TransferFromPda<'info> {
    #[account(
        mut,
        seeds = [b"pda", recipient.key().as_ref()],
        bump,
    )]
    pub pda_account: SystemAccount<'info>,
    #[account(mut)]
    pub recipient: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}
```

The Solana runtime validates that the seeds and caller program ID correctly derive the PDA before approving the signature.

### Signer Seeds Format

The type for signer seeds is `&[&[&[u8]]]`. This is:

- The outermost `&[...]` — a slice of PDAs (you can sign for multiple PDAs in one CPI)
- The middle `&[...]` — the individual seed components for one PDA
- The innermost `&[u8]` — one seed component (e.g., a string literal or a pubkey bytes slice)

```rust
// For a PDA with seeds [b"vault", user_key.as_ref()] and bump
let signer_seeds: &[&[&[u8]]] = &[&[
    b"vault",
    user_key.as_ref(),
    &[bump],
]];
```

---

## 10. Token Operations (SPL Tokens)

All examples in this section work with both the original Token Program and Token Extensions Program (Token 2022) because they use `token_interface` which is compatible with both.

Required imports:

```rust
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface,
    MintTo, TransferChecked, Burn, CloseAccount,
};
use anchor_spl::associated_token::AssociatedToken;
```

### What is a Mint Account

A mint account uniquely represents a token type on Solana. It stores:

- `mint_authority` — who can create new tokens (optional, can be None for fixed supply)
- `supply` — total tokens in existence
- `decimals` — how many decimal places (e.g., 6 means 1 token = 1,000,000 base units)
- `freeze_authority` — who can freeze token accounts (optional)

The USDC mint address on mainnet is `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`. Every USDC token account on Solana is associated with this one mint.

### What is a Token Account

A token account holds a user's balance of a specific token. It stores:

- `mint` — which token type it holds
- `owner` — who controls it (can transfer, burn, delegate)
- `amount` — current balance in base units
- `delegate` / `delegated_amount` — if transfer authority has been delegated

### What is an Associated Token Account (ATA)

An ATA is a token account whose address is deterministically derived from:

- The owner's wallet address
- The token mint address
- The token program ID (Token Program or Token Extensions)

This creates a standard "default token account" for a user per token. You can always find a user's USDC account from their wallet address + the USDC mint address.

### Create a Mint Account

Using a generated keypair:

```rust
#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 6,
        mint::authority = signer.key(),
        mint::freeze_authority = signer.key(),
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

Using a PDA as the mint address (recommended for program-controlled tokens):

```rust
#[derive(Accounts)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 6,
        mint::authority = mint.key(),    // PDA is its own authority
        mint::freeze_authority = mint.key(),
        seeds = [b"mint"],
        bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

Using a PDA as the mint authority means only your program can mint tokens, controlled by program logic rather than a wallet key.

### Create a Token Account

**Associated Token Account (for user wallets):**

```rust
use anchor_spl::associated_token::AssociatedToken;

#[derive(Accounts)]
pub struct CreateTokenAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
```

**PDA token account (for program-controlled escrows, vaults):**

```rust
#[derive(Accounts)]
pub struct CreateVaultTokenAccount<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        token::mint = mint,
        token::authority = vault_token_account,  // PDA owns itself
        token::token_program = token_program,
        seeds = [b"vault_token"],
        bump,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

### Mint Tokens

**With signer as mint authority:**

```rust
pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };
    let cpi_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    token_interface::mint_to(cpi_context, amount)?;
    Ok(())
}
```

**With PDA mint authority:**

```rust
pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[b"mint", &[ctx.bumps.mint]]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.mint.to_account_info(),  // PDA is the authority
    };
    let cpi_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    ).with_signer(signer_seeds);

    token_interface::mint_to(cpi_context, amount)?;
    Ok(())
}
```

### Transfer Tokens

Use `transfer_checked` (not the older `transfer`). It requires specifying the mint decimals, which prevents a class of precision attacks.

**With signer as token account owner:**

```rust
pub fn transfer_tokens(ctx: Context<TransferTokens>, amount: u64) -> Result<()> {
    let decimals = ctx.accounts.mint.decimals;

    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.sender_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };
    let cpi_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    );
    token_interface::transfer_checked(cpi_context, amount, decimals)?;
    Ok(())
}

#[derive(Accounts)]
pub struct TransferTokens<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
}
```

**With PDA as token account authority:**

```rust
pub fn transfer_from_vault(ctx: Context<TransferFromVault>) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[b"vault_token", &[ctx.bumps.vault_token_account]]];

    let amount = ctx.accounts.vault_token_account.amount;
    let decimals = ctx.accounts.mint.decimals;

    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: ctx.accounts.vault_token_account.to_account_info(),  // PDA signs
    };
    let cpi_context = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
    ).with_signer(signer_seeds);

    token_interface::transfer_checked(cpi_context, amount, decimals)?;
    Ok(())
}
```

---

## 11. Token Extensions (Token 2022)

The Token Extensions Program (Token 2022) adds optional functionality to mints and token accounts through extensions. Extensions must generally be configured during account creation. Only a few (`cpi-guard`, `memo-transfer`, `token-group`, `token-member`, `token-metadata`) can be added after the fact.

### Available Extensions

| Extension                  | Applied To    | Purpose                                  |
| -------------------------- | ------------- | ---------------------------------------- |
| `TransferFeeConfig`        | Mint          | Charge a fee on every transfer           |
| `MintCloseAuthority`       | Mint          | Allow the mint to be closed              |
| `DefaultAccountState`      | Mint          | Default state for new token accounts     |
| `NonTransferable`          | Mint          | Soulbound tokens — cannot be transferred |
| `InterestBearingConfig`    | Mint          | Tokens that accrue interest              |
| `PermanentDelegate`        | Mint          | Authority that can always transfer/burn  |
| `TransferHook`             | Mint          | Call a custom program on every transfer  |
| `MetadataPointer`          | Mint          | Point to metadata account                |
| `TokenMetadata`            | Mint          | Store metadata directly on mint          |
| `GroupPointer`             | Mint          | Point to group config                    |
| `TokenGroup`               | Mint          | Group configuration                      |
| `ImmutableOwner`           | Token Account | Owner cannot be changed                  |
| `MemoTransfer`             | Token Account | Require memo on incoming transfers       |
| `CpiGuard`                 | Token Account | Prevent certain CPI operations           |
| `ConfidentialTransferMint` | Mint          | Encrypted transfer amounts               |
| `ScaledUiAmount`           | Mint          | Display scaling factor                   |
| `Pausable`                 | Mint          | Pause all token operations               |

Some extensions are incompatible. For example, `NonTransferable` and `TransferFeeConfig` conflict because non-transferable tokens cannot have transfer fees.

The `anchor-spl` crate's `token_2022_extensions` module provides helper functions for working with extensions. Not all instructions are fully implemented — for missing ones you may need to build the CPI manually. Examples for all extensions are available in the Anchor program examples repository.

---

## 12. Custom Errors

Define program-specific errors with the `#[error_code]` macro. Error codes start at 6000. The `#[msg]` attribute provides a human-readable message included in program logs and client error responses.

```rust
#[error_code]
pub enum MyError {
    #[msg("Amount must be greater than zero")]
    AmountZero,
    #[msg("Amount exceeds maximum allowed")]
    AmountTooLarge,
    #[msg("Only the authority can perform this action")]
    Unauthorized,
}
```

### Error Code Numbering

| Range   | Category                                      |
| ------- | --------------------------------------------- |
| >= 100  | Instruction errors                            |
| >= 1000 | IDL errors                                    |
| >= 2000 | Constraint errors                             |
| >= 3000 | Account errors                                |
| >= 4100 | Misc errors                                   |
| = 5000  | Deprecated                                    |
| >= 6000 | Custom program errors (your codes start here) |

### Using Errors in Instructions

```rust
// Return an error explicitly
pub fn set_amount(ctx: Context<SetAmount>, amount: u64) -> Result<()> {
    if amount == 0 {
        return err!(MyError::AmountZero);
    }
    ctx.accounts.state.amount = amount;
    Ok(())
}

// require! is the idiomatic shorthand
pub fn set_amount(ctx: Context<SetAmount>, amount: u64) -> Result<()> {
    require!(amount > 0, MyError::AmountZero);
    require!(amount <= 1_000_000, MyError::AmountTooLarge);
    ctx.accounts.state.amount = amount;
    Ok(())
}
```

### Require Macro Variants

| Macro                            | Checks                       |
| -------------------------------- | ---------------------------- |
| `require!(condition, error)`     | condition is true            |
| `require_eq!(a, b, error)`       | `a == b` (non-pubkey values) |
| `require_neq!(a, b, error)`      | `a != b` (non-pubkey values) |
| `require_keys_eq!(a, b, error)`  | pubkey `a == b`              |
| `require_keys_neq!(a, b, error)` | pubkey `a != b`              |
| `require_gt!(a, b, error)`       | `a > b`                      |
| `require_gte!(a, b, error)`      | `a >= b`                     |

### Error Response Structure (TypeScript Client)

When a custom error is returned, the Anchor client provides structured error info:

```json
{
  "errorLogs": [
    "Program log: AnchorError thrown in programs/my-program/src/lib.rs:15. Error Code: AmountTooLarge. Error Number: 6001. Error Message: Amount exceeds maximum allowed."
  ],
  "error": {
    "errorCode": { "code": "AmountTooLarge", "number": 6001 },
    "errorMessage": "Amount exceeds maximum allowed",
    "origin": { "file": "programs/my-program/src/lib.rs", "line": 15 }
  }
}
```

---

## 13. Events

Events allow programs to emit structured data that clients can subscribe to or fetch from transaction logs.

### Define an Event

```rust
#[event]
pub struct TransferEvent {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}
```

### emit! (via Program Logs)

Emits event data encoded as base64 in program logs via `sol_log_data()`. The log line is prefixed with `Program data:`.

```rust
pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
    // ... transfer logic ...

    emit!(TransferEvent {
        from: ctx.accounts.sender.key(),
        to: ctx.accounts.recipient.key(),
        amount,
        timestamp: Clock::get()?.unix_timestamp,
    });
    Ok(())
}
```

Client subscription (TypeScript):

```typescript
const listener = program.addEventListener("TransferEvent", (event) => {
  console.log("Transfer:", event);
});

// Later: remove listener
await program.removeEventListener(listener);
```

**Limitation:** Some RPC providers truncate program logs, which can cause events to be lost.

### emit_cpi! (via CPI)

An alternative that embeds event data in a CPI instruction's data field instead of logs. More reliable than `emit!` but costs additional compute units.

Enable the feature:

```toml
[dependencies]
anchor-lang = { version = "0.32.1", features = ["event-cpi"] }
```

Add `#[event_cpi]` to the accounts struct:

```rust
#[event_cpi]
#[derive(Accounts)]
pub struct Transfer<'info> {
    // ... your accounts
}
```

Emit from instruction:

```rust
emit_cpi!(TransferEvent {
    from: ctx.accounts.sender.key(),
    to: ctx.accounts.recipient.key(),
    amount,
    timestamp: Clock::get()?.unix_timestamp,
});
```

Fetch event data from client (TypeScript):

```typescript
const transactionData = await program.provider.connection.getTransaction(
  transactionSignature,
  { commitment: "confirmed" },
);

const eventIx = transactionData.meta.innerInstructions[0].instructions[0];
const rawData = anchor.utils.bytes.bs58.decode(eventIx.data);
const base64Data = anchor.utils.bytes.base64.encode(rawData.subarray(8));
const event = program.coder.events.decode(base64Data);
```

Note: `emit_cpi!` events cannot be subscribed to in real time. You must fetch the full transaction and decode manually.

For production-grade event infrastructure, consider Triton or Helius geyser gRPC services.

---

## 14. Zero-Copy Deserialization

Standard `Account<T>` copies account data from the account into a heap-allocated struct on deserialization. This limits account size (heap is 32KB, stack is 4KB) and costs significant compute units.

`AccountLoader<T>` with `#[account(zero_copy)]` instead gives you a reference directly into the account's raw bytes. No copying, no heap allocation.

### When to Use Zero-Copy

| Account Size | `Account<T>` | `AccountLoader<T>` | Improvement |
| ------------ | ------------ | ------------------ | ----------- |
| 1 KB         | ~8,000 CU    | ~1,500 CU          | 81%         |
| 10 KB        | ~50,000 CU   | ~5,000 CU          | 90%         |
| 100 KB       | Not possible | ~12,000 CU         | Enabled     |
| 1 MB         | Not possible | ~25,000 CU         | Enabled     |

Use zero-copy for:

- Accounts larger than ~1KB
- Order books, event queues, large arrays
- High-frequency operations
- Programs near compute unit limits

Do not use zero-copy for:

- Small, simple accounts
- Accounts with `Vec`, `String`, `HashMap` (these are not valid inside zero-copy structs)
- Accounts that change schema frequently

### Setting Up

```toml
[dependencies]
bytemuck = { version = "1.20.0", features = ["min_const_generics"] }
anchor-lang = "0.32.1"
```

### Define a Zero-Copy Account

```rust
#[account(zero_copy)]
pub struct LargeData {
    pub authority: Pubkey,     // 32 bytes
    pub values: [u64; 1000],   // 8000 bytes — must be fixed-size
    pub count: u64,
}
```

All fields must be `Copy` types. `Vec`, `String`, `HashMap` are NOT allowed — use fixed-size arrays instead.

`#[account(zero_copy)]` implicitly derives:

- `Copy`, `Clone`
- `bytemuck::Zeroable`
- `bytemuck::Pod`
- `#[repr(C)]`

### Use AccountLoader in Account Struct

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<LargeData>(),
    )]
    pub data_account: AccountLoader<'info, LargeData>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

The `init` constraint with `AccountLoader` is limited to 10240 bytes due to CPI limitations (System Program). For larger accounts, use `zero` constraint and create the account manually first.

### Loading Account Data

```rust
// First initialization — sets discriminator
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let account = &mut ctx.accounts.data_account.load_init()?;
    account.count = 0;
    account.authority = ctx.accounts.payer.key();
    Ok(())
}

// Subsequent writes — use load_mut
pub fn update(ctx: Context<Update>) -> Result<()> {
    let account = &mut ctx.accounts.data_account.load_mut()?;
    account.count += 1;
    Ok(())
}

// Read only — use load
pub fn read(ctx: Context<Read>) -> Result<()> {
    let account = &ctx.accounts.data_account.load()?;
    msg!("Count: {}", account.count);
    Ok(())
}
```

`load_init` must only be called once — it sets the discriminator. All subsequent calls should use `load_mut` or `load`.

### Accounts Larger Than 10240 Bytes

```rust
// Use zero constraint instead of init
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(zero)]
    pub data_account: AccountLoader<'info, HugeData>,
}

#[account(zero_copy)]
pub struct HugeData {
    pub data: [u8; 10_485_752],  // Maximum: 10MB - 8 bytes discriminator
}
```

The client must create the account first using a direct System Program call, then call your `initialize` instruction separately.

### Nested Zero-Copy Types

For struct types used as fields inside a zero-copy account, use `#[zero_copy]` (without `account`):

```rust
#[account(zero_copy)]
pub struct OrderBook {
    pub market: Pubkey,
    pub bid_count: u32,
    pub ask_count: u32,
    pub bids: [Order; 1000],
    pub asks: [Order; 1000],
}

#[zero_copy]
pub struct Order {
    pub trader: Pubkey,
    pub price: u64,
    pub size: u64,
    pub timestamp: i64,
}
```

### Separate Types for Instruction Parameters

Zero-copy types cannot derive `AnchorSerialize`/`AnchorDeserialize`, so they cannot be used as instruction arguments. Define a separate DTO type:

```rust
#[zero_copy]
pub struct Event {
    pub from: Pubkey,
    pub data: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct EventParams {
    pub from: Pubkey,
    pub data: u64,
}

impl From<EventParams> for Event {
    fn from(p: EventParams) -> Self {
        Event { from: p.from, data: p.data }
    }
}
```

---

## 15. Program IDL File

The IDL (Interface Description Language) is a JSON file generated by `anchor build` at `target/idl/<program-name>.json`. It describes every instruction, account type, error, event, and type in the program.

The IDL enables:

- TypeScript client type generation (`target/types/<program-name>.ts`)
- The `declare_program!` macro for dependency-free CPI and client code
- Third-party tools to generate clients in any language
- On-chain storage via `anchor idl init` for public discoverability

### IDL Structure

```json
{
  "address": "...",
  "metadata": { "name": "my_program", "version": "0.1.0" },
  "instructions": [
    {
      "name": "initialize",
      "discriminator": [175, 175, 109, 31, 13, 152, 155, 237],
      "accounts": [
        { "name": "newAccount", "writable": true, "signer": true },
        { "name": "signer", "writable": true, "signer": true },
        { "name": "systemProgram", "address": "11111111111111111111111111111111" }
      ],
      "args": [
        { "name": "data", "type": "u64" }
      ]
    }
  ],
  "accounts": [
    {
      "name": "NewAccount",
      "discriminator": [...]
    }
  ],
  "types": [...],
  "errors": [...]
}
```

### Storing the IDL On-Chain

```bash
# Store IDL on-chain (makes program publicly introspectable)
anchor idl init -f target/idl/my_program.json <program-id>

# Update after redeployment
anchor idl upgrade <program-id> -f target/idl/my_program.json

# Fetch from chain
anchor idl fetch -o fetched.json <program-id>

# Transfer IDL authority
anchor idl set-authority -n <new-authority> -p <program-id>

# Remove authority (freeze IDL permanently)
anchor idl erase-authority -p <program-id>
```

---

## 16. `declare_program!` Macro

`declare_program!` generates a complete set of Rust modules from an IDL file. This allows you to interact with any Anchor program without adding it as a Rust dependency — you only need its IDL JSON file.

Place the IDL in a directory named `idls/` relative to your crate. The `idls/` directory can be at any level of your project structure.

```
project/
├── idls/
│   └── example.json
├── programs/
│   └── caller/
│       └── src/lib.rs
```

In your code:

```rust
declare_program!(example);  // looks for idls/example.json
```

### Generated Modules

| Module                      | Contents                                              |
| --------------------------- | ----------------------------------------------------- |
| `example::cpi`              | Helper functions for making CPIs to the program       |
| `example::client::accounts` | Account structs for building client-side transactions |
| `example::client::args`     | Argument structs for each instruction                 |
| `example::accounts`         | Account data types (deserialization)                  |
| `example::program`          | Program ID constant and program type                  |
| `example::constants`        | Program constants                                     |
| `example::events`           | Event types                                           |
| `example::types`            | Custom types defined in the program                   |
| `example::errors`           | Error codes                                           |

### On-Chain CPI Using `declare_program!`

Caller program that invokes another program's instructions:

```rust
use anchor_lang::prelude::*;

declare_id!("...");
declare_program!(example);

use example::{
    accounts::Counter,
    cpi::{self, accounts::{Increment, Initialize}},
    program::Example,
};

#[program]
pub mod caller {
    use super::*;

    pub fn call_increment(ctx: Context<CallIncrement>) -> Result<()> {
        let cpi_ctx = CpiContext::new(
            ctx.accounts.example_program.to_account_info(),
            Increment {
                counter: ctx.accounts.counter.to_account_info(),
            },
        );
        cpi::increment(cpi_ctx)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CallIncrement<'info> {
    #[account(mut)]
    pub counter: Account<'info, Counter>,
    pub example_program: Program<'info, Example>,
}
```

### Off-Chain Rust Client

```rust
use anchor_client::{Client, Cluster, solana_sdk::*, solana_signer::Signer};
use anchor_lang::prelude::*;
use std::rc::Rc;

declare_program!(example);
use example::{accounts::Counter, client::accounts, client::args};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let payer = Keypair::new();
    let provider = Client::new_with_options(
        Cluster::Localnet,
        Rc::new(payer),
        CommitmentConfig::confirmed(),
    );
    let program = provider.program(example::ID)?;

    let counter = Keypair::new();

    let signature = program
        .request()
        .accounts(accounts::Initialize {
            counter: counter.pubkey(),
            payer: program.payer(),
            system_program: system_program::ID,
        })
        .args(args::Initialize)
        .signer(&counter)
        .send()
        .await?;

    let counter_account: Counter = program.account::<Counter>(counter.pubkey()).await?;
    println!("Counter: {}", counter_account.count);
    Ok(())
}
```

---

## 17. Anchor.toml Configuration

`Anchor.toml` is the workspace configuration file. It controls deployment, testing, and toolchain settings.

### `[provider]` (required)

```toml
[provider]
cluster = "Localnet"                    # Localnet | Devnet | Mainnet
wallet = "~/.config/solana/id.json"     # keypair for signing and paying
```

### `[scripts]` (required for testing)

```toml
[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

### `[programs.<cluster>]`

```toml
[programs.localnet]
my_program = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"

[programs.devnet]
my_program = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
```

### `[features]`

```toml
[features]
resolution = true     # IDL account resolution, default true
skip-lint = false
```

### `[workspace]`

```toml
[workspace]
# Copy IDL TypeScript types to frontend after build
types = "app/src/idl/"

# Explicitly specify which programs are in the workspace
members = [
    "programs/*",
    "other_programs/special_program",
]

# Exclude specific programs
exclude = [
    "programs/deprecated_program",
]
```

### `[toolchain]`

```toml
[toolchain]
anchor_version = "0.32.1"       # anchor-cli version (requires AVM)
solana_version = "2.3.0"        # Solana tools version
package_manager = "yarn"        # npm | yarn | pnpm | bun (default: yarn)
```

### `[test]`

```toml
[test]
startup_wait = 10000    # milliseconds to wait for test validator to start
upgradeable = true      # deploy test program as upgradeable

# Load programs at genesis during tests
[[test.genesis]]
address = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"
program = "dex.so"
upgradeable = true
```

### `[test.validator]`

```toml
[test.validator]
url = "https://api.mainnet-beta.solana.com"  # source for cloned accounts
warp_slot = 1337
rpc_port = 8899
slots_per_epoch = 32

# Clone accounts from mainnet into local test validator
[[test.validator.clone]]
address = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"   # USDC mint

# Load accounts from JSON file
[[test.validator.account]]
address = "Ev8WSPQsGb4wfjybqff5eZNcS3n6HaMsBkMk9suAiuM"
filename = "fixtures/my_account.json"
```

### `[hooks]`

```toml
[hooks]
pre-build = "cargo fmt"
post-build = "echo Build complete"
pre-test = ["echo Running tests", "solana-keygen new --no-passphrase --force -o test-keypair.json"]
post-test = "rm test-keypair.json"
```

---

## 18. Anchor CLI Reference

```
anchor <SUBCOMMAND>
```

### `anchor init`

```bash
anchor init <project-name>
anchor init --template multiple <n>   # modular (default, recommended)
anchor init --template single <n>     # single lib.rs
anchor init --test-template rust <n>  # Rust tests
anchor init --test-template mollusk <n>
```

### `anchor build`

```bash
anchor build                    # compile all programs, generate IDL
anchor build --verifiable       # build inside Docker for deterministic output
anchor build -- --features my-feature   # pass args to cargo build-sbf
```

### `anchor test`

```bash
anchor test                            # build + deploy + test (starts local validator)
anchor test --skip-local-validator     # test against already-running validator
anchor test --skip-build               # skip build, test current binary
anchor test --skip-deploy              # skip deploy, test currently deployed program
```

Transaction logs are written to `.anchor/program-logs/<address>.<name>.log`.

### `anchor deploy`

```bash
anchor deploy           # deploy to cluster in Anchor.toml
```

This is different from `solana program deploy` — `anchor deploy` uses program IDs defined in `Anchor.toml` and generates a new program address each time if none is defined.

### `anchor upgrade`

```bash
anchor upgrade target/deploy/my_program.so --program-id <id>
```

Uses the upgradeable BPF loader to update the on-chain binary without changing the program ID.

### `anchor verify`

```bash
anchor verify <program-id>
```

Verifies that the on-chain bytecode matches the locally compiled binary (requires a verifiable build).

### `anchor keys`

```bash
anchor keys list          # list all program keypair public keys
anchor keys sync          # sync declare_id! values with actual keypairs
```

### `anchor idl`

```bash
anchor idl build                                    # generate IDL using compilation method
anchor idl init -f target/idl/prog.json <program-id>   # store IDL on chain
anchor idl upgrade <program-id> -f target/idl/prog.json
anchor idl fetch -o out.json <program-id>
anchor idl authority <program-id>
anchor idl erase-authority -p <program-id>
anchor idl set-authority -n <new-auth> -p <program-id>
```

### `anchor account`

```bash
anchor account my-program.EscrowAccount <pubkey>
anchor account my-program.EscrowAccount <pubkey> --idl path/to/idl.json
```

Fetches and deserializes an on-chain account into JSON. `program-name` is the kebab-case folder name under `programs/`. `AccountTypeName` is PascalCase.

### `anchor new`

```bash
anchor new <program-name>                    # add program to workspace
anchor new --template single <program-name>
```

### `anchor expand`

```bash
anchor expand   # show macro-expanded code (useful for debugging macros)
```

### `anchor cluster list`

```bash
anchor cluster list   # print RPC endpoint URLs for all clusters
```

---

## 19. Anchor Version Manager (AVM)

```bash
avm install latest                    # install latest Anchor
avm install 0.32.1                    # install specific version
avm install cfe82aa682138f7c6c...     # install from commit hash
avm use latest
avm use 0.32.1
avm list                              # list available versions
avm uninstall 0.30.0                  # remove a version
```

The `use` command must be run explicitly after `install`. AVM does not activate a version automatically.

### Shell Completions

**Bash:**

```bash
mkdir -p $HOME/.local/share/bash-completion/completions
anchor completions bash > $HOME/.local/share/bash-completion/completions/anchor
avm completions bash > $HOME/.local/share/bash-completion/completions/avm
exec bash
```

**Zsh:**

```bash
anchor completions zsh | sudo tee /usr/local/share/zsh/site-functions/_anchor
avm completions zsh | sudo tee /usr/local/share/zsh/site-functions/_avm
exec zsh
```

**Fish:**

```bash
mkdir -p $HOME/.config/fish/completions
anchor completions fish > $HOME/.config/fish/completions/anchor.fish
avm completions fish > $HOME/.config/fish/completions/avm.fish
```

---

## 20. Testing

### TypeScript Tests (Default)

The default test file at `tests/my-project.ts` uses Mocha + Anchor's TypeScript client.

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MyProject } from "../target/types/my_project";
import { assert } from "chai";

describe("my-project", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.MyProject as Program<MyProject>;

  it("initialize", async () => {
    const newAccount = anchor.web3.Keypair.generate();

    const txHash = await program.methods
      .initialize(new anchor.BN(42))
      .accounts({
        newAccount: newAccount.publicKey,
        signer: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([newAccount])
      .rpc();

    await provider.connection.confirmTransaction(txHash);

    const accountData = await program.account.newAccount.fetch(
      newAccount.publicKey,
    );
    assert(accountData.data.eq(new anchor.BN(42)));
  });
});
```

Run tests:

```bash
anchor test
```

### LiteSVM — Rust Unit Tests (Recommended)

LiteSVM is a fast, in-process Solana VM. It is far faster to compile and run than `solana-test-validator`. Use it for unit tests and integration tests of your program logic.

```toml
[dev-dependencies]
litesvm = "*"
```

Basic usage:

```rust
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_message::Message;
use solana_system_interface::instruction::transfer;

#[test]
fn test_transfer() {
    let from = Keypair::new();
    let to = Keypair::new().pubkey();

    let mut svm = LiteSVM::new();
    svm.airdrop(&from.pubkey(), 10_000).unwrap();

    let ix = transfer(&from.pubkey(), &to, 1_000);
    let tx = Transaction::new(
        &[&from],
        Message::new(&[ix], Some(&from.pubkey())),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();
    assert_eq!(svm.get_account(&to).unwrap().lamports, 1_000);
}
```

Loading a compiled program:

```rust
let program_id = pubkey!("...");
let bytes = include_bytes!("../../target/deploy/my_program.so");
svm.add_program(program_id, bytes);
```

Pulling a program from mainnet for testing:

```bash
solana program dump <program-id> program.so
```

### LiteSVM — Time Travel

Many programs use `Clock::get()`. LiteSVM lets you override the clock:

```rust
use solana_clock::Clock;

let mut clock = svm.get_sysvar::<Clock>();
clock.unix_timestamp = 1_700_000_000;  // set to a specific time
svm.set_sysvar::<Clock>(&clock);

// Jump to a future slot
svm.warp_to_slot(1000);
```

### LiteSVM — Arbitrary Account State

Inject any account state without needing actual keypairs or on-chain transactions:

```rust
use solana_account::Account;

svm.set_account(
    address,
    Account {
        lamports: 1_000_000_000,
        data: my_serialized_data.to_vec(),
        owner: my_program_id,
        executable: false,
        rent_epoch: 0,
    },
).unwrap();
```

This is powerful for testing: inject a USDC token account with a large balance without needing the USDC mint keypair.

### When to Use `solana-test-validator`

Use `solana-test-validator` (via `anchor test`) instead of LiteSVM when:

- You need to test RPC method behavior
- You need real validator behaviour (stake, vote transactions)
- You are testing the interaction between your program and a real on-chain program in a more realistic environment

For all other cases, LiteSVM is faster and easier.

---

## 21. Verifiable Builds

Two developers building the same program may get different binaries due to machine-specific compiler output. Verifiable builds use a pinned Docker image to produce a deterministic binary.

### Build Verifiably

```bash
# Run from within the program's directory (contains Cargo.toml)
cd programs/my-program
anchor build --verifiable
```

### Verify an On-Chain Program

```bash
anchor verify -p <lib-name> <program-id>
```

The `lib-name` is the `name` field in the program's `Cargo.toml`. If the program has an IDL on-chain, Anchor verifies that too.

### Docker Images

Anchor provides Docker images for each release:

```bash
docker pull solanafoundation/anchor:v0.32.1
```

If you exit a verifiable build prematurely, a Docker container may keep running. Remove it with:

```bash
docker rm -f anchor-program
```

---

## 22. Common Pitfalls and Security Notes

### Missing 8-byte Discriminator

```rust
// Wrong
#[account(init, payer = signer, space = std::mem::size_of::<MyData>())]

// Correct
#[account(init, payer = signer, space = 8 + std::mem::size_of::<MyData>())]
```

Every Anchor account requires 8 bytes for the discriminator. Forgetting this causes account creation to fail or results in an undersized account.

### Wrong Cluster on Deploy

The default cluster in `Anchor.toml` is `Localnet`. Before deploying to devnet or mainnet, explicitly change:

```toml
[provider]
cluster = "Devnet"   # or "Mainnet"
```

Deploying to the wrong cluster is a common mistake that wastes SOL on mainnet.

### Closed Program ID Cannot Be Reused

Once you close a program with `solana program close <program-id>`, that program ID cannot be used to deploy again. If you want to reclaim rent but may redeploy, keep a copy of the keypair and just keep the program.

### Zero-Copy with Dynamic Types

```rust
// WRONG — Vec and String are not valid inside zero_copy
#[account(zero_copy)]
pub struct Bad {
    pub items: Vec<u64>,
    pub name: String,
}

// Correct — fixed-size only
#[account(zero_copy)]
pub struct Good {
    pub items: [u64; 100],
    pub name: [u8; 32],
}
```

### load_init vs load_mut Confusion

```rust
// First and only time — sets discriminator
let acc = &mut ctx.accounts.data.load_init()?;

// All subsequent writes
let acc = &mut ctx.accounts.data.load_mut()?;
```

Calling `load_init` twice on an already-initialized account will fail. Calling `load_mut` on an uninitialized account will fail. Know which one applies to your instruction.

### Array Index Panics

Zero-copy accounts often hold large arrays. Always validate indices:

```rust
require!(
    (index as usize) < account.items.len(),
    MyError::IndexOutOfBounds
);
account.items[index as usize] = value;
```

### Anchor's Duplicate Account Protection

Anchor prevents passing the same mutable account twice by default. If you genuinely need this, use `dup`. If you are seeing unexpected errors about duplicate accounts, check whether your client is passing the same address in multiple fields.

### Sealevel Security Patterns

Anchor prevents many attack vectors automatically. Be aware of what it handles vs what you must handle:

| Attack                   | Anchor Handles?                                |
| ------------------------ | ---------------------------------------------- |
| Account ownership check  | Yes — `Account<T>` checks owner                |
| Discriminator check      | Yes — automatic on deserialization             |
| Signer check             | Yes — `Signer<'info>` and `#[account(signer)]` |
| Mutable account tracking | Yes — `#[account(mut)]`                        |
| PDA address check        | Yes — `seeds` + `bump` constraints             |
| `has_one` field matching | Yes — `#[account(has_one)]`                    |
| Arbitrary logic checks   | No — use `require!` and `constraint`           |
| Reentrancy               | Partial — no recursive CPIs by default         |

A comprehensive list of Solana-specific attacks with insecure/secure/recommended examples is maintained in the Anchor repository under Sealevel Attacks.

---

## 23. Dependency Reference

### Cargo.toml

```toml
[dependencies]
# Core Anchor
anchor-lang = "0.32.1"

# With optional features
anchor-lang = { version = "0.32.1", features = ["init-if-needed"] }
anchor-lang = { version = "0.32.1", features = ["event-cpi"] }

# SPL token integration
anchor-spl = "0.32.1"

# Zero-copy support
bytemuck = { version = "1.20.0", features = ["min_const_generics"] }

# Off-chain Rust client
anchor-client = { version = "0.32.1", features = ["async"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
# Fast in-process test VM (preferred)
litesvm = "*"
```

### Program Client Cargo.toml (off-chain binary)

```toml
[package]
name = "my-client"
version = "0.1.0"
edition = "2021"

[dependencies]
anchor-client = { version = "0.32.1", features = ["async"] }
anchor-lang = "0.32.1"
anyhow = "1.0.93"
tokio = { version = "1.0", features = ["full"] }
```

### Package.json (TypeScript tests and client)

```json
{
  "dependencies": {
    "@coral-xyz/anchor": "^0.32.1"
  },
  "devDependencies": {
    "@types/mocha": "^9.0.0",
    "chai": "^4.3.4",
    "mocha": "^9.0.3",
    "ts-mocha": "^10.0.0",
    "typescript": "^4.9.4"
  }
}
```

---

## Quick Reference Card

### Build and Deploy Workflow

```bash
anchor init my-project            # 1. scaffold
cd my-project
# write your program in programs/my-project/src/
anchor build                      # 2. compile + generate IDL
anchor keys sync                  # 3. sync program ID if needed
anchor test                       # 4. test on local validator

# When ready for devnet:
# Change Anchor.toml cluster to Devnet
anchor deploy                     # 5. deploy
anchor idl init -f target/idl/my_project.json <program-id>   # 6. publish IDL
```

### Instruction Template

```rust
pub fn my_instruction(ctx: Context<MyAccounts>, arg1: u64, arg2: String) -> Result<()> {
    require!(arg1 > 0, MyError::InvalidArg);

    let account = &mut ctx.accounts.my_account;
    account.value = arg1;

    emit!(MyEvent { value: arg1 });

    msg!("Instruction complete: {}", arg1);
    Ok(())
}
```

### Accounts Struct Template

```rust
#[derive(Accounts)]
#[instruction(arg1: u64)]  // only needed if you use instruction args in constraints
pub struct MyAccounts<'info> {
    #[account(
        init,
        payer = signer,
        space = 8 + MyData::INIT_SPACE,
        seeds = [b"my_account", signer.key().as_ref()],
        bump,
    )]
    pub my_account: Account<'info, MyData>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
```

### Data Account Template

```rust
#[account]
#[derive(InitSpace)]
pub struct MyData {
    pub owner: Pubkey,          // 32
    pub value: u64,             // 8
    pub bump: u8,               // 1
    #[max_len(50)]
    pub name: String,           // 4 + 50
}
```

### Error Definition Template

```rust
#[error_code]
pub enum MyError {
    #[msg("Value must be greater than zero")]
    InvalidArg,
    #[msg("Unauthorized")]
    Unauthorized,
}
```
