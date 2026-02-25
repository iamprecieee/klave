use std::path::PathBuf;
use std::process::{Command, Stdio};

fn project_root() -> PathBuf {
    let mut dir = std::env::current_dir().expect("cannot read cwd");
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            let content = std::fs::read_to_string(&candidate).unwrap_or_default();
            if content.contains("[workspace]") {
                return dir;
            }
        }
        if !dir.pop() {
            return std::env::current_dir().expect("cannot read cwd");
        }
    }
}

fn print_status(stage: &str, msg: &str, color: &str) {
    let code = match color {
        "green" => "\x1b[32m",
        "yellow" => "\x1b[33m",
        "red" => "\x1b[31m",
        "cyan" => "\x1b[36m",
        _ => "\x1b[0m",
    };
    eprintln!("  {code}[{stage}]\x1b[0m {msg}");
}

pub fn run(cluster: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let anchor_dir = root.join("klave-anchor");

    if !anchor_dir.exists() {
        return Err("klave-anchor/ directory not found".into());
    }

    println!("\x1b[1mKLAVE deploy\x1b[0m (cluster: {cluster})");
    println!();

    // ── Anchor build ────────────────────────────────────────────
    print_status("build", "running anchor build...", "cyan");

    let build_status = Command::new("anchor")
        .arg("build")
        .current_dir(&anchor_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !build_status.success() {
        return Err("anchor build failed".into());
    }
    print_status("build", "done", "green");

    // ── Anchor deploy ───────────────────────────────────────────
    print_status("deploy", &format!("deploying to {cluster}..."), "cyan");

    let deploy_status = Command::new("anchor")
        .args(["deploy", "--provider.cluster", cluster])
        .current_dir(&anchor_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !deploy_status.success() {
        return Err("anchor deploy failed".into());
    }

    print_status("deploy", "program deployed successfully", "green");
    println!();

    Ok(())
}
