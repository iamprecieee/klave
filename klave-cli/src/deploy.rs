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
    ui::flow_step("Compiling Anchor program...");

    let build_status = Command::new("anchor")
        .arg("build")
        .current_dir(&anchor_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !build_status.success() {
        return Err("Anchor compilation reported critical errors.".into());
    }

    ui::flow_done("Bytecode finalized");
    ui::flow_blank();
    ui::flow_step(&format!("Migrating to {}...", ui::info(cluster)));

    let deploy_status = Command::new("anchor")
        .args(["deploy", "--provider.cluster", cluster])
        .current_dir(&anchor_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !deploy_status.success() {
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
