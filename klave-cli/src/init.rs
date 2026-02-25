use rand::random;
use std::fs;
use std::path::PathBuf;

use crate::utils::set_key;

fn project_root() -> PathBuf {
    let mut dir = std::env::current_dir().expect("cannot read cwd");
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            let content = fs::read_to_string(&candidate).unwrap_or_default();
            if content.contains("[workspace]") {
                return dir;
            }
        }
        if !dir.pop() {
            return std::env::current_dir().expect("cannot read cwd");
        }
    }
}

fn random_hex(bytes: usize) -> String {
    (0..bytes)
        .map(|_| format!("{:02x}", random::<u8>()))
        .collect()
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let example_path = root.join(".env.example");
    let env_path = root.join(".env");

    println!("\x1b[1mKLAVE init\x1b[0m");
    println!();

    if env_path.exists() {
        println!("  \x1b[33m[skip]\x1b[0m .env already exists");
    } else if example_path.exists() {
        fs::copy(&example_path, &env_path)?;
        println!("  \x1b[32m[done]\x1b[0m created .env from .env.example");
    } else {
        return Err(".env.example not found — are you in the KLAVE project root?".into());
    }

    let content = fs::read_to_string(&env_path)?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    // KLAVE_ENCRYPTION_KEY — 32 bytes hex (AES-256-GCM)
    let enc_key = random_hex(32);
    set_key(&mut lines, "KLAVE_ENCRYPTION_KEY=", &enc_key);
    println!("  \x1b[32m[done]\x1b[0m generated KLAVE_ENCRYPTION_KEY");

    // KLAVE_API_KEY — 16 bytes hex
    let api_key = random_hex(16);
    set_key(&mut lines, "KLAVE_API_KEY=", &api_key);
    println!("  \x1b[32m[done]\x1b[0m generated KLAVE_API_KEY");

    // KORA_PRIVATE_KEY + KORA_PUBKEY — Ed25519 keypair via solana-sdk
    {
        use solana_sdk::signer::Signer;
        let keypair = solana_sdk::signer::keypair::Keypair::new();
        set_key(&mut lines, "KORA_PRIVATE_KEY=", &keypair.to_base58_string());
        set_key(&mut lines, "KORA_PUBKEY=", &keypair.pubkey().to_string());
        println!("  \x1b[32m[done]\x1b[0m generated KORA_PRIVATE_KEY and KORA_PUBKEY");
    }

    // KORA_API_KEY — 16 bytes hex
    let kora_key = random_hex(16);
    set_key(&mut lines, "KORA_API_KEY=", &kora_key);
    println!("  \x1b[32m[done]\x1b[0m generated KORA_API_KEY");

    let content = lines.join("\n");
    fs::write(&env_path, &content)?;

    println!();
    println!("  .env written to: {}", env_path.display());
    println!();
    println!("  \x1b[1mNext:\x1b[0m klave start");
    println!();

    Ok(())
}
