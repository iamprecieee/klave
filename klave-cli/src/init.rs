use std::fs;

use rand::random;

use crate::ui;
use crate::utils::{project_root, set_key};

fn random_hex(bytes: usize) -> String {
    (0..bytes)
        .map(|_| format!("{:02x}", random::<u8>()))
        .collect()
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let example_path = root.join(".env.example");
    let env_path = root.join(".env");

    ui::flow_start("init");
    ui::flow_blank();

    ui::flow_box("Project detected", &[("root", &root.display().to_string())]);

    ui::flow_blank();

    ui::flow_step("Environment mapping");
    if env_path.exists() {
        ui::flow_line(&format!(
            "Mapped {} (retaining existing keys)",
            ui::dim(".env")
        ));
    } else if example_path.exists() {
        fs::copy(&example_path, &env_path)?;
        ui::flow_line(&format!("Created {} from template", ui::info(".env")));
    } else {
        return Err("Configuration template (.env.example) not found.".into());
    }

    let content = fs::read_to_string(&env_path)?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    ui::flow_blank();

    ui::flow_step("Identity cryptography");
    {
        use solana_sdk::signer::Signer;
        let keypair = solana_sdk::signer::keypair::Keypair::new();
        set_key(&mut lines, "KORA_PRIVATE_KEY=", &keypair.to_base58_string());
        set_key(&mut lines, "KORA_PUBKEY=", &keypair.pubkey().to_string());
        ui::flow_line(&format!(
            "Generated {} gateway credentials",
            ui::info("KORA")
        ));
        ui::flow_line(&format!(
            "Identity: {}",
            ui::dim(&keypair.pubkey().to_string())
        ));
    }

    ui::flow_blank();

    ui::flow_step("Secrets hardening");

    let enc_key = random_hex(32);
    set_key(&mut lines, "KLAVE_ENCRYPTION_KEY=", &enc_key);
    ui::flow_line(&format!("Locked {}", ui::brand("KLAVE_ENCRYPTION_KEY")));

    let api_key = random_hex(16);
    set_key(&mut lines, "KLAVE_API_KEY=", &api_key);
    ui::flow_line(&format!("Locked {}", ui::brand("KLAVE_API_KEY")));

    let operator_key = random_hex(16);
    set_key(&mut lines, "KLAVE_OPERATOR_API_KEY=", &operator_key);
    ui::flow_line(&format!("Locked {}", ui::brand("KLAVE_OPERATOR_API_KEY")));

    let kora_key = random_hex(16);
    set_key(&mut lines, "KORA_API_KEY=", &kora_key);
    ui::flow_line(&format!("Locked {}", ui::brand("KORA_API_KEY")));

    let content = lines.join("\n");
    fs::write(&env_path, &content)?;

    ui::flow_blank();
    ui::flow_end(&format!(
        "Provisioning complete. Next: {}",
        ui::info("klave start")
    ));
    println!();

    Ok(())
}
