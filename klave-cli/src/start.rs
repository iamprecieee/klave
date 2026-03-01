use std::{env, fs, io, mem, os::unix::io::AsRawFd, process::Stdio, time::Duration};

use tokio::{process::Command, signal, time};

use crate::{ui, utils::project_root};

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

    ui::flow_start("start");
    ui::flow_blank();

    let profile_str = if release { "release" } else { "dev" };
    let kora_str = if with_kora { "enabled" } else { "disabled" };
    let dash_str = if dashboard { "enabled" } else { "disabled" };

    ui::flow_box(
        "Configuration",
        &[
            ("profile", profile_str),
            ("kora", kora_str),
            ("dashboard", dash_str),
        ],
    );
    ui::flow_blank();
    ui::flow_step("Compiling workspace...");

    let pb = ui::make_spinner("Assembling binaries...");

    let mut build_args = vec!["build", "--workspace", "--quiet"];
    if release {
        build_args.push("--release");
    }

    let build_status = Command::new("cargo")
        .args(&build_args)
        .current_dir(&root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    pb.finish_and_clear();

    if !build_status.success() {
        return Err("Build failed. Run `cargo build` manually to see errors.\n".into());
    }
    ui::flow_done("Build complete");

    ui::flow_blank();

    let target_dir = if release { "release" } else { "debug" };
    let server_bin = root.join(format!("target/{target_dir}/klave-server"));

    let mut children: Vec<(String, tokio::process::Child)> = Vec::new();
    let mut service_rows: Vec<(&str, String)> = Vec::new();

    // Kora (optional).
    if with_kora {
        let kora_toml_path = root.join("kora.toml");
        let price_source;
        if kora_toml_path.exists() {
            let mut kora_cfg = fs::read_to_string(&kora_toml_path)?;
            let has_jupiter_key = env::var("JUPITER_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .is_some();

            if has_jupiter_key {
                kora_cfg =
                    kora_cfg.replace("price_source = \"Mock\"", "price_source = \"Jupiter\"");
                price_source = "Jupiter";
            } else {
                kora_cfg =
                    kora_cfg.replace("price_source = \"Jupiter\"", "price_source = \"Mock\"");
                price_source = "Mock";
            }

            fs::write(&kora_toml_path, &kora_cfg)?;
        } else {
            price_source = "Mock";
        }

        let rpc_url = env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());

        let kora_log = fs::File::create(root.join("kora.log"))?;
        let kora_log_err = kora_log.try_clone()?;
        let kora_child = Command::new("kora")
            .args([
                "--rpc-url",
                &rpc_url,
                "rpc",
                "start",
                "--signers-config",
                "signers.toml",
            ])
            .current_dir(&root)
            .stdout(Stdio::from(kora_log))
            .stderr(Stdio::from(kora_log_err))
            .spawn();

        match kora_child {
            Ok(child) => {
                children.push(("kora".to_string(), child));
                service_rows.push(("kora", format!("ACTIVE ({})", price_source)));
            }
            Err(_) => {
                service_rows.push(("kora", "offline (binary not found)".to_string()));
            }
        }
    }

    // Dashboard (optional)
    if dashboard {
        let dashboard_dir = root.join("dashboard");
        if dashboard_dir.exists() {
            let dash_child = Command::new("python3")
                .args(["-m", "http.server", "8888", "--bind", "127.0.0.1"])
                .current_dir(&dashboard_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            if let Ok(child) = dash_child {
                children.push(("dashboard".to_string(), child));

                let _api_key = env::var("KLAVE_OPERATOR_API_KEY").unwrap_or_default();
                service_rows.push(("dashboard", "http://localhost:8888".to_string()));
            }
        }
    }

    // KLAVE server.
    let log_file = fs::File::create(root.join("klave.log"))?;
    let log_file_err = log_file.try_clone()?;
    let server_child = Command::new(&server_bin)
        .current_dir(&root)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()?;

    children.push(("server".to_string(), server_child));

    let port = env::var("KLAVE_PORT").unwrap_or_else(|_| "3000".to_string());
    service_rows.push(("server", format!("http://localhost:{port}")));
    service_rows.push(("logs", "klave.log".to_string()));

    let rows_ref: Vec<(&str, &str)> = service_rows.iter().map(|(k, v)| (*k, v.as_str())).collect();

    ui::flow_box("Services", &rows_ref);
    ui::flow_blank();
    ui::flow_step("Ready");
    ui::flow_line("Accepting autonomous agent connections.");
    ui::flow_line(&format!(
        "Press {} to terminate all services.",
        ui::brand("Ctrl+C")
    ));
    ui::flow_blank();

    // Suppress the ^C echo when the user presses Ctrl+C.
    #[cfg(unix)]
    let _termios_guard = suppress_ctrl_c_echo();

    signal::ctrl_c().await?;

    ui::flow_blank();
    ui::flow_step("Shutting down...");

    for (name, mut child) in children.into_iter().rev() {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                unsafe {
                    libc::kill(pid as i32, libc::SIGTERM);
                }
                time::sleep(Duration::from_millis(200)).await;
            }
        }

        let _ = child.kill().await;
        let _ = child.wait().await;

        ui::flow_line(&format!("{} detached", ui::dim(&name)));
    }

    ui::flow_blank();
    ui::flow_end("Session terminated. Goodbye!");

    println!();
    Ok(())
}

// ── Suppress ^C echo ────────────────────────────────────────────

#[cfg(unix)]
struct TermiosGuard {
    fd: i32,
    original: libc::termios,
}

#[cfg(unix)]
impl Drop for TermiosGuard {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.original) };
    }
}

#[cfg(unix)]
fn suppress_ctrl_c_echo() -> Option<TermiosGuard> {
    let fd = io::stdin().as_raw_fd();
    let mut termios: libc::termios = unsafe { mem::zeroed() };

    if unsafe { libc::tcgetattr(fd, &mut termios) } != 0 {
        return None;
    }
    let guard = TermiosGuard {
        fd,
        original: termios,
    };

    termios.c_lflag &= !libc::ECHOCTL;
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) };
    Some(guard)
}
