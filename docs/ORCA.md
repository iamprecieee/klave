i've been looking into this:

Overview of Orca's SDKs
Overview of Orca Whirlpools SDK suite

Orca provides a range of SDKs that cater to different levels of development needs for interacting with the Whirlpool Program on Solana and Eclipse. Whether you are managing liquidity, building applications that require pool infrastructure, or building automation tools that interact with the program, our SDKs cover a spectrum of functionality from low-level granular control to high-level abstractions.

What follows is an overview of our SDK suite, distinguishing between the various layers of the SDKs, and explaining their intended purposes and relationships.

1. High-Level SDKs
   The High-Level SDKs are our top recommendation for anyone who wants to integrate with the Whirlpool Program. These SDKs abstract many of the underlying complexities, such as tick array management, and makes managing pools and positions, and executing swaps much simpler. It is suitable for developers who need efficient, high-level functionalities and want to minimize manual configuration and management.

Rust: orca_whirlpools
Compatible with Solana SDK versions ^1.18.0 but <3.0.0. By default, Cargo will install the latest version of Solana SDK ^v2. This can cause dependency issues when using older versions. To solve this you can apply a lockfile patch with the following command:
cargo update solana-program:<current-version> --precise <required-version>

You might also have to apply lockfile patches for solana-program, solana-client, solana-account-decoder, spl-token, spl-memo, spl-token-2022, spl-associated-token-account.
TypeScript Kit: @orca-so/whirlpools
Compatible with Solana Kit
Typescript Legacy: @orca-so/whirlpools-sdk
Compatible with Solana Web3.js. Despite being called "Legacy", this class-based SDK remains a reliable choice for integrating with projects that use Solana Web3.js. It offers foundational tools for interacting with Orca's Whirlpool Program and includes utilities from @orca-so/common-sdk. 2. Core SDKs
The Core SDKs provide essential utilities for math operations and quotes, required for working with liquidity pools. These libraries focus on calculations such as determining position status, price conversions, and computing quotes on adjusting liquidity and swaps. It is written in Rust but has been compiled to WebAssembly (Wasm) for easy integration into TypeScript projects.

Rust: orca_whirlpools_core
TypeScript Kit: @orca-so/whirlpools-core
TypeScript Legacy: @orca-so/whirlpools-sdk
The Legacy SDK has separate utility classes for certain math operations such as PoolUtil, TickUtil, TickArrayUtil, and SwapUtils. For quotes, there are separate functions exported, such as decreaseLiquidityQuoteByLiquidity, increaseLiquidityQuoteByInputToken, swapQuoteByInputToken, collectFeesQuote, collectRewardsQuote, and more. Check out the reference docs in the navbor for more details. 3. Low-Level SDKs
This SDK provides direct program interactions and is designed for developers who need complete, low-level control over Whirlpool operations. It covers direct access to Solana accounts, instructions, and transactions.

Rust: orca_whirlpools_client
Compatible with anchor versions ^0.26 but <0.30. If you enable the anchor feature of orca_whirlpools_client in cargo.toml while using a version of anchor that's ^0.30 in your project, you may need to apply a lockfile patch to switch to a lower version:
cargo update anchor:<current-version> --precise 0.29

Compatible with solana-program versions ^1.18.0 but <3.0.0. By default, Cargo will install the latest version of Solana SDK ^v2. This can cause dependency issues when using older versions. To solve this you can apply a lockfile patch with the following command:
cargo update solana-program:<current-version> --precise <required-version>

NOTE: if you are dealing with compatibility issues for both anchor and solana-program, the order of the patches matters. First patch anchor, then patch solana-program.
TypeScript Kit: @orca-so/whirlpools-client
Compatible with Solana Kit
Typescript Legacy: @orca-so/whirlpools-sdk
The Legacy SDK offers the WhirlpoolIx class which enables you to interface directly with the instructions of the Whirlpool Program.

Environment Setup
This document covers the essential setup required to start building on Orca's SDK using the Whirlpools protocol. It includes installation, wallet setup, RPC client configuration, and the basics of interacting with the Solana ecosystem.

Rust
TypeScript Kit
TypeScript Legacy
Prerequisites
Before you start, ensure you have Rust installed. To ensure compatibility with the Solana SDK v1.18, we recommend using rustc 1.78.0.

1. Initialize a new project
   Initialize a new Rust project:

cargo new <project-name>

Add the necessary dependencies to your project:

cargo add orca_whirlpools solana-sdk solana-client tokio serde_json

Note: If you're using the Rust SDK in an already existing project which does not use the latest version of Solana SDK, you may need to apply a patchfile lock with the following command:

cargo update solana-sdk:<current-version> --precise <required-version>

You might also have to apply lockfile patches for solana-program, solana-client, solana-account-decoder, spl-token, spl-memo, spl-token-2022, spl-associated-token-account.

2. Wallet Management
   You can generate a file system wallet using the Solana CLI and load it in your program.

use solana_sdk::signer::keypair::Keypair;
use solana_sdk::signature::Signer;
use std::fs;

fn main() {
let wallet_string = fs::read_to_string("path/to/wallet.json").unwrap();
let keypair_bytes: Vec<u8> = serde_json::from_str(&wallet_string).unwrap();
let wallet = Keypair::from_bytes(&keypair_bytes).unwrap();
}

⚠️ Important: Never share your private key publicly.

3. Configure the Whirlpools SDK for Your Network
   Orca's Whirlpools SDK supports several networks: Solana Mainnet, Solana Devnet, Eclipse Mainnet, and Eclipse Testnet. To select a network, use the setWhirlpoolsConfig function.

use orca_whirlpools::{WhirlpoolsConfigInput, set_whirlpools_config_address};

fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
// Rest of the code
}

Available networks are:

solanaMainnet
solanaDevnet
eclipseMainnet
eclipseTestnet
ℹ️ The set_whirlpools_config_address function accepts either one of Orca's default network keys or a custom address. This allows you to specify a WhirlpoolsConfig account of your choice, including configurations not owned by Orca.

4. Airdrop SOL to Your Wallet
   Once your wallet is created, you will need some SOL to pay for transactions. You can request an airdrop of SOL from the network, but this is only available on Solana Devnet and Ecipse Testnet.

use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signer;

fn main() {
// Rest of the code
let rpc_client = RpcClient::new("https://api.devnet.solana.com");
match rpc_client.request_airdrop(&wallet.pubkey(), 1_000_000_000) {
Ok(signature) => println!("Airdrop successful. Transactoin signature: {:?}", signature),
Err(e) => println!("Error: {:?}", e),
}
}

5. Set the default funder for Transactions
   After funding your wallet, you can set the wallet as the FUNDER for future transactions within the SDK. The funder is the account that will cover the transaction costs for initializing pools, providing liquidity, etc.

use orca_whirlpools::{set_funder};
use solana_sdk::signature::Signer;

fn main() {
// Rest of the code
set_funder(wallet.pubkey()).unwrap();
}

Next steps
Once you've completed the setup, you can move on to building more complex functionalities using the Orca SDK, such as creating and managing pools, providing liquidity, etc. Refer to individual function documentation to use this wallet setup in action.

Creating Liquidity Pools on Orca
Creating liquidity pools on Orca is an essential step for launching your token and enabling trading. In this guide, we'll explore two types of liquidity pools available in the Orca ecosystem, Splash Pools and Concentrated Liquidity Pools, and help you understand how to create them, their differences, and which one best suits your needs.

1. Introduction to Pool Types
   Overview
   Liquidity pools are a foundational concept in DeFi, enabling users to trade tokens without relying on traditional order books. On Orca, liquidity pools provide the means for traders to swap between two tokens, while liquidity providers earn fees by supplying the tokens to the pool.

Splash Pools vs. Concentrated Liquidity Pools
Splash Pools: Splash Pools are the simplest type of liquidity pool. They are ideal for those looking to launch a new token with minimal parameters. You only need to provide the mint addresses of the two tokens and set the initial price. Splash Pools offer an easy entry point into liquidity provision, making them especially appealing for community-driven projects like memecoins. These projects often prioritize community engagement over technical complexity, and Splash Pools provide a straightforward way to get started.

Concentrated Liquidity Pools: Concentrated Liquidity Pools are more advanced and allow liquidity providers to concentrate their liquidity within specific price ranges. This results in higher capital efficiency but requires a deeper understanding of how to manage liquidity. Concentrated Liquidity Pools are better suited for experienced users who want greater control over their liquidity.

2. Getting Started Guide
   Rust
   TypeScript Kit
   TypeScript Legacy
   Before creating a Splash Pool or a Concentrated Liquidity Pool, ensure you have completed the environment setup:

RPC Setup: Use a Solana RPC client to communicate with the blockchain.
Wallet Creation: Create a wallet to interact with the Solana network.
Devnet Airdrop: Fund your wallet with a Solana devnet airdrop to cover transaction fees.
For more details, refer to our Environment Setup Guide.

Creating Splash Pools
Splash Pools are the easiest way to get started:

Token Mint Addresses: Provide the mint addresses of the two tokens that will make up the liquidity pool. The order of the tokens is important: the first token will be priced in terms of the second token. This means that the price you set will reflect how many units of the second token are needed to equal one unit of the first token. For example, if you set the price to 0.0001 SOL, this means that one unit of the first token is worth 0.0001 units of the second token (SOL). Make sure to verify the order of your tokens.
Initial Price: Set the initial price of token 1 in terms of token 2.
Funder: This will be your wallet, which will fund the initialization process.
Create Instructions: Use the appropriate function to generate the required pool creation instructions.
use orca_whirlpools::{
create_splash_pool_instructions, set_whirlpools_config_address, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signer, signer::keypair::Keypair};
use std::str::FromStr;
use tokio;
use orca_tx_sender::{
build_and_send_transaction,
set_rpc, get_rpc_client
};
use solana_sdk::commitment_config::CommitmentLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
set_rpc("https://api.devnet.solana.com").await?;
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();

    let token_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let token_b = Pubkey::from_str("BRjpCHtyQLNCo8gqRUr8jtdAj5AjPYQaoqbvcZiHok1k").unwrap(); // devUSDC
    let initial_price = Some(0.01);
    let wallet = Keypair::new(); // CAUTION: This wallet is not persistent.
    let funder = Some(wallet.pubkey());
    let rpc = get_rpc_client()?;

    let result =
        create_splash_pool_instructions(&rpc, token_a, token_b, initial_price, funder)
            .await?;

    // The instructions include new Tick Array accounts that need to be created
    // and signed for with their corresponding Keypair.
    let mut signers: Vec<&dyn Signer> = vec![&wallet];
    signers.extend(result.additional_signers.iter().map(|kp| kp as &dyn Signer));

    println!("Pool Address: {:?}", result.pool_address);
    println!(
        "Initialization Cost: {} lamports",
        result.initialization_cost
    );
    println!("Signers: {:?}", signers);

    let signature = build_and_send_transaction(
        result.instructions,
        &signers,
        Some(CommitmentLevel::Confirmed),
        None, // No address lookup tables
    ).await?;

    println!("Transaction sent: {}", signature);
    Ok(())

}

Creating Concentrated Liquidity Pools
Concentrated Liquidity Pools offer more flexibility:

Token Mint Addresses: Provide the two token mints.
Tick Spacing: Set the tick spacing, which defines the intervals for price ticks. Visit the Whirlpools Parameters page to learn more about the available values of tick spacing and their corresponding fee rates.
Initial Price: Specify the initial price of token 1 in terms of token 2.
Funder: This can be your wallet, which will fund the pool initialization. If the funder is not specified, the default wallet will be used. You can configure the default wallet through the SDK.
Create instructions: Use the appropriate function to create the pool.
use orca_whirlpools::{
create_concentrated_liquidity_pool_instructions, set_whirlpools_config_address,
WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signer, signer::keypair::Keypair};
use std::str::FromStr;
use tokio;
use orca_tx_sender::{
build_and_send_transaction,
set_rpc, get_rpc_client
};
use solana_sdk::commitment_config::CommitmentLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
set_rpc("https://api.devnet.solana.com").await?;
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();

    let token_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let token_b = Pubkey::from_str("BRjpCHtyQLNCo8gqRUr8jtdAj5AjPYQaoqbvcZiHok1k").unwrap(); // devUSDC
    let tick_spacing = 64;
    let initial_price = Some(0.01);
    let wallet = Keypair::new(); // CAUTION: This wallet is not persistent.
    let funder = Some(wallet.pubkey());
    let rpc = get_rpc_client()?;

    let result = create_concentrated_liquidity_pool_instructions(
        &rpc,
        token_a,
        token_b,
        tick_spacing,
        initial_price,
        funder,
    )
    .await?;

    println!("Pool Address: {:?}", result.pool_address);
    println!(
        "Initialization Cost: {} lamports",
        result.initialization_cost
    );

    let signature = build_and_send_transaction(
        result.instructions,
        &[&wallet],
        Some(CommitmentLevel::Confirmed),
        None, // No address lookup tables
    ).await?;

    println!("Transaction sent: {}", signature);
    Ok(())

}

Comparison of Pool Types
Feature Splash Pools Concentrated Liquidity Pools
Complexity Low High
Initial Parameters Token mints, price Token mints, tick spacing, price
Capital Efficiency Moderate High
Ideal For Beginners Advanced Users 3. Usage Examples
Launching a Token Pair with a Splash Pool
Suppose you want to launch a new memecoin and pair it with USDC. You can leverage the simplicity of Splash Pools to quickly set up the pool with an initial price. This is ideal if you want to keep things simple and start earning trading fees with minimal configuration. For example, if a development team is building a launchpad for memecoins, Splash Pools are an ideal solution.

Creating a Concentrated Liquidity Pool for Efficiency
If you want to maximize capital efficiency, you can use the flexibility of Concentrated Liquidity Pools to define specific price ranges for your liquidity. This approach is beneficial when you expect price movements within certain bounds and want to concentrate liquidity accordingly. For example, a DeFi protocol might use a Concentrated Liquidity Pool to facilitate a stablecoin-stablecoin pair, where the price is expected to remain within a tight range. By concentrating liquidity in this range, the protocol can maximize returns for liquidity providers and reduce slippage for traders.

4. Next Steps
   After creating a liquidity pool, the pool is still empty and requires liquidity for people to trade against. To make the pool functional, open a position and add liquidity. This enables traders to swap between tokens and helps you start earning fees.

Monitoring Liquidity Pools on Orca
Monitoring and fetching details about liquidity pools is crucial for understanding their current state, whether you want to gather insights about a Splash Pool, a Concentrated Liquidity Pool, or all pools between specific token pairs.

1. Overview of Pool Monitoring
   Fetching liquidity pool details helps developers gain insight into the current state of the pool, whether it is initialized or uninitialized, and retrieve relevant metrics like liquidity, price, and fee rates.

The SDKs offer three main functions to help developers monitor the pools:

Fetch Splash Pool: Fetches the details of a specific Splash Pool.
Fetch Concentrated Liquidity Pool: Fetches the details of a specific Concentrated Liquidity Pool.
Fetch Pools: Fetches all possible liquidity pools between two token mints, with various tick spacings.
Initialized vs. Uninitialized Pools
Each token pair can have multiple pools based on different tick spacings, corresponding to various fee tiers. When fetching pool data, it's possible to request a pool with a tick spacing that hasn't been used to create a pool for the given token pair. In this case, you'll receive a pool object with default parameters and an indication that the pool has not been set up.

When fetching all pools for a token pair, which iterates through all possible tick spacings, both initialized and uninitialized pools can be returned, allowing you to identify pools that have not yet been created.

2. Getting Started Guide
   Rust
   TypeScript Kit
   TypeScript Legacy
   Fetching a pool by Address
   If you already have the address of a Whirlpool:

Whirlpool Address: Provide the address of the specific Whirlpool you want to fetch.
Fetch Pool Details: Use the function to fetch the details of the Whirlpool at the provided address.
use orca_whirlpools::{
fetch_whirlpool, set_whirlpools_config_address, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
let whirlpool_address = Pubkey::from_str("3KBZiL2g8C7tiJ32hTv5v3KM7aK9htpqTw4cTXz1HvPt").unwrap(); // SOL/devUSDC

    let whirlpool = fetch_whirlpool(&rpc, whirlpool_address).await.unwrap();

    println!("Pool data: {:?}", whirlpool.data);

}

Fetching a Splash Pool by Token Pair
Token Mint Addresses: Provide the mint addresses of the two tokens that make up the liquidity pool.
Fetch Pool Details: Use the appropriate function to fetch the details of the specified Splash Pool.
use orca_whirlpools::{
fetch_splash_pool, set_whirlpools_config_address, PoolInfo, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

async fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
let token_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
let token_b = Pubkey::from_str("BRjpCHtyQLNCo8gqRUr8jtdAj5AjPYQaoqbvcZiHok1k").unwrap(); // devUSDC

    let pool_info = fetch_splash_pool(&rpc, token_a, token_b).await.unwrap();

    match pool_info {
        PoolInfo::Initialized(pool) => println!("Pool is initialized: {:?}", pool),
        PoolInfo::Uninitialized(pool) => println!("Pool is not initialized: {:?}", pool),
    }

}

Fetching a Concentrated Liquidity Pool by Token Pair
Token Mint Addresses: Provide the mint addresses of the two tokens that make up the liquidity pool.
Tick Spacing: Specify the tick spacing, which defines the intervals for price ticks.
Fetch Pool Details: Use the appropriate function to fetch the details of the specified Concentrated Liquidity Pool.
use orca_whirlpools::{
fetch_concentrated_liquidity_pool, set_whirlpools_config_address, PoolInfo, WhirlpoolsConfigInput
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
let token_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
let token_b = Pubkey::from_str("BRjpCHtyQLNCo8gqRUr8jtdAj5AjPYQaoqbvcZiHok1k").unwrap(); // devUSDC
let tick_spacing = 64;

    let pool_info = fetch_concentrated_liquidity_pool(&rpc, token_a, token_b, tick_spacing).await.unwrap();

    match pool_info {
        PoolInfo::Initialized(pool) => println!("Pool is initialized: {:?}", pool),
        PoolInfo::Uninitialized(pool) => println!("Pool is not initialized: {:?}", pool),
    }

}

Fetching Pools by Token Pairs
Token Mint Addresses: Provide the mint addresses of the two tokens that make up the liquidity pool.
Fetch Pool Details: Use the appropriate function to fetch the details of all pools for the specified token pair.
use orca_whirlpools::{
fetch_whirlpools_by_token_pair, set_whirlpools_config_address, PoolInfo, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
let token_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
let token_b = Pubkey::from_str("BRjpCHtyQLNCo8gqRUr8jtdAj5AjPYQaoqbvcZiHok1k").unwrap(); // devUSDC

    let pool_infos = fetch_whirlpools_by_token_pair(&rpc, token_a, token_b)
        .await
        .unwrap();

    for pool_info in pool_infos {
        match pool_info {
            PoolInfo::Initialized(pool) => println!("Pool is initialized: {:?}", pool),
            PoolInfo::Uninitialized(pool) => println!("Pool is not initialized: {:?}", pool),
        }
    }

}

3. Using Pool Data
   After fetching pool information, you can use it to:

Check if Pool Exists: Determine if a pool for a specific token pair and tick spacing has been created.
Monitor Liquidity: Track the amount of liquidity in the pool over time.
Track Prices: Monitor the current price of tokens in the pool.
Calculate Fees: Calculate expected fees based on the pool's fee rate and volume.
Data Analytics: Build analytics dashboards tracking pool performance and metrics. 4. Best Practices
When monitoring pools, consider these best practices:

Caching: Implement caching to reduce RPC calls, especially for frequently accessed pools.
Error Handling: Properly handle cases where pools might not exist.
Batch Requests: When possible, batch your requests to reduce the number of RPC calls.
Rate Limiting: Be mindful of RPC rate limits when monitoring multiple pools.
Data Freshness: Determine how recent the data needs to be for your application.

Opening a Position
Rust
TypeScript Kit
TypeScript Legacy
Opening a position in liquidity pools on Orca is a fundamental step for providing liquidity and earning fees. In this guide, we'll explore how to open a position in both Splash Pools and Concentrated Liquidity Pools, their differences, and which approach is suitable for different use cases.

1. Introduction to Positions in Pools
   A position in a liquidity pool represents your contribution of liquidity, which allows traders to swap between tokens while you earn a share of the trading fees. When you open a position, you decide how much liquidity to add, and this liquidity can later be adjusted or removed.

Splash Pools: Provide liquidity without specifying a price range. Ideal for those seeking a simple way to start providing liquidity.

Concentrated Liquidity Pools: Allow you to provide liquidity within a specified price range, enabling higher capital efficiency but requiring more advanced management.

Upon creation of the position, an NFT will be minted to represent ownership of the position. This NFT is used by the program to verify your ownership when adjusting liquidity, harvesting rewards, or closing the position. For more information, refer to Tokenized Positions.

⚠️ Risk of Divergence loss: The ratio of Token A to Token B that you deposit as liquidity is determined by several factors, including the current price. As trades occur against the pool, the amounts of Token A and Token B in the pool — and in your position — will change, which affects the price of the tokens relative to each other. This can work to your advantage, but it may also result in the combined value of your tokens (including any earned fees and rewards) being lower than when you initially provided liquidity.

2. Getting Started Guide
   Before opening a position, ensure you have completed the environment setup:

RPC Setup: Use a Solana RPC client to communicate with the blockchain.
Wallet Creation: Create a wallet to interact with the Solana network.
Devnet Airdrop: Fund your wallet with a Solana Devnet airdrop to cover transaction fees.
For more details, refer to our Environment Setup Guide

Opening a Position in Splash Pools
Pool Address: Provide the address of the Splash Pool where you want to open a position.
Liquidity Parameters: Specify tokenMaxA and tokenMaxB — the maximum amounts of each token you are willing to deposit. The program will use the minimum liquidity achievable within these caps. Use 0 for one-sided liquidity (e.g., tokenMaxB: 0 to deposit only token A).
Slippage Tolerance: Set the maximum slippage tolerance (optional, defaults to 1%). Slippage refers to the difference between the expected price and the actual price at which the transaction is executed. A lower slippage tolerance reduces the risk of price changes during the transaction but may lead to failed transactions if the market moves too quickly.
Funder: This will be your wallet, which will fund the transaction.
Create Instructions: Use the appropriate function to generate the necessary instructions.
use orca_whirlpools::{
open_full_range_position_instructions, set_whirlpools_config_address,
IncreaseLiquidityParam, WhirlpoolsConfigInput
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use std::str::FromStr;
use orca_tx_sender::{build_and_send_transaction, set_rpc, get_rpc_client};
use solana_sdk::commitment_config::CommitmentLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
set_rpc("https://api.devnet.solana.com").await?;
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let wallet = Keypair::new(); // Replace with your wallet loader
let rpc = get_rpc_client()?;
let whirlpool_address = Pubkey::from_str("3KBZiL2g8C7tiJ32hTv5v3KM7aK9htpqTw4cTXz1HvPt").unwrap();
let param = IncreaseLiquidityParam { token_max_a: 10_000_000, token_max_b: 0 };

let result = open_full_range_position_instructions(
&rpc,
whirlpool_address,
param,
Some(100),
Some(wallet.pubkey())
).await?;

let mut signers: Vec<&dyn Signer> = vec![&wallet];
signers.extend(result.additional_signers.iter().map(|kp| kp as &dyn Signer));

println!("Initialization cost: {:?}", result.initialization_cost);
println!("Position mint: {:?}", result.position_mint);

let signature = build_and_send_transaction(
result.instructions,
&signers,
Some(CommitmentLevel::Confirmed),
None,
).await?;

println!("Transaction sent: {}", signature);
Ok(())
}

Opening a Position in Concentrated Liquidity Pools
Pool Address: Provide the address of the Concentrated Liquidity Pool where you want to open a position.
Liquidity Parameters: Specify tokenMaxA and tokenMaxB — the maximum amounts of each token you are willing to deposit. The program will use the minimum liquidity achievable within these caps. Use 0 for one-sided liquidity.
Price Range: Set the lower and upper bounds of the price range within which your liquidity will be active. The current price and the specified price range will influence the quote amounts. If the current price is in the middle of your price range, the ratio of token A to token B will reflect that price. However, if the current price is outside your range, you will only deposit one token, resulting in one-sided liquidity. Note that your position will only earn fees when the price falls within your selected price range, so it's important to choose a range where you expect the price to remain active.
Slippage Tolerance: Set the maximum slippage tolerance (optional, defaults to 1%). Slippage refers to the difference between the expected token amounts and the actual amounts deposited into the liquidity pool. A lower slippage tolerance reduces the risk of depositing more tokens than expected but may lead to failed transactions if the market moves too quickly. For example, if you expect to deposit 100 units of Token A and 1,000 units of Token B, with a 1% slippage tolerance, the maximum amounts would be 101 Token A and 1,010 Token B.
Funder: This can be your wallet, which will fund the pool initialization. If the funder is not specified, the default wallet will be used. You can configure the default wallet through the SDK.
Create Instructions: Use the appropriate function to generate the necessary instructions.
use orca_whirlpools::{
open_position_instructions, set_whirlpools_config_address,
IncreaseLiquidityParam, WhirlpoolsConfigInput
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use std::str::FromStr;
use orca_tx_sender::{build_and_send_transaction, set_rpc, get_rpc_client};
use solana_sdk::commitment_config::CommitmentLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
set_rpc("https://api.devnet.solana.com").await?;
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let wallet = Keypair::new(); // Replace with your wallet loader
let rpc = get_rpc_client()?;
let whirlpool_address = Pubkey::from_str("3KBZiL2g8C7tiJ32hTv5v3KM7aK9htpqTw4cTXz1HvPt").unwrap();
let param = IncreaseLiquidityParam { token_max_a: 10_000_000, token_max_b: 0 };

let result = open_position_instructions(
&rpc,
whirlpool_address,
0.001,
100.0,
param,
Some(100),
Some(wallet.pubkey())
).await?;

let mut signers: Vec<&dyn Signer> = vec![&wallet];
signers.extend(result.additional_signers.iter().map(|kp| kp as &dyn Signer));

println!("Initialization cost: {:?}", result.initialization_cost);
println!("Position mint: {:?}", result.position_mint);

let signature = build_and_send_transaction(
result.instructions,
&signers,
Some(CommitmentLevel::Confirmed),
None,
).await?;

println!("Transaction sent: {}", signature);
Ok(())
}

⚠️ You cannot use this function on Splash Pools, as this function is specifically for Concentrated Liquidity Pools.

3. Usage examples
   Opening a Position in a Splash Pool
   Suppose you want to provide 1,000,000 tokens of Token A at a price of 0.0001 SOL. You will also need to provide 100 SOL as Token B to match the price. By using the SDK to open full range positions, you ensure that your liquidity is spread evenly across all price levels. This approach is ideal if you are launching a new token and want to facilitate easy swaps for traders.

Opening a Position in a Concentrated Liquidity Pool
If you want to maximize capital efficiency, you can open a position in a Concentrated Liquidity Pool. For example, if the current price is at 0.01 and you want to maximize profitability, you could use the SDK to deposit liquidity between the price range of 0.009 and 0.011. This approach allows you to focus your liquidity in a narrow range, making it more effective and potentially more profitable.

Next Steps
After opening a position, you can:

Add or Remove Liquidity: Adjust the amount of liquidity in your position based on market conditions.
Harvest Rewards: Collect rewards and fees without closing the position.
Monitor Performance: Track your position's performance and earned fees.
Close Position: When you decide to exit, close the position and withdraw the provided tokens along with any earned fees.

Adjusting Position Liquidity
Rust
TypeScript Kit
TypeScript Legacy
Once you've opened a position in an Orca Whirlpool, you may need to adjust the amount of liquidity you've provided to align with market conditions or your strategy. Whether you want to add more liquidity to capture additional fees or withdraw liquidity to reduce exposure or realize profits, the Whirlpools SDK provides functions for both.

This guide explains how to use the SDK functions to increase and decrease the liquidity in your position.

1. Overview of Adjusting Liquidity
   The SDK provides separate functions for increasing and decreasing liquidity:

Increase liquidity: Specify tokenMaxA and tokenMaxB — the maximum amounts you are willing to deposit. The program uses the minimum liquidity achievable within these caps. Use 0 for one-sided deposits.
Decrease liquidity: Specify one of liquidity, tokenA, or tokenB — the amount to withdraw. The SDK computes the others from your input.
With these functions, you can:

Increase liquidity to potentially earn more fees as trading volume grows.
Decrease liquidity to reduce exposure or withdraw profits. 2. Getting Started Guide
Adjusting Liquidity in a Position
Adjusting liquidity in an existing position can be done as follows:

RPC Client: Use a Solana RPC client to interact with the blockchain.
Position Mint: Provide the mint address of the NFT representing your position. This NFT serves as proof of ownership of the position you want to adjust.
Liquidity Parameters:
Increase: Specify tokenMaxA and tokenMaxB (max amounts to deposit; use 0 for one-sided).
Decrease: Specify one of liquidity, tokenA, or tokenB; the function computes the others.
Slippage tolerance: Set the maximum slippage tolerance (optional, defaults to 1%). Slippage refers to the difference between the expected token amounts added or removed when adjusting liquidity and the actual amounts that are ultimately deposited or withdrawn. A lower slippage tolerance reduces the risk of depositing or withdrawing more or fewer tokens than intended, but it may lead to failed transactions if the market moves too quickly.
Funder: This can be your wallet, which will fund the pool initialization. If a funder is not specified, the default wallet will be used. You can configure the default wallet through the SDK.
Create Instructions: Use the appropriate function to generate the necessary instructions.
use orca_whirlpools::{
decrease_liquidity_instructions, increase_liquidity_instructions,
set_whirlpools_config_address, DecreaseLiquidityParam, IncreaseLiquidityParam,
WhirlpoolsConfigInput
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use std::str::FromStr;
use orca_tx_sender::{build_and_send_transaction, set_rpc, get_rpc_client};
use solana_sdk::commitment_config::CommitmentLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
set_rpc("https://api.devnet.solana.com").await?;
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let wallet = Keypair::new(); // Replace with your wallet loader
let rpc = get_rpc_client()?;
let position_mint_address = Pubkey::from_str("HqoV7Qv27REUtmd9UKSJGGmCRNx3531t33bDG1BUfo9K").unwrap();
let increase_param = IncreaseLiquidityParam { token_max_a: 1_000_000, token_max_b: 0 };
let decrease_param = DecreaseLiquidityParam::TokenA(1_000_000);

    let increase_result = increase_liquidity_instructions(
        &rpc,
        position_mint_address,
        increase_param,
        Some(100),
        Some(wallet.pubkey()),
    )
    .await?;

    let mut signers_increase: Vec<&dyn Signer> = vec![&wallet];
    signers_increase.extend(increase_result.additional_signers.iter().map(|kp| kp as &dyn Signer));

    let increase_signature = build_and_send_transaction(
        increase_result.instructions,
        &signers_increase,
        Some(CommitmentLevel::Confirmed),
        None,
    ).await?;

    println!("Increase liquidity transaction sent: {}", increase_signature);

    let decrease_result = decrease_liquidity_instructions(
        &rpc,
        position_mint_address,
        decrease_param,
        Some(100),
        Some(wallet.pubkey()),
    )
    .await?;

    let mut signers_decrease: Vec<&dyn Signer> = vec![&wallet];
    signers_decrease.extend(decrease_result.additional_signers.iter().map(|kp| kp as &dyn Signer));

    println!("Decrease quote: {:?}", decrease_result.quote);

    let decrease_signature = build_and_send_transaction(
        decrease_result.instructions,
        &signers_decrease,
        Some(CommitmentLevel::Confirmed),
        None,
    ).await?;

    println!("Decrease liquidity transaction sent: {}", decrease_signature);
    Ok(())

}

3. Usage example
   You are creating a bot to manage investors' funds and want to optimize returns. Such a bot could rebalance liquidity based on market signals to maintain a specific target price range or to optimize fee collection during periods of high volatility.

4. Next steps
   After adjusting liquidity, you can:

Monitor Performance: Track your adjusted position to evaluate its performance and earned fees.
Harvest Rewards: Collect any earned fees and rewards without closing your position.
Make Further Adjustments: Depending on market conditions, continue to adjust liquidity as needed to maximize returns or manage risk.
By using the SDK to adjust liquidity, you gain flexibility in managing your positions and optimizing your liquidity provision strategy.

Harvesting a Position
Rust
TypeScript Kit
TypeScript Legacy
Harvesting a position in Orca Whirlpools allows you to collect any accumulated fees and rewards without closing the position. This process is useful when you want to claim your earnings while keeping your liquidity active in the pool, ensuring you continue to benefit from potential future fees.

1. Overview of Harvesting a Position
   The SDK helps you generate the instructions needed to collect fees and rewards from a position without closing it. This allows you to realize your earnings while maintaining liquidity in the pool.

With this function, you can:

Collect accumulated trading fees from your position.
Harvest rewards earned by providing liquidity, all while keeping the position active. 2. Getting Started Guide
Step-by-Step Guide to Harvesting a Position
To harvest fees and rewards from a position, follow these steps:

RPC Client: Use a Solana RPC client to interact with the blockchain.
Position Mint: Provide the mint address of the NFT representing your position. This NFT serves as proof of ownership and represents the liquidity in the position.
Authority: This can be your wallet, which will fund the pool initialization. If the authority is not specified, the default wallet will be used. You can configure the default wallet through the SDK.
Create Instructions: Use the appropriate function to generate the necessary instructions to harvest fees and rewards.
use orca_whirlpools::{
harvest_position_instructions, set_whirlpools_config_address, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::signature::Signer;
use orca_tx_sender::{
build_and_send_transaction,
set_rpc, get_rpc_client
};
use solana_sdk::commitment_config::CommitmentLevel;
use crate::utils::load_wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
set_rpc("https://api.devnet.solana.com").await?;
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let wallet = load_wallet();
let rpc = get_rpc_client()?;

    let position_mint_address =
        Pubkey::from_str("HqoV7Qv27REUtmd9UKSJGGmCRNx3531t33bDG1BUfo9K").unwrap();

    let result = harvest_position_instructions(&rpc, position_mint_address, Some(wallet.pubkey()))
        .await?;

    // The instructions may include new token accounts that need to be created
    // and signed for with their corresponding Keypair.
    let mut signers: Vec<&dyn Signer> = vec![&wallet];
    signers.extend(result.additional_signers.iter().map(|kp| kp as &dyn Signer));

    println!("Fees Quote: {:?}", result.fees_quote);
    println!("Rewards Quote: {:?}", result.rewards_quote);
    println!("Number of Instructions: {}", result.instructions.len());
    println!("Signers: {:?}", signers);

    let signature = build_and_send_transaction(
        result.instructions,
        &signers,
        Some(CommitmentLevel::Confirmed),
        None, // No address lookup tables
    ).await?;

    println!("Harvest transaction sent: {}", signature);
    Ok(())

}

3. Usage Example
   Suppose you are a developer creating a bot to manage investments for a group of investors. The bot periodically collects accumulated fees and rewards from liquidity positions to distribute profits among investors. By using the SDK, you can generate the instructions to collect earnings from each active position without closing it, allowing the liquidity to continue generating returns and potentially reinvest your earned fees into the position.

4. Next Steps
   After harvesting fees and rewards, you can:

Monitor Performance: Keep track of your position to evaluate future earnings and the overall performance.
Reinvest Earnings: Use the harvested fees and rewards to add more liquidity or diversify your positions.
Harvest Regularly: Regularly collect your earnings to maintain optimal capital efficiency while keeping your liquidity active.
By using the SDK, you can maximize the benefits of providing liquidity while keeping your position open and continuously earning fees.

Close a Position
Rust
TypeScript Kit
TypeScript Legacy
Once you've provided liquidity in a pool, there may come a time when you want to close your position entirely. The SDK allows you to fully remove liquidity from the pool, collect any outstanding fees and rewards, and close the position. This is useful when you want to exit the pool, either to realize profits or to reallocate capital to other opportunities.

This guide explains how to use the SDK to close a position.

1. Overview of Closing a Position
   When using the SDK to fully close a liquidity position, you generate all the necessary instructions. It performs the following key actions:

Collect Fees: Retrieves any fees earned from trades involving your liquidity.
Collect Rewards: Retrieves any rewards you've accumulated for the pool.
Decrease Liquidity: Removes any remaining liquidity in the position.
Close Position: Closes the position and returns the tokens in your account. 2. Getting Started Guide
Closing a Position
To close a position and withdraw all liquidity, follow these steps:

RPC Client: Use a Solana RPC client to interact with the blockchain.
Position Mint: Provide the mint address of the NFT representing your position. This NFT serves as proof of ownership and represents the liquidity in the position.
Parameters for Liquidity: Define the parameters for decreasing liquidity. This can be specified as a liquidity amount or as specific token amounts.
Slippage Tolerance: Set the maximum slippage tolerance (optional, defaults to 1%). Slippage refers to the difference between the expected token amounts you receive when closing a position and the actual amounts returned to your wallet. A lower slippage tolerance reduces the risk of receiving fewer tokens than expected but may lead to failed transactions if the market moves too quickly. For example, if you expect to receive 100 units of Token A and 1,000 units of Token B when closing your position, with a 1% slippage tolerance, the minimum amounts returned would be 99 Token A and 990 Token B.
Authority: This can be your wallet, which will fund the pool initialization. If the authority is not specified, the default wallet will be used. You can configure the default wallet through the SDK.
Create Instructions: Use the appropriate function to generate the necessary instructions.
use crate::utils::load_wallet;
use orca_whirlpools::{
close_position_instructions, set_whirlpools_config_address, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::signature::Signer;
use orca_tx_sender::{
build_and_send_transaction,
set_rpc, get_rpc_client
};
use solana_sdk::commitment_config::CommitmentLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
set_rpc("https://api.devnet.solana.com").await?;
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let wallet = load_wallet();
let rpc = get_rpc_client()?;

    let position_mint_address =
        Pubkey::from_str("HqoV7Qv27REUtmd9UKSJGGmCRNx3531t33bDG1BUfo9K").unwrap();

    let result = close_position_instructions(
        &rpc,
        position_mint_address,
        Some(100),
        Some(wallet.pubkey()),
    )
    .await?;

    // The instructions may include new token accounts that need to be created
    // and signed for with their corresponding Keypair.
    let mut signers: Vec<&dyn Signer> = vec![&wallet];
    signers.extend(result.additional_signers.iter().map(|kp| kp as &dyn Signer));

    println!("Quote token max B: {:?}", result.quote.token_est_b);
    println!("Fees Quote: {:?}", result.fees_quote);
    println!("Rewards Quote: {:?}", result.rewards_quote);
    println!("Number of Instructions: {}", result.instructions.len());
    println!("Signers: {:?}", signers);


    let signature = build_and_send_transaction(
        result.instructions,
        &signers,
        Some(CommitmentLevel::Confirmed),
        None, // No address lookup tables
    ).await?;

    println!("Close position transaction sent: {}", signature);
    Ok(())

}

3. Usage Example
   Suppose your trading strategy predicts that the current price range will lead to divergence loss, and you need to close the position to avoid further losses. Using the SDK, you can generate the instructions to collect all accumulated fees, rewards, and remove liquidity to prevent further losses.

4. Next Steps
   After closing a position, you can:

Open a New Position: If you want to redeploy your capital, you can open a new position in a different price range or pool.
Fetch All Positions: Check all your remaining positions to manage your overall liquidity strategy.
Reinvest the funds from the closed position into other opportunities based on market conditions.

Monitor Positions
Rust
TypeScript Kit
TypeScript Legacy
Retrieving details about positions held in liquidity pools is an essential part of managing your liquidity and monitoring performance. This guide explains how to use the SDK to gather information about all active positions held by a given wallet.

1. Overview of Position Monitoring
   Monitoring positions helps developers retrieve information on liquidity positions associated with a specific wallet. It scans the Solana blockchain for token accounts owned by the wallet, determines which ones represent positions, and decodes the data to provide detailed information about each position.

With position monitoring, you can:

Identify all liquidity positions held by a wallet
Gather information about liquidity, price ranges, and fees earned
Track position performance over time
Make informed decisions about adjusting or closing positions 2. Fetching Positions
Fetching Positions for a Wallet
Fetching positions is a straightforward process:

RPC Client: Use a Solana RPC client to interact with the blockchain.
Wallet Address: Provide the wallet address of the user whose positions you want to fetch.
Fetch Positions: Use the appropriate function to retrieve all positions held by the specified wallet.
use orca_whirlpools::{
fetch_positions_for_owner, set_whirlpools_config_address, WhirlpoolsConfigInput
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
let owner_address =
Pubkey::from_str("3KBZiL2g8C7tiJ32hTv5v3KM7aK9htpqTw4cTXz1HvPt").unwrap();

    let positions = fetch_positions_for_owner(&rpc, owner_address)
        .await
        .unwrap();

    println!("Positions: {:?}", positions);

}

Fetching Positions in a Whirlpool
To fetch all positions in a specific Whirlpool:

RPC Client: Use a Solana RPC client to interact with the blockchain.
Whirlpool Address: Provide the whirlpool address for the positions you want to fetch.
Fetch Positions: Use the appropriate function to retrieve all positions in a whirlpool.
use orca_whirlpools::{
fetch_positions_in_whirlpool, set_whirlpools_config_address, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
let whirlpool_address =
Pubkey::from_str("3KBZiL2g8C7tiJ32hTv5v3KM7aK9htpqTw4cTXz1HvPt").unwrap();

    let positions = fetch_positions_in_whirlpool(&rpc, whirlpool_address)
        .await
        .unwrap();

    println!("Positions: {:?}", positions);

}

3. Working with Position Data
   After fetching position information, you can use it to:

Track Position Performance: Monitor the performance of each position over time, including fees earned and value changes.
Identify Optimal Actions: Determine when to adjust liquidity, harvest rewards, or close positions based on performance metrics.
Calculate Returns: Compute the return on investment for each position by comparing current value to initial deposit.
Build Trading Strategies: Develop automated strategies for position management based on market conditions.
Portfolio Analytics: Create dashboards to visualize position performance across multiple pools. 4. Implementation Example
Suppose you're building a portfolio tracker for Whirlpool positions. You can create a monitoring service that periodically:

Fetches all positions for a user's wallet
Calculates current value and accumulated fees for each position
Compares performance against market benchmarks
Alerts users when positions require attention (e.g., out of range, significant fee accumulation)
This monitoring capability is essential for both manual traders and algorithmic strategies.

5. Next Steps
   After monitoring positions, you might want to:

Adjust Liquidity: Modify the amount of liquidity in positions based on their performance.
Harvest Rewards: Collect accumulated fees and rewards from profitable positions.
Close Position: Exit positions that are no longer aligned with your strategy.
By effectively monitoring positions, you gain the insights needed to optimize your liquidity management strategy and maximize returns.

Executing a Token Swap
You can use the SDK to execute a token swap on Orca. Whether you're swapping a specific amount of input tokens or looking to receive a precise amount of output tokens, this function handles the preparation of token accounts, liquidity data, and instruction assembly. It also manages slippage tolerance to ensure that swaps are executed within acceptable price changes.

This guide explains how to use the SDK to perform a token swap in an Orca Whirlpool.

Rust
TypeScript Kit
TypeScript Legacy

1. Overview of Executing a Token Swap
   The SDK allows you to swap tokens between different pools on Orca. It handles the calculation of token amounts, manages slippage, and assembles the necessary instructions for executing the swap.

With this function, you can:

Swap an exact amount of input tokens for the maximum possible output.
Specify the desired amount of output tokens and determine the necessary input.
Control slippage to manage your risk during volatile market conditions. 2. Getting Started Guide
Before executing a token swap, ensure you have completed the environment setup:

RPC Setup: Use a Solana RPC client to communicate with the blockchain.
Wallet Creation: Create a wallet to interact with the Solana network.
Devnet Airdrop: Fund your wallet with a Solana devnet airdrop to cover transaction fees.
For more details, refer to our Environment Setup Guide

Executing a Token Swap
To execute a token swap in an Orca Whirlpool, follow these steps:

RPC Client: Use a Solana RPC client to interact with the blockchain.
Pool Address: Provide the address of the Orca Whirlpool pool where the swap will take place.
Swap Parameters: Define the swap parameters. You only need to provide one of these parameters, and the function will compute the others in the returned quote based on the current price of the pool:
inputAmount: Specify the amount of tokens to swap (if exact input).
outputAmount: Specify the desired amount of tokens to receive (if exact output).
mint: Provide the mint address of the token you want to swap out.
Slippage tolerance: Set the maximum slippage tolerance (optional, defaults to 1%). Slippage refers to the difference between the expected amounts of tokens received or sent during the swap and the actual amounts executed. A lower slippage tolerance reduces the risk of receiving fewer tokens than expected, but may lead to failed transactions if the market moves too quickly. For example, if you expect to receive 1,000 units of Token B for 100 units of Token A, with a 1% slippage tolerance, the maximum Token A spent will be 101, and the minimum Token B received will be 990.
Signer: This can be your wallet, which will fund the pool initialization. If a signer is not specified, the default wallet will be used. You can configure the default wallet through the SDK.
Create Instructions: Use the appropriate function to generate the necessary instructions for the swap.
use crate::utils::load_wallet;
use orca_whirlpools::{
set_whirlpools_config_address, swap_instructions, SwapType, WhirlpoolsConfigInput,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaDevnet).unwrap();
let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
let wallet = load_wallet();
let whirlpool_address =
Pubkey::from_str("3KBZiL2g8C7tiJ32hTv5v3KM7aK9htpqTw4cTXz1HvPt").unwrap();
let mint_address = Pubkey::from_str("BRjpCHtyQLNCo8gqRUr8jtdAj5AjPYQaoqbvcZiHok1k").unwrap();
let input_amount = 1_000_000;

    let result = swap_instructions(
        &rpc,
        whirlpool_address,
        input_amount,
        mint_address,
        SwapType::ExactIn,
        Some(100),
        Some(wallet.pubkey()),
    )
    .await
    .unwrap();

    // The instructions may include new token accounts that need to be created
    // and signed for with their corresponding Keypair.
    let mut signers: Vec<&dyn Signer> = vec![&wallet];
    signers.extend(result.additional_signers.iter().map(|kp| kp as &dyn Signer));

    let signature = build_and_send_transaction(
        result.instructions,
        &signers,
        Some(CommitmentLevel::Confirmed),
        None, // No address lookup tables
    ).await?;

    println!("Quote estimated token out: {:?}", result.quote);
    println!("Number of Instructions: {}", result.instructions.len());
    println!("Signers: {:?}", signers);
    println!("Transaction sent: {}", signature);

Submit Transaction: Include the generated instructions in a Solana transaction and send it to the network using the Solana SDK. 3. Example Usage
Suppose you are developing an arbitrage bot that looks for price discrepancies between different liquidity pools on Orca. By using the SDK, the bot can retrieve the quote object for a potential swap, which includes details about the token amounts and expected output. The bot can quickly compare quotes from multiple pools to identify arbitrage opportunities and execute profitable swaps.

4. Next Steps
   After successfully executing a token swap, you might want to:

Open a Position: Provide liquidity to the pool and earn fees.
Monitor Positions: Track your position's performance over time.
Build more complex trading strategies by combining multiple swaps.
By effectively using the SDK's swap functionality, you can create powerful trading applications on Orca Whirlpools.

Sending and Landing Transactions
Rust & Typescript Kit
Typescript Legacy
In this guide, we'll explore how to send transactions to the Solana blockchain for any Solana project. We'll cover two approaches:

Using the simplified tx-sender library - a lightweight solution that works with any Solana project
The manual approach using Solana's native SDKs directly
The Easy Way: Using tx-sender
The tx-sender library is a lightweight package designed to simplify transaction building and sending in Solana. It handles all the complex aspects like priority fees, Jito tips, compute unit estimation, and retry logic automatically.

Installation
Rust
Typescript Kit
Cargo.toml
[dependencies]
orca_tx_sender = { version = "^3.0.0" }

Usage
Rust
Typescript Kit
main.rs
use orca_tx_sender::{
build_and_send_transaction,
set_rpc, get_rpc_client
};
use solana_sdk::signature::Signer;
use solana_sdk::commitment_config::CommitmentLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
// Initialize RPC configuration (required!)
set_rpc("https://api.mainnet-beta.solana.com").await?;

    // Get instructions from Whirlpools SDK
    let instructions_result = // your whirlpool instructions here

    // Some instructions may require additional signers
    let mut signers: Vec<&dyn Signer> = vec![&wallet];
    signers.extend(instructions_result.additional_signers.iter().map(|kp| kp as &dyn Signer));

    // Build and send transaction
    let signature = build_and_send_transaction(
        instructions_result.instructions,
        &signers, // signers array including your wallet and any additional signers
        Some(CommitmentLevel::Confirmed),
        None, // No address lookup tables
    ).await?;

    println!("Transaction sent: {}", signature);
    Ok(())

}

Configuration Options
The tx-sender library provides flexible configuration options to optimize your transaction sending strategy. Let's break these down in detail:

Default Settings
By default, tx-sender uses the following configuration:

Priority Fees: Dynamic pricing with a max cap of 0.004 SOL (4,000,000 lamports)
Jito Tips: Dynamic pricing with a max cap of 0.004 SOL (4,000,000 lamports)
Compute Unit Margin: 1.1x multiplier for compute unit calculation (10% margin)
Jito Block Engine URL: https://bundles.jito.wtf
Priority Fee Configuration
Priority fees incentivize validators to include your transaction in blocks. The tx-sender library supports three priority fee strategies:

Understanding Dynamic Fees and Percentiles
When using the "dynamic" fee strategy, tx-sender automatically analyzes recent network conditions to determine an appropriate fee. The system works by:

Calling the getRecentPrioritizationFees RPC method, which returns data about fees from the last 150 blocks
Sorting these fees from lowest to highest
Selecting a specific percentile from this data
The tx-sender library allows capping these dynamic fees at a maximum amount to prevent excessive spending during extreme network conditions.

Note: The tx-sender library automatically filters out zero-fee transactions before calculating percentiles. This ensures that during periods of low network activity when many blocks have zero fees, your transaction still has an appropriate non-zero fee to improve landing probability.

Rust
Typescript Kit
// 1. Dynamic fees - automatically adjusts based on network conditions
set_priority_fee_strategy(PriorityFeeStrategy::Dynamic {
percentile: Percentile::P75, // Options: P25, P50, P75, P95, P99
max_lamports: 5_000_000, // Optional: Cap at 0.005 SOL (default: 4,000,000)
})?;

// 2. Exact fees - specify an exact amount
set_priority_fee_strategy(PriorityFeeStrategy::Exact(10_000))?; // Exactly 0.00001 SOL

// 3. No priority fees
set_priority_fee_strategy(PriorityFeeStrategy::Disabled)?;

Jito Tip Configuration
Jito tips are additional fees that go to Jito MEV (Maximal Extractable Value) validators, who represent approximately 85% of Solana's validator stake. These tips can improve transaction landing probability even further than regular priority fees.

Understanding Jito Tips and Dynamic Pricing
Jito tips work similarly to priority fees but are specifically for Jito validators. When using dynamic Jito tips:

The percentile system works the same way as with priority fees - selecting from recent fee data
Jito offers an additional option: "50ema" (Exponential Moving Average), which smooths out fee spikes by using a weighted average
Jito tips are sent directly to the Jito block engine rather than through the regular fee mechanism
Using Jito tips is particularly effective because:

Jito validators account for about 85% of Solana's stake weight
They use a specialized searching algorithm to look for higher-tipped transactions
During congestion, Jito validators can help your transaction land faster
Like priority fees, Jito tips can be capped to prevent excessive spending. The default cap is 4,000,000 lamports (0.004 SOL).

Rust
Typescript Kit
// 1. Dynamic Jito tips
set_jito_fee_strategy(JitoFeeStrategy::Dynamic {
percentile: JitoPercentile::P50Ema, // P25, P50, P75, P95, P99, P50Ema
max_lamports: 3_000_000, // Optional: Cap at 0.003 SOL
})?;

// 2. Exact Jito tip
set_jito_fee_strategy(JitoFeeStrategy::Exact(20_000))?; // Exactly 0.00002 SOL

// 3. No Jito tips
set_jito_fee_strategy(JitoFeeStrategy::Disabled)?;

// Set custom Jito block engine URL (defaults to "https://bundles.jito.wtf")
set_jito_block_engine_url("https://your-jito-service.com")?;

Compute Unit Configuration
The compute units represent the computational resources your transaction requires. The margin multiplier adds extra units as a safety buffer to prevent transaction failures.

How Compute Unit Estimation Works
When sending a transaction, tx-sender performs these steps to optimize compute unit usage:

First, it simulates your transaction on the RPC to estimate the required compute units
Then, it applies the margin multiplier to add a safety buffer (default is 1.1, or 10% extra)
Finally, it sets a compute unit limit instruction at the beginning of your transaction
This process ensures that your transaction:

Has enough compute units to complete execution
Doesn't allocate unnecessarily high compute units (which would cost more in fees)
Has a safety margin to account for differences between simulation and actual execution
Setting an appropriate margin is important because:

Too low (close to 1.0): Transaction might fail with "out of compute units" error if network conditions change
Too high (over 1.5): Transactions that request higher compute units get lower priority for the same prioritization fee. Higher compute units signal to validators that your transaction will use more resources.
For most transactions, a value between 1.1 and 1.2 (10-20% margin) is appropriate. For complex or unpredictable transactions, you might want to use a higher value like 1.3 or 1.4.

Rust
Typescript Kit
// Values typically range from 1.0 (no margin) to 2.0 (100% extra margin)
// Default is 1.1 (10% margin)
set_compute_unit_margin_multiplier(1.2)?; // 20% margin

RPC Configuration
Rust
Typescript Kit
// Basic RPC configuration
set_rpc("https://api.mainnet-beta.solana.com").await?;
// Get the configured RPC client for other operations
let client = get_rpc_client()?;

Example: Comprehensive Configuration
Here's an example of a complete configuration setup:

Rust
Typescript Kit
use orca_tx_sender::{
build_and_send_transaction,
PriorityFeeStrategy, JitoFeeStrategy,
Percentile, JitoPercentile,
set_priority_fee_strategy, set_jito_fee_strategy,
set_compute_unit_margin_multiplier, set_jito_block_engine_url,
set_rpc, get_rpc_client
};
use solana_sdk::commitment_config::CommitmentLevel;

// 1. Set up RPC connection
set_rpc("https://api.mainnet-beta.solana.com").await?;

// 2. Configure priority fees
set_priority_fee_strategy(PriorityFeeStrategy::Dynamic {
percentile: Percentile::P75,
max_lamports: 5_000_000, // 0.005 SOL
})?;

// 3. Configure Jito tips
set_jito_fee_strategy(JitoFeeStrategy::Dynamic {
percentile: JitoPercentile::P50Ema,
max_lamports: 3_000_000, // 0.003 SOL
})?;

// 4. Set compute unit margin
set_compute_unit_margin_multiplier(1.2)?;

// 5. Optional: Custom Jito endpoint
set_jito_block_engine_url("https://bundles.jito.wtf")?;

// 6. Send transaction with configured settings
let signature = build_and_send_transaction(
instructions,
&[&wallet],
Some(CommitmentLevel::Confirmed),
None
).await?;

The Manual Way: Using Solana SDKs Directly
In this section, we'll explore how to send the instructions using the Solana SDK directly - both in Typescript and Rust. We'll cover the following key topics:

Client-side retry
Prioritization fees
Compute budget estimation
We also cover key considerations for sending transactions in web applications with wallet extensions, along with additional steps to improve transaction landing.

Code Overview

1. Dependencies
   Let's start by importing the necessary dependencies from Solana's SDKs.

Rust
Typescript Kit
Cargo.toml
serde_json = { version = "^1.0" }
solana-client = { version = "^1.18" }
solana-sdk = { version = "^1.18" }
tokio = { version = "^1.41.1" }

main.rs
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::fs;
use std::str::FromStr;
use tokio::time::{sleep, Duration, Instant};

2. Create Transaction Message From Instructions
   To send a transaction on Solana, you need to include a blockhash to the transaction. A blockhash acts as a timestamp and ensures the transaction has a limited lifetime. Validators use the blockhash to verify the recency of a transaction before including it in a block. A transaction referencing a blockhash is only valid for 150 blocks (~1-2 minutes, depending on slot time). After that, the blockhash expires, and the transaction will be rejected.

Durable Nonces: In some cases, you might need a transaction to remain valid for longer than the typical blockhash lifespan, such as when scheduling future payments or collecting multi-signature approvals over time. In that case, you can use durable nonces to sign the transaction, which includes a nonce in place of a recent blockhash.

You also need to add the signers to the transactions. With Solana Kit, you can create instructions and add additional signers as TransactionSigner to the instructions. The Typescript Whirlpools SDK leverages this functioanlity and appends all additional signers to the instructions for you. In Rust, this feautures is not available. Therefore, the Rust Whirlpools SDK may return instruction_result.additional_signers if there are any, and you need to manually append them to the transaction.

Here's how the transaction message is created:

Rust
Typescript Kit
main.rs #[tokio::main]
async fn main() {
// ...
let instructions_result = // get instructions from Whirlpools SDK
let message = Message::new(
&instructions_result.instructions,
Some(&wallet.pubkey()),
);
let mut signers: Vec<&dyn Signer> = vec![&wallet];
signers.extend(
instructions_result
.additional_signers
.iter()
.map(|kp| kp as &dyn Signer),
);
let recent_blockhash = rpc.get_latest_blockhash().await.unwrap();
let transaction = Transaction::new(&signers, message, recent_blockhash);
// ...
}

3. Estimating Compute Unit Limit and Prioritization Fee
   Before sending a transaction, it's important to set a compute unit limit and an appropriate prioritization fee.

Transactions that request fewer compute units get high priority for the same amount of prioritization fee (which is defined per compute unit). Setting the compute units too low will result in a failed transaction.

You can get an estimate of the compute units by simulating the transaction on the RPC. To avoid transaction failures caused by underestimating this limit, you can add an additional 100,000 compute units, but you can adjust this based on your own tests.

The prioritization fee per compute unit also incentivizes validators to prioritize your transaction, especially during times of network congestion. You can call the getRecentPrioritizationFees RPC method to retrieve an array of 150 values, where each value represents the lowest priority fee paid for transactions that landed in each of the past 150 blocks. In this example, we sort that list and select the 50th percentile, but you can adjust this if needed. The prioritization fee is provided in micro-lamports per compute unit. The total priority fee in lamports you will pay is calculated as
(
estimated compute units
⋅
prioritization fee
)
/
10
6
(estimated compute units⋅prioritization fee)/10
6
.

Rust
Typescript Kit
main.rs #[tokio::main]
async fn main() {
// ...
let simulated_transaction = rpc.simulate_transaction(&transaction).await.unwrap();

let mut all_instructions = vec![];
if let Some(units_consumed) = simulated_transaction.value.units_consumed {
let units_consumed_safe = units_consumed as u32 + 100_000;
let compute_limit_instruction =
ComputeBudgetInstruction::set_compute_unit_limit(units_consumed_safe);
all_instructions.push(compute_limit_instruction);

    let prioritization_fees = rpc
      .get_recent_prioritization_fees(&[whirlpool_address])
      .await
      .unwrap();
    let mut prioritization_fees_array: Vec<u64> = prioritization_fees
      .iter()
      .map(|fee| fee.prioritization_fee)
      .collect();
    prioritization_fees_array.sort_unstable();
    let prioritization_fee = prioritization_fees_array
      .get(prioritization_fees_array.len() / 2)
      .cloned();

    if let Some(prioritization_fee) = prioritization_fee {
      let priority_fee_instruction =
        ComputeBudgetInstruction::set_compute_unit_price(prioritization_fee);
      all_instructions.push(priority_fee_instruction);
    }

}
// ...
}

4. Sign and Submit Transaction
   Finally, the transaction needs to be signed, encoded, and submitted to the network. A client-side time-base retry mechanism ensures that the transaction is repeatedly sent until it is confirmed or the time runs out. We use a time-based loop, because we know that the lifetime of a transaction is 150 blocks, which on average takes about 79-80 seconds. The signing of the transactions is an idempotent operation and produces a transaction hash, which acts as the transaction ID. Since transactions can be added only once to the block chain, we can keep sending the transaction during the lifetime of the trnsaction.

You're probably wondering why we don't just use the widely used sendAndConfirm method. This is because the retry mechanism of the sendAndConfirm method is executed on the RPC. By default, RPC nodes will try to forward (rebroadcast) transactions to leaders every two seconds until either the transaction is finalized, or the transaction's blockhash expires. If the outstanding rebroadcast queue size is greater than 10,000 transaction, newly submitted transactions are dropped. This means that at times of congestion, your transaction might not even arrive at the RPC in the first place. Moreover, the confirmTransaction RPC method that sendAndConfirm calls is deprecated.

Rust
Typescript Kit
main.rs #[tokio::main]
async fn main() {
// ...
all_instructions.extend(open_position_instructions.instructions);
let message = Message::new(&all_instructions, Some(&wallet.pubkey()));

let transaction = Transaction::new(&signers ,message , recent_blockhash);
let transaction_config = RpcSendTransactionConfig {
skip_preflight: true,
preflight_commitment: Some(CommitmentLevel::Confirmed),
max_retries: Some(0),
..Default::default()
};

let start_time = Instant::now();
let timeout = Duration::from_secs(90);
let send_transaction_result = loop {
if start_time.elapsed() >= timeout {
break Err(Box::<dyn std::error::Error>::from("Transaction timed out"));
}
let transaction_start_time = Instant::now();

    let signature: Signature = rpc
      .send_transaction_with_config(&transaction, transaction_config)
      .await
      .unwrap();
    let statuses = rpc
      .get_signature_statuses(&[signature])
      .await
      .unwrap()
      .value;

    if let Some(status) = statuses[0].clone() {
      break Ok((status, signature));
    }

    let elapsed_time = transaction_start_time.elapsed();
    let remaining_time = Duration::from_millis(1000).saturating_sub(elapsed_time);
    if remaining_time > Duration::ZERO {
      sleep(remaining_time).await;
    }

};

let signature = send_transaction_result.and_then(|(status, signature)| {
if let Some(err) = status.err {
Err(Box::new(err))
} else {
Ok(signature)
}
});
println!("Result: {:?}", signature);
}

Handling transactions with Wallets in web apps.
Creating Noop Signers
When sending transactions from your web application, users need to sign the transaction using their wallet. Since the transaction needs to assembled beforehand, you can create a noopSigner (no-operation signer) and add it to the instructions. This will act as a placeholder for you instructions, indicating that a given account is a signer and the signature wil be added later. After assembling the transaction you can pass it to the wallet extension. If the user signs, it will return a serialized transaction with the added signature.

Prioritization Fees
Some wallets will calculate and apply priority fees for your transactions, provided:

The transaction does not already have signatures present.
The transaction does not have existing compute-budget instructions.
The transactions will still be less than the maximum transaction size fo 1232 bytes, after applying compute-budget instructions.
Additional Improvements for Landing Transactions
You could send your transaction to multiple RPC nodes at the same time, all within each iteration of the time-based loop.
At the time of writing, 85% of Solana validators are Jito validators. Jito validators happily accept an additional tip, in the form a SOL transfer, to prioritize a transaction. A good place to get familiarized with Jito is here: https://www.jito.network/blog/jito-solana-is-now-open-source/
Solana gives staked validators more reliable performance when sending transactions by routing them through prioritized connections. This mechanism is referred to as stake-weighted Quality of Service (swQoS). Validators can extend this service to RPC nodes, essentially giving staked connections to RPC nodes as if they were validators with that much stake in the network. RPC providers, like Helius and Titan, expose such peered RPC nodes to paid users, allowing users to send transactions to RPC nodes which use the validator's staked connections. From the RPC, the transaction is then sent over the staked connection with a lower likelihood of being delayed or dropped.

Orca Utility Helpers
The Orca SDKs provide a range of utility functions that you may use when interacting with the Whirlpool protocol.

Rust & Typescript Kit
Typescript Legacy
Orca Whirlpools Core SDK
This package provides developers with advanced functionalities for interacting with the Whirlpool Program on Solana. Originally written in Rust, it has been compiled to WebAssembly (Wasm). This compilation makes the SDK accessible in JavaScript/TypeScript environments, offering developers the same core features and calculations for their Typescript projects. The SDK exposes convenient methods for math calculations, quotes, and other utilities, enabling seamless integration within web-based projects.

Key Features
Math Library: Contains a variety of functions for math operations related to bundles, positions, prices, ticks, and tokens, including calculations such as determining position status or price conversions.
Quote Library: Provides utility functions for generating quotes, such as increasing liquidity, collecting fees or rewards, and swapping, to help developers make informed decisions regarding liquidity management.
Installation:
Rust
Typescript Kit
cargo add orca_whirlpools_core

Usage
Here are some basic examples of how to use the package:

Math Example
The following example demonstrates how to use the isPositionInRange function to determine whether a position is currently in range.

Rust
Typescript Kit
use orca_whirlpools_core::is_position_in_range;

fn main() {
let current_sqrt_price = 7448043534253661173u128;
let tick_index_1 = -18304;
let tick_index_2 = -17956;

    let in_range = is_position_in_range(current_sqrt_price.into(), tick_index_1, tick_index_2);
    println!("Position in range? {:?}", in_range);

}

Expected output:

Position in range? true

Quote Example
The following example demonstrates how to use the increaseLiquidityQuoteA function to calculate a quote for increasing liquidity given a token A amount.

Rust
Typescript Kit
use orca_whirlpools_core::increase_liquidity_quote_a;
use orca_whirlpools_core::TransferFee;

fn main() {
let token_amount_a = 1000000000u64;
let slippage_tolerance_bps = 100u16;
let current_sqrt_price = 7437568627975669726u128;
let tick_index_1 = -18568;
let tick_index_2 = -17668;
let transfer_fee_a = Some(TransferFee::new(200));
let transfer_fee_b = None;

    let quote = increase_liquidity_quote_a(
        token_amount_a,
        slippage_tolerance_bps,
        current_sqrt_price.into(),
        tick_index_1,
        tick_index_2,
        transfer_fee_a,
        transfer_fee_b,
    ).unwrap();

    println!("{:?}", quote);

}

Expected output:

IncreaseLiquidityQuote {
liquidity_delta: 16011047470,
token_est_a: 1000000000,
token_est_b: 127889169,
token_max_a: 1010000000,
token_max_b: 129168061,
}

Orca Public API
Orca Team

Download OpenAPI Document

Download OpenAPI Document
A list of public endpoints for Orca protocol

Server
Server:
https://api.orca.so/v2/{chain}
chain

Select value
Client Libraries
Rust reqwest
health (Collapsed)​Copy link
Health and diagnostic endpoints

protocol ​Copy link
Orca protocol information endpoints

protocolOperations
get
/protocol
get
/protocol/token
get
/protocol/token/circulating_supply
get
/protocol/token/total_supply
Recent protocol stats for the Orca​Copy link
This endpoint returns general information about the Orca protocol including:

Total Value Locked (TVL) in USDC
24-hour trading volume in USDC
24-hour fees collected in USDC
24-hour protocol revenue in USDC
Responses

200
Protocol information retrieved successfully
application/json

500
Internal server error
application/json
Request Example forget/protocol
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/protocol");

let response = request.send().await?;

Test Request
(get /protocol)
Status:200
Status:500
{
"fees24hUsdc": "317428.0521046",
"revenue24hUsdc": "41265.646773",
"tvl": "230551269.0085",
"volume24hUsdc": "552567794.7830"
}

Protocol information retrieved successfully

Information about the Orca token​Copy link
This endpoint returns detailed information about the Orca token, including:

Token symbol, name, and description
Token image URL
Current price in USDC
Circulating and total supply
24-hour trading volume statistics
The circulating supply is calculated as the total supply minus tokens held in excluded addresses (treasury, grants, etc.). The data is cached to reduce RPC and database load.

Responses

200
Token information retrieved successfully
application/json

500
Internal server error
application/json
Request Example forget/protocol/token
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/protocol/token");

let response = request.send().await?;

Test Request
(get /protocol/token)
Status:200
Status:500
{
"circulatingSupply": "53275182.419413",
"description": "Orca Token",
"imageUrl": "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE/logo.png",
"name": "Orca",
"price": "1.6767140",
"stats": {
"24h": {
"volume": "594947.6898176792"
}
},
"symbol": "ORCA",
"totalSupply": "99999712.243267"
}

Token information retrieved successfully

Circulating supply of the Orca protocol's token​Copy link
This endpoint returns the circulating supply of the protocol's token. The circulating supply is calculated as the total supply minus the tokens held in excluded addresses (treasury, grants, etc.).

Responses

200
Circulating supply retrieved successfully
application/json

500
Internal server error
application/json
Request Example forget/protocol/token/circulating_supply
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/protocol/token/circulating_supply");

let response = request.send().await?;

Test Request
(get /protocol/token/circulating_supply)
Status:200
Status:500
{
"circulating_supply": "53275183"
}

Circulating supply retrieved successfully

Total supply of the Orca token​Copy link
This endpoint returns the total supply of the protocol's token. The total supply represents all tokens that have been minted, including those held in excluded addresses (treasury, grants, etc.).

The response is adjusted for token decimals (6 decimal places) and rounded up to the nearest whole number.

Responses

200
Total supply retrieved successfully
application/json

500
Internal server error
application/json
Request Example forget/protocol/token/total_supply
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/protocol/token/total_supply");

let response = request.send().await?;

Test Request
(get /protocol/token/total_supply)
Status:200
Status:500
{
"total_supply": "99999713"
}

Total supply retrieved successfully

tokens ​Copy link
Token information endpoints

tokensOperations
get
/tokens
get
/tokens/search
get
/tokens/{mint_address}
List tokens with pagination and filtering options​Copy link
Returns a paginated list of tokens with optional filtering and sorting.

Query Parameters
nextCopy link to next
Type:string
Optional cursor for pagination (next)

previousCopy link to previous
Type:string
Optional cursor for pagination (previous)

sizeCopy link to size
Type:integer
Format:int32
min:  
0
Maximum number of tokens to return (default: 50, max: 3000)

sort_byCopy link to sort_by
Type:string
enum
Field to sort by (default: mint_id)

address
mint_id
volume_24h
sort_directionCopy link to sort_direction
Type:string
enum
Sort direction (default: asc)

asc
desc
tokensCopy link to tokens
Type:string
Optional filter for specific token addresses

Responses

200
List of tokens retrieved successfully
application/json
400Copy link to 400
Bad user parameters

500Copy link to 500
Internal server error

Request Example forget/tokens
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/tokens");

let response = request.send().await?;

Test Request
(get /tokens)
Status:200
{
"data": [
{
"address": "So11111111111111111111111111111111111111112",
"decimals": 6,
"extensions": "{\n \"confidentialTransferFeeConfig\": {\n \"authority\": \"9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM\",\n \"harvestToMintEnabled\": true,\n \"withdrawWithheldAuthorityElgamalPubkey\": \"...\",\n \"withheldAmount\": \"...\"\n },\n \"confidentialTransferMint\": {\n \"auditorElgamalPubkey\": null,\n \"authority\": \"...\",\n \"autoApproveNewAccounts\": false\n },\n \"metadataPointer\": {\n \"authority\": \"...\",\n \"metadataAddress\": \"...\"\n },\n \"mintCloseAuthority\": {\n \"closeAuthority\": \"...\"\n },\n \"permanentDelegate\": {\n \"delegate\": \"...\"\n },\n \"tokenMetadata\": {\n \"additionalMetadata\": [],\n \"mint\": \"...\",\n \"name\": \"Name of Coin\",\n \"symbol\": \"Coin\",\n \"updateAuthority\": \"...\",\n \"uri\": \"https://example.com/image.json\"\n },\n \"transferFeeConfig\": {\n \"newerTransferFee\": {\n \"epoch\": 605,\n \"maximumFee\": 0,\n \"transferFeeBasisPoints\": 0\n },\n \"olderTransferFee\": {\n \"epoch\": 605,\n \"maximumFee\": 0,\n \"transferFeeBasisPoints\": 0\n },\n \"transferFeeConfigAuthority\": \"...\",\n \"withdrawWithheldAuthority\": \"...\",\n \"withheldAmount\": 0\n },\n \"transferHook\": {\n \"authority\": \"...\",\n \"programId\": null\n }\n }",
"freezeAuthority": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
"isInitialized": true,
"metadata": "{\n \"description\": \"Description about the token\",\n \"image\": \"https://example.com/image.png\",\n \"name\": \"Name of Token\",\n \"risk\": 2,\n \"symbol\": \"TKN\"\n }",
"mintAuthority": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
"priceUsdc": "0.9998715445029346189836299351",
"stats": "{\"24h\": {\"volume\": 1197647.1678154785}}",
"supply": "154400747126297",
"tags": "[\"confidentialTransferFeeConfig\", \"confidentialTransferMint\", \"metadataPointer\", \"mintCloseAuthority\", \"permanentDelegate\", \"tokenMetadata\", \"transferFeeConfig\", \"transferHook\"]",
"tokenProgram": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
"updatedAt": "2025-05-09T00:04:50.745163Z",
"updatedEpoch": 784
}
],
"meta": {
"next": null,
"previous": null
}
}

List of tokens retrieved successfully

Search for tokens by query string​Copy link
Returns a list of tokens that match the query string. The query string is matched against the token name, symbol, and address.

Query Parameters
qCopy link to q
Type:string
Query text to search by

Responses

200
List of matching tokens retrieved successfully
application/json

400
Bad request - Invalid search parameters
application/json

500
Internal server error
application/json
Request Example forget/tokens/search
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/tokens/search");

let response = request.send().await?;

Test Request
(get /tokens/search)
Status:200
Status:400
Status:500
{
"data": [
{
"address": "So11111111111111111111111111111111111111112",
"decimals": 6,
"extensions": "{\n \"confidentialTransferFeeConfig\": {\n \"authority\": \"9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM\",\n \"harvestToMintEnabled\": true,\n \"withdrawWithheldAuthorityElgamalPubkey\": \"...\",\n \"withheldAmount\": \"...\"\n },\n \"confidentialTransferMint\": {\n \"auditorElgamalPubkey\": null,\n \"authority\": \"...\",\n \"autoApproveNewAccounts\": false\n },\n \"metadataPointer\": {\n \"authority\": \"...\",\n \"metadataAddress\": \"...\"\n },\n \"mintCloseAuthority\": {\n \"closeAuthority\": \"...\"\n },\n \"permanentDelegate\": {\n \"delegate\": \"...\"\n },\n \"tokenMetadata\": {\n \"additionalMetadata\": [],\n \"mint\": \"...\",\n \"name\": \"Name of Coin\",\n \"symbol\": \"Coin\",\n \"updateAuthority\": \"...\",\n \"uri\": \"https://example.com/image.json\"\n },\n \"transferFeeConfig\": {\n \"newerTransferFee\": {\n \"epoch\": 605,\n \"maximumFee\": 0,\n \"transferFeeBasisPoints\": 0\n },\n \"olderTransferFee\": {\n \"epoch\": 605,\n \"maximumFee\": 0,\n \"transferFeeBasisPoints\": 0\n },\n \"transferFeeConfigAuthority\": \"...\",\n \"withdrawWithheldAuthority\": \"...\",\n \"withheldAmount\": 0\n },\n \"transferHook\": {\n \"authority\": \"...\",\n \"programId\": null\n }\n }",
"freezeAuthority": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
"isInitialized": true,
"metadata": "{\n \"description\": \"Description about the token\",\n \"image\": \"https://example.com/image.png\",\n \"name\": \"Name of Token\",\n \"risk\": 2,\n \"symbol\": \"TKN\"\n }",
"mintAuthority": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
"priceUsdc": "0.9998715445029346189836299351",
"stats": "{\"24h\": {\"volume\": 1197647.1678154785}}",
"supply": "154400747126297",
"tags": "[\"confidentialTransferFeeConfig\", \"confidentialTransferMint\", \"metadataPointer\", \"mintCloseAuthority\", \"permanentDelegate\", \"tokenMetadata\", \"transferFeeConfig\", \"transferHook\"]",
"tokenProgram": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
"updatedAt": "2025-05-09T00:04:50.745163Z",
"updatedEpoch": 784
}
],
"meta": {
"next": null,
"previous": null
}
}

List of matching tokens retrieved successfully

Get token details by mint address​Copy link
Returns detailed information for a specific token identified by its mint address.

Path Parameters
mint_addressCopy link to mint_address
Type:string
required
Token mint address

Responses

200
Token details retrieved successfully
application/json
400Copy link to 400
Invalid mint address format

404Copy link to 404
Token not found

500Copy link to 500
Internal server error

Request Example forget/tokens/{mint_address}
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/tokens/{mint_address}");

let response = request.send().await?;

Test Request
(get /tokens/{mint_address})
Status:200
{
"data": {
"address": "So11111111111111111111111111111111111111112",
"decimals": 6,
"extensions": "{\n \"confidentialTransferFeeConfig\": {\n \"authority\": \"9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM\",\n \"harvestToMintEnabled\": true,\n \"withdrawWithheldAuthorityElgamalPubkey\": \"...\",\n \"withheldAmount\": \"...\"\n },\n \"confidentialTransferMint\": {\n \"auditorElgamalPubkey\": null,\n \"authority\": \"...\",\n \"autoApproveNewAccounts\": false\n },\n \"metadataPointer\": {\n \"authority\": \"...\",\n \"metadataAddress\": \"...\"\n },\n \"mintCloseAuthority\": {\n \"closeAuthority\": \"...\"\n },\n \"permanentDelegate\": {\n \"delegate\": \"...\"\n },\n \"tokenMetadata\": {\n \"additionalMetadata\": [],\n \"mint\": \"...\",\n \"name\": \"Name of Coin\",\n \"symbol\": \"Coin\",\n \"updateAuthority\": \"...\",\n \"uri\": \"https://example.com/image.json\"\n },\n \"transferFeeConfig\": {\n \"newerTransferFee\": {\n \"epoch\": 605,\n \"maximumFee\": 0,\n \"transferFeeBasisPoints\": 0\n },\n \"olderTransferFee\": {\n \"epoch\": 605,\n \"maximumFee\": 0,\n \"transferFeeBasisPoints\": 0\n },\n \"transferFeeConfigAuthority\": \"...\",\n \"withdrawWithheldAuthority\": \"...\",\n \"withheldAmount\": 0\n },\n \"transferHook\": {\n \"authority\": \"...\",\n \"programId\": null\n }\n }",
"freezeAuthority": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
"isInitialized": true,
"metadata": "{\n \"description\": \"Description about the token\",\n \"image\": \"https://example.com/image.png\",\n \"name\": \"Name of Token\",\n \"risk\": 2,\n \"symbol\": \"TKN\"\n }",
"mintAuthority": "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
"priceUsdc": "0.9998715445029346189836299351",
"stats": "{\"24h\": {\"volume\": 1197647.1678154785}}",
"supply": "154400747126297",
"tags": "[\"confidentialTransferFeeConfig\", \"confidentialTransferMint\", \"metadataPointer\", \"mintCloseAuthority\", \"permanentDelegate\", \"tokenMetadata\", \"transferFeeConfig\", \"transferHook\"]",
"tokenProgram": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
"updatedAt": "2025-05-09T00:04:50.745163Z",
"updatedEpoch": 784
},
"meta": {
"next": null,
"previous": null
}
}

Token details retrieved successfully

whirlpools ​Copy link
Whirlpool information endpoints

whirlpoolsOperations
get
/lock/{address}
get
/pools
get
/pools/search
get
/pools/{address}
Get locked liquidity for a given whirlpool​Copy link
This endpoint returns the locked liquidity for a given whirlpool.

Path Parameters
addressCopy link to address
Type:string
required
The Solana account address of the whirlpool.

Responses

200
Locked liquidity for the whirlpool
application/json
400Copy link to 400
Invalid address format

404Copy link to 404
Whirlpool not found

500Copy link to 500
Internal server error

Request Example forget/lock/{address}
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/lock/{address}");

let response = request.send().await?;

Test Request
(get /lock/{address})
Status:200
[
{
"lockedPercentage": "0.7",
"name": "Whirlpool-Lock"
}
]

Locked liquidity for the whirlpool

List whirlpools with optional filtering and pagination​Copy link
Query Parameters
sortByCopy link to sortBy
Type:string
enum
Field to sort whirlpools by

volume
volume5m
volume15m
volume30m
volume1h
Show all values
sortDirectionCopy link to sortDirection
Type:string
enum
Direction to sort whirlpools in

asc
desc
nextCopy link to next
Type:string
Cursor to start the next page of results

previousCopy link to previous
Type:string
Cursor to start the previous page of results

hasRewardsCopy link to hasRewards
Type:boolean
Filter whirlpools that must have rewards

hasWarningCopy link to hasWarning
Type:boolean
Filter whirlpools that must have a warning

hasAdaptiveFeeCopy link to hasAdaptiveFee
Type:boolean
Whether a whirlpool must be using adaptive fee

isWavebreakCopy link to isWavebreak
Type:boolean
Whether a whirlpool must have graduated from wavebreak

minTvlCopy link to minTvl
Type:string
Format:float
Minimum TVL of a whirlpool in USDC

minVolumeCopy link to minVolume
Type:string
Format:float
Minimum volume of a whirlpool in USDC

minLockedLiquidityPercentCopy link to minLockedLiquidityPercent
Type:string
Format:float
Minimum locked liquidity percentage of a whirlpool

sizeCopy link to size
Type:integer
Format:int32
min:  
0
Number of results to return

tokenCopy link to token
Type:array integer[]
Filter whirlpools containing this token

tokensBothOfCopy link to tokensBothOf
Type:array Pubkey[]
Filter whirlpools containing both specified tokens

Type:array integer[]
A Solana pubkey

addressesCopy link to addresses
Type:array Pubkey[]
Filter whirlpools with these addresses

Type:array integer[]
A Solana pubkey

statsCopy link to stats
Type:array TimePeriod[]
enum
Filter whirlpools with stats for these time periods

5m
15m
30m
1h
2h
Show all values
Type:TimePeriod
enum
Time period for the whirlpool stats

5m
15m
30m
1h
2h
Show all values
includeBlockedCopy link to includeBlocked
Type:boolean
Include blocked whirlpools if true

Responses

200
List of whirlpools
application/json
400Copy link to 400
Invalid query parameters

500Copy link to 500
Internal server error

Request Example forget/pools
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/pools");

let response = request.send().await?;

Test Request
(get /pools)
Status:200
{
"data": [
{
"address": "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE",
"feeGrowthGlobalA": "9554655121161208077",
"feeGrowthGlobalB": "1228285739014669796",
"feeRate": 400,
"liquidity": "748079040958583",
"protocolFeeOwedA": "167818699",
"protocolFeeOwedB": "13808875",
"protocolFeeRate": 1300,
"rewardLastUpdatedTimestamp": "2025-05-09T03:40:08Z",
"sqrtPrice": "7428739679004743247",
"tickCurrentIndex": -18192,
"tickSpacing": 4,
"tickSpacingSeed": "[4, 0]",
"tokenMintA": "So11111111111111111111111111111111111111112",
"tokenMintB": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
"tokenVaultA": [
0
],
"tokenVaultB": "2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP",
"updatedAt": "2025-05-09T03:40:22.210625Z",
"updatedSlot": 338786229,
"whirlpoolBump": "[255]",
"whirlpoolsConfig": "2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ",
"writeVersion": "1459165811157",
"adaptiveFee": {
"constants": {
"adaptiveFeeControlFactor": 0,
"decayPeriod": 0,
"filterPeriod": 0,
"majorSwapThresholdTicks": 0,
"maxVolatilityAccumulator": 0,
"reductionFactor": 0,
"tickGroupSize": 0
},
"currentRate": 0,
"maxRate": 0,
"variables": {
"lastMajorSwapTimestamp": "2026-02-24T19:41:02.288Z",
"lastReferenceUpdateTimestamp": "2026-02-24T19:41:02.288Z",
"tickGroupIndexReference": 1,
"volatilityAccumulator": 0,
"volatilityReference": 0
}
},
"adaptiveFeeEnabled": false,
"addressLookupTable": [
0
],
"feeTierIndex": 4,
"hasWarning": false,
"lockedLiquidityPercent": [
{
"lockedPercentage": "0.7",
"name": "Whirlpool-Lock"
}
],
"poolType": "splashpool",
"price": "162.17758715438083504000",
"rewards": [
{
"authority": "string",
"emissions_per_second_x64": "string",
"growth_global_x64": "string",
"mint": "string",
"vault": "string",
"active": false,
"emissionsPerSecond": "0"
}
],
"stats": {
"additionalProperty": {
"fees": "109195.458980424000",
"rewards": "0",
"volume": "272988647.45106000",
"yieldOverTvl": "0.00305562472380393000"
}
},
"tokenA": {
"address": "So11111111111111111111111111111111111111112",
"decimals": 6,
"imageUrl": "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png",
"name": "Solana",
"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
"symbol": "SOL",
"tags": "[\"confidentialTransferFeeConfig\", \"confidentialTransferMint\", \"metadataPointer\", \"mintCloseAuthority\", \"permanentDelegate\", \"tokenMetadata\", \"transferFeeConfig\", \"transferHook\"]"
},
"tokenB": {
"address": "So11111111111111111111111111111111111111112",
"decimals": 6,
"imageUrl": "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png",
"name": "Solana",
"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
"symbol": "SOL",
"tags": "[\"confidentialTransferFeeConfig\", \"confidentialTransferMint\", \"metadataPointer\", \"mintCloseAuthority\", \"permanentDelegate\", \"tokenMetadata\", \"transferFeeConfig\", \"transferHook\"]"
},
"tokenBalanceA": "105580027977071",
"tokenBalanceB": "18616923791931",
"tradeEnableTimestamp": "1970-01-01T00:00:00Z",
"tvlUsdc": "35735886.7176539739861592478846702700",
"yieldOverTvl": "0.00305562472377327000"
}
],
"meta": {
"next": null,
"previous": null
}
}

List of whirlpools

Search for whirlpools by token symbols or address​Copy link
This endpoint allows searching for whirlpools by:

Token symbols (e.g., "SOL USDC" to find pools with both tokens)
Single token symbol (e.g., "SOL" to find all pools containing SOL)
Whirlpool address (single address exact match)
The search is case-insensitive and supports partial matching for token symbols.

Query Parameters
qCopy link to q
Type:string
Query text to search by

nextCopy link to next
Type:string
Cursor to start the next page of results

sizeCopy link to size
Type:integer
Format:int32
min:  
0
Maximum number of results to return

sortByCopy link to sortBy
Type:string
enum
Field to sort by

volume
volume5m
volume15m
volume30m
volume1h
Show all values
sortDirectionCopy link to sortDirection
Type:string
enum
Sort direction

asc
desc
minTvlCopy link to minTvl
Type:string
Format:float
Minimum TVL of a whirlpool in USDC

minVolumeCopy link to minVolume
Type:string
Format:float
Minimum volume of a whirlpool in USDC

statsCopy link to stats
Type:array TimePeriod[]
enum
Filter whirlpools with stats for these time periods

5m
15m
30m
1h
2h
Show all values
Type:TimePeriod
enum
Time period for the whirlpool stats

5m
15m
30m
1h
2h
Show all values
userTokensCopy link to userTokens
Type:array Pubkey[]
Comma-separated list of token addresses owned by the user

Type:array integer[]
A Solana pubkey

hasRewardsCopy link to hasRewards
Type:boolean
Filter for pools with rewards

verifiedOnlyCopy link to verifiedOnly
Type:boolean
Filter for pools with verified tokens

hasLockedLiquidityCopy link to hasLockedLiquidity
Type:boolean
Filter for pools with locked liquidity

Responses

200
List of matching whirlpools
application/json
400Copy link to 400
Invalid query parameters

500Copy link to 500
Internal server error

Request Example forget/pools/search
Rust reqwest
let client = reqwest::Client::new();

let request = client.get("https://api.orca.so/v2/{chain}/pools/search");

let response = request.send().await?;

Test Request
(get /pools/search)
Status:200
{
"data": [
{
"address": "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE",
"feeGrowthGlobalA": "9554655121161208077",
"feeGrowthGlobalB": "1228285739014669796",
"feeRate": 400,
"liquidity": "748079040958583",
"protocolFeeOwedA": "167818699",
"protocolFeeOwedB": "13808875",
"protocolFeeRate": 1300,
"rewardLastUpdatedTimestamp": "2025-05-09T03:40:08Z",
"sqrtPrice": "7428739679004743247",
"tickCurrentIndex": -18192,
"tickSpacing": 4,
"tickSpacingSeed": "[4, 0]",
"tokenMintA": "So11111111111111111111111111111111111111112",
"tokenMintB": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
"tokenVaultA": [
0
],
"tokenVaultB": "2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP",
"updatedAt": "2025-05-09T03:40:22.210625Z",
"updatedSlot": 338786229,
"whirlpoolBump": "[255]",
"whirlpoolsConfig": "2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ",
"writeVersion": "1459165811157",
"adaptiveFee": {
"constants": {
"adaptiveFeeControlFactor": 0,
"decayPeriod": 0,
"filterPeriod": 0,
"majorSwapThresholdTicks": 0,
"maxVolatilityAccumulator": 0,
"reductionFactor": 0,
"tickGroupSize": 0
},
"currentRate": 0,
"maxRate": 0,
"variables": {
"lastMajorSwapTimestamp": "2026-02-24T19:41:02.288Z",
"lastReferenceUpdateTimestamp": "2026-02-24T19:41:02.288Z",
"tickGroupIndexReference": 1,
"volatilityAccumulator": 0,
"volatilityReference": 0
}
},
"adaptiveFeeEnabled": false,
"addressLookupTable": [
0
],
"feeTierIndex": 4,
"hasWarning": false,
"lockedLiquidityPercent": [
{
"lockedPercentage": "0.7",
"name": "Whirlpool-Lock"
}
],
"poolType": "splashpool",
"price": "162.17758715438083504000",
"rewards": [
{
"authority": "string",
"emissions_per_second_x64": "string",
"growth_global_x64": "string",
"mint": "string",
"vault": "string",
"active": false,
"emissionsPerSecond": "0"
}
],
"stats": {
"additionalProperty": {
"fees": "109195.458980424000",
"rewards": "0",
"volume": "272988647.45106000",
"yieldOverTvl": "0.00305562472380393000"
}
},
"tokenA": {
"address": "So11111111111111111111111111111111111111112",
"decimals": 6,
"imageUrl": "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png",
"name": "Solana",
"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
"symbol": "SOL",
"tags": "[\"confidentialTransferFeeConfig\", \"confidentialTransferMint\", \"metadataPointer\", \"mintCloseAuthority\", \"permanentDelegate\", \"tokenMetadata\", \"transferFeeConfig\", \"transferHook\"]"
},
"tokenB": {
"address": "So11111111111111111111111111111111111111112",
"decimals": 6,
"imageUrl": "https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png",
"name": "Solana",
"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
"symbol": "SOL",
"tags": "[\"confidentialTransferFeeConfig\", \"confidentialTransferMint\", \"metadataPointer\", \"mintCloseAuthority\", \"permanentDelegate\", \"tokenMetadata\", \"transferFeeConfig\", \"transferHook\"]"
},
"tokenBalanceA": "105580027977071",
"tokenBalanceB": "18616923791931",
"tradeEnableTimestamp": "1970-01-01T00:00:00Z",
"tvlUsdc": "35735886.7176539739861592478846702700",
"yieldOverTvl": "0.00305562472377327000"
}
],
"meta": {
"next": null,
"previous": null
}
}

List of matching whirlpools

Get whirlpool data by address​Copy link
Path Parameters
addressCopy link to address
Type:string
required
Whirlpool address

Query Parameters
sortByCopy link to sortBy
Type:string
enum
Field to sort whirlpools by

volume
volume5m
volume15m
volume30m
volume1h
Show all values
sortDirectionCopy link to sortDirection
Type:string
enum
Direction to sort whirlpools in

asc
desc
nextCopy link to next
Type:string
Cursor to start the next page of results

previousCopy link to previous
Type:string
Cursor to start the previous page of results

hasRewardsCopy link to hasRewards
Type:boolean
Filter whirlpools that must have rewards

hasWarningCopy link to hasWarning
Type:boolean
Filter whirlpools that must have a warning

hasAdaptiveFeeCopy link to hasAdaptiveFee
Type:boolean
Whether a whirlpool must be using adaptive fee

isWavebreakCopy link to isWavebreak
Type:boolean
Whether a whirlpool must have graduated from wavebreak

minTvlCopy link to minTvl
Type:string
Format:float
Minimum TVL of a whirlpool in USDC

minVolumeCopy link to minVolume
Type:string
Format:float
Minimum volume of a whirlpool in USDC

minLockedLiquidityPercentCopy link to minLockedLiquidityPercent
Type:string
Format:float
Minimum locked liquidity percentage of a whirlpool

sizeCopy link to size
Type:integer
Format:int32
min:  
0
Number of results to return

tokenCopy link to token
Type:array integer[]
Filter whirlpools containing this token

tokensBothOfCopy link to tokensBothOf
Type:array Pubkey[]
Filter whirlpools containing both specified tokens

Type:array integer[]
A Solana pubkey

addressesCopy link to addresses
Type:array Pubkey[]
Filter whirlpools with these addresses

Type:array integer[]
A Solana pubkey

statsCopy link to stats
Type:array TimePeriod[]
enum
Filter whirlpools with stats for these time periods

5m
15m
30m
1h
2h
Show all values
Type:TimePeriod
enum
Time period for the whirlpool stats

5m
15m
30m
1h
2h
Show all values
includeBlockedCopy link to includeBlocked
Type:boolean
Include blocked whirlpools if true

Responses

200
Whirlpool data retrieved successfully
application/json
400Copy link to 400
Invalid whirlpool address

404Copy link to 404
Whirlpool not found

500Copy link to 500
Internal server error
