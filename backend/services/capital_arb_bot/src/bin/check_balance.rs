use anyhow::Result;
use ethers::prelude::*;
use std::env;
use std::sync::Arc;

const TOKENS: &[(&str, &str, u8)] = &[
    ("WMATIC", "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", 18),
    ("USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", 6),
    ("USDT", "0xc2132D05D31c914a87C6611C10748AEb04B58e8F", 6),
    ("WETH", "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", 18),
    ("DAI", "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063", 18),
];

abigen!(
    IERC20,
    r#"[
        function balanceOf(address owner) external view returns (uint256)
        function decimals() external view returns (uint8)
        function symbol() external view returns (string)
    ]"#
);

#[tokio::main]
async fn main() -> Result<()> {
    println!("💰 Polygon Wallet Balance Checker (Rust Edition)\n");

    // Load from .env
    dotenv::dotenv().ok();
    
    let private_key = env::var("PRIVATE_KEY")
        .expect("PRIVATE_KEY not set in .env file");
    let wallet = private_key.parse::<LocalWallet>()?
        .with_chain_id(137u64);
    
    let provider = Provider::<Http>::try_from("https://polygon-mainnet.public.blastapi.io")?;
    let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
    
    let address = client.address();
    
    println!("🔗 Connected to Polygon Mainnet");
    println!("📍 Wallet Address: {:?}", address);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Check MATIC balance
    let matic_balance = client.get_balance(address, None).await?;
    let matic_formatted = ethers::utils::format_ether(matic_balance);
    let matic_float: f64 = matic_formatted.parse().unwrap_or(0.0);
    println!("MATIC: {} MATIC (${:.2} @ $0.80)", matic_formatted, matic_float * 0.8);

    // Check token balances
    for (symbol, address_str, decimals) in TOKENS {
        let token_address: Address = address_str.parse()?;
        let token = IERC20::new(token_address, client.clone());
        
        match token.balance_of(address).call().await {
            Ok(balance) => {
                if balance > U256::zero() {
                    let formatted = ethers::utils::format_units(balance, *decimals)?;
                    println!("{}: {}", symbol, formatted);
                }
            }
            Err(_) => {} // Skip if can't read
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Check readiness
    if matic_float < 1.0 {
        println!("\n⚠️  Low MATIC balance! You need MATIC for gas fees.");
        println!("   Recommended: Send at least 5 MATIC for trading");
    } else if matic_float < 5.0 {
        println!("\n⚠️  MATIC balance is low. Consider adding more for active trading.");
    } else {
        println!("\n✅ MATIC balance sufficient for trading");
    }

    // Check USDC
    let usdc_address: Address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse()?;
    let usdc = IERC20::new(usdc_address, client.clone());
    let usdc_balance = usdc.balance_of(address).call().await?;
    let usdc_formatted = ethers::utils::format_units(usdc_balance, 6)?;
    let usdc_float: f64 = usdc_formatted.parse().unwrap_or(0.0);

    if usdc_float == 0.0 {
        println!("\n📌 To start trading:");
        println!("   1. Send USDC from Coinbase to: {:?}", address);
        println!("   2. Make sure to select \"Polygon\" network (not Ethereum!)");
        println!("   3. Start with a small test amount (e.g., $10)");
    } else {
        println!("\n✅ Ready to trade with ${:.2} USDC", usdc_float);
    }

    println!("\n🔍 View on Polygonscan:");
    println!("   https://polygonscan.com/address/{:?}", address);

    Ok(())
}