use std::process::{Command, Stdio};

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
    ui::flow_end(&format!(
        "Treasury {} on {}.",
        ui::brand("live"),
        ui::info(cluster)
    ));

    println!();

    Ok(())
}
