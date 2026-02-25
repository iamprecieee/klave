use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::signal;

use crate::utils::set_key;

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

fn print_status(service: &str, msg: &str, color: &str) {
    let code = match color {
        "green" => "\x1b[32m",
        "yellow" => "\x1b[33m",
        "red" => "\x1b[31m",
        "cyan" => "\x1b[36m",
        _ => "\x1b[0m",
    };
    eprintln!("  {code}[{service}]\x1b[0m {msg}");
}

pub async fn run(
    with_kora: bool,
    dashboard: bool,
    release: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = project_root();
    let env_path = root.join(".env");

    if !env_path.exists() {
        return Err("No .env found. Run `klave init` first.".into());
    }

    dotenvy::from_path(&env_path)?;

    println!("\x1b[1mKLAVE start\x1b[0m");
    println!();

    // ── Build workspace ──────────────────────────────────────────
    let profile = if release { "release" } else { "dev" };
    print_status(
        "build",
        &format!("compiling workspace ({profile})..."),
        "cyan",
    );

    let mut build_args = vec!["build", "--workspace"];
    if release {
        build_args.push("--release");
    }

    let build_status = Command::new("cargo")
        .args(&build_args)
        .current_dir(&root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    if !build_status.success() {
        return Err("klave-server build failed".into());
    }
    print_status("build", "done", "green");

    // Resolve binary path.
    let target_dir = if release { "release" } else { "debug" };
    let server_bin = root.join(format!("target/{target_dir}/klave-server"));

    if !server_bin.exists() {
        return Err(format!("binary not found at {}", server_bin.display()).into());
    }

    // ── Spawn children ──────────────────────────────────────────
    let mut children: Vec<(String, tokio::process::Child)> = Vec::new();

    // Kora (optional).
    if with_kora {
        print_status("kora", "starting...", "cyan");

        let kora_toml_path = root.join("kora.toml");

        // Patch kora.toml price_source based on JUPITER_API_KEY availability.
        if kora_toml_path.exists() {
            let mut kora_cfg = std::fs::read_to_string(&kora_toml_path)?;
            let has_jupiter_key = std::env::var("JUPITER_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .is_some();

            if has_jupiter_key {
                if kora_cfg.contains("price_source = \"Mock\"") {
                    kora_cfg =
                        kora_cfg.replace("price_source = \"Mock\"", "price_source = \"Jupiter\"");
                    std::fs::write(&kora_toml_path, &kora_cfg)?;
                    print_status(
                        "kora",
                        "using Jupiter pricing (JUPITER_API_KEY set)",
                        "green",
                    );
                }
            } else {
                if kora_cfg.contains("price_source = \"Jupiter\"") {
                    kora_cfg =
                        kora_cfg.replace("price_source = \"Jupiter\"", "price_source = \"Mock\"");
                    std::fs::write(&kora_toml_path, &kora_cfg)?;
                }
                print_status(
                    "kora",
                    "using Mock pricing (set JUPITER_API_KEY for real prices)",
                    "yellow",
                );
            }
        }

        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
        let kora_api_key = std::env::var("KORA_API_KEY").ok().filter(|k| !k.is_empty());

        if kora_toml_path.exists() {
            let mut kora_cfg = std::fs::read_to_string(&kora_toml_path)?;
            if let Some(key) = &kora_api_key {
                let mut lines = kora_cfg
                    .lines()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>();
                set_key(&mut lines, "api_key = ", &format!("\"{}\"", key));
                kora_cfg = lines.join("\n");
                std::fs::write(&kora_toml_path, &kora_cfg)?;
            }
        }

        let kora_args = vec![
            "--rpc-url".to_string(),
            rpc_url,
            "rpc".to_string(),
            "start".to_string(),
            "--signers-config".to_string(),
            "signers.toml".to_string(),
        ];

        let kora_child = Command::new("kora")
            .args(&kora_args)
            .current_dir(&root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn();

        match kora_child {
            Ok(child) => {
                children.push(("kora".to_string(), child));
                print_status("kora", "running", "green");
            }
            Err(e) => {
                print_status("kora", &format!("failed to start: {e}"), "red");
                eprintln!("         Is `kora` installed? Install with: cargo install kora");
                eprintln!("         Continuing without Kora — server will use direct RPC.");
            }
        }
    }

    // Dashboard (optional) — simple Python http.server on port 8888.
    if dashboard {
        let dashboard_dir = root.join("dashboard");
        if dashboard_dir.exists() {
            print_status("dashboard", "serving on http://localhost:8888", "cyan");
            let dash_child = Command::new("python3")
                .args(["-m", "http.server", "8888", "--bind", "127.0.0.1"])
                .current_dir(&dashboard_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            match dash_child {
                Ok(child) => {
                    children.push(("dashboard".to_string(), child));
                    print_status("dashboard", "http://localhost:8888", "green");
                }
                Err(e) => {
                    print_status("dashboard", &format!("failed: {e}"), "red");
                }
            }
        } else {
            print_status(
                "dashboard",
                "dashboard/ directory not found, skipping",
                "yellow",
            );
        }
    }

    // KLAVE server.
    print_status("server", "starting...", "cyan");
    let server_child = Command::new(&server_bin)
        .current_dir(&root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    children.push(("server".to_string(), server_child));

    let port = std::env::var("KLAVE_PORT").unwrap_or_else(|_| "3000".to_string());
    print_status("server", &format!("http://localhost:{port}"), "green");

    println!();
    println!("  Press \x1b[1mCtrl+C\x1b[0m to stop all services.");
    println!();

    // ── Wait for Ctrl+C ─────────────────────────────────────────
    signal::ctrl_c().await?;

    println!();
    print_status("shutdown", "stopping services...", "yellow");

    for (name, mut child) in children.into_iter().rev() {
        // Send SIGTERM on Unix, kill on Windows.
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        let _ = child.kill().await;
        let _ = child.wait().await;
        print_status(&name, "stopped", "green");
    }

    print_status("shutdown", "all services stopped", "green");
    Ok(())
}
