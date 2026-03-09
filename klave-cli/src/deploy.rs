use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use regex::Regex;

use crate::{ui, utils::project_root};

pub fn run(cluster: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let anchor_dir = root.join("klave-anchor");

    if !anchor_dir.exists() {
        return Err("Execution failed: klave-anchor/ treasury directory not found.".into());
    }

    ui::flow_start("deploy");
    ui::flow_blank();
    ui::flow_box(
        "Target",
        &[("cluster", cluster), ("program", "klave-anchor")],
    );
    ui::flow_blank();

    ui::flow_step("Checking account balance...");

    let address_out = Command::new("solana").arg("address").output()?;
    let address = String::from_utf8_lossy(&address_out.stdout)
        .trim()
        .to_string();

    let balance_out = Command::new("solana")
        .args(["balance", "--url", cluster])
        .output()?;

    if balance_out.status.success() {
        let balance_str = String::from_utf8_lossy(&balance_out.stdout);
        if let Some(balance_val_str) = balance_str.split_whitespace().next() {
            if let Ok(balance) = balance_val_str.parse::<f64>() {
                if balance < 1.5 {
                    ui::flow_line(&format!(
                        "{} Address: {}",
                        ui::brand("!"),
                        ui::info(&address)
                    ));
                    ui::flow_line(&format!(
                        "{} Current balance: {} SOL (Minimum ~1.5 SOL required)",
                        ui::brand("!"),
                        ui::info(&balance.to_string())
                    ));
                    ui::flow_line("   Please fund your account manually to continue.");
                    return Err("Insufficient funds for deployment.".into());
                }
            }
        }
    }

    let sync_spinner = ui::make_spinner("Synchronizing program keys...");
    let sync_output = Command::new("anchor")
        .args(["keys", "sync"])
        .current_dir(&anchor_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    sync_spinner.finish_and_clear();

    if !sync_output.status.success() {
        let stderr = String::from_utf8_lossy(&sync_output.stderr);
        eprintln!("{}", stderr);
        return Err("Failed to synchronize program keys.".into());
    }
    ui::flow_done("Program keys synchronized");

    let build_spinner = ui::make_spinner("Compiling Anchor program...");
    let build_output = Command::new("anchor")
        .arg("build")
        .current_dir(&anchor_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    build_spinner.finish_and_clear();

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        eprintln!("{}", stderr);
        return Err("Anchor compilation reported critical errors.".into());
    }

    ui::flow_done("Bytecode finalized");
    ui::flow_blank();

    let deploy_spinner = ui::make_spinner(&format!("Migrating to {}...", ui::info(cluster)));
    let deploy_output = Command::new("anchor")
        .args(["deploy", "--provider.cluster", cluster])
        .current_dir(&anchor_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    deploy_spinner.finish_and_clear();

    if !deploy_output.status.success() {
        let stderr = String::from_utf8_lossy(&deploy_output.stderr);
        eprintln!("{}", stderr);
        return Err("Program migration rejected by the cluster.".into());
    }

    ui::flow_done("Program deployed");
    ui::flow_blank();

    ui::flow_step("Propagating Program ID...");
    if let Err(e) = propagate_program_id(&root, cluster) {
        ui::flow_line(&format!(
            "{} Warning: ID propagation failed: {}",
            ui::brand("!"),
            e
        ));
    } else {
        ui::flow_done("Program ID propagated to server, SDK, and docs");
    }
    ui::flow_blank();

    ui::flow_end(&format!(
        "Treasury {} on {}.",
        ui::brand("live"),
        ui::info(cluster)
    ));

    println!();

    Ok(())
}

fn propagate_program_id(root: &Path, _cluster: &str) -> anyhow::Result<()> {
    let anchor_toml_path = root.join("klave-anchor").join("Anchor.toml");
    let anchor_toml = fs::read_to_string(&anchor_toml_path)?;

    // Extract new ID from [programs.cluster] or [programs.devnet]
    let re_id = Regex::new(&format!(r#"klave_anchor\s*=\s*"([^"]+)"#))?;
    let new_id = re_id
        .captures(&anchor_toml)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow::anyhow!("Could not find klave_anchor ID in Anchor.toml"))?;

    // Find old ID from klave-core/src/agent/model.rs
    let model_rs_path = root
        .join("klave-core")
        .join("src")
        .join("agent")
        .join("model.rs");
    let model_rs = fs::read_to_string(&model_rs_path)?;
    let re_old_id = Regex::new(r#"pub const TREASURY_PROGRAM_ID: &str = "([^"]+)";"#)?;
    let old_id = re_old_id
        .captures(&model_rs)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow::anyhow!("Could not find TREASURY_PROGRAM_ID in model.rs"))?;

    if old_id == new_id {
        return Ok(());
    }

    // Update all files
    let files_to_update = vec![
        root.join("klave-core")
            .join("src")
            .join("agent")
            .join("model.rs"),
        root.join("sdk").join("klave").join("models.py"),
        root.join("kora.example.toml"),
        root.join("docs").join("README.md"),
        root.join("docs").join("SKILLS.md"),
        root.join("docs").join("REGISTER.md"),
        root.join("docs").join("HEARTBEAT.md"),
    ];

    for path in files_to_update {
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let updated = content.replace(old_id, new_id);
            fs::write(path, updated)?;
        }
    }

    Ok(())
}
