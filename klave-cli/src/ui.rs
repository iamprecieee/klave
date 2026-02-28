use std::time::Duration;

use console::style;
use rand::random;

//   #16a085  → teal   (xterm 36)  — accent, step markers, success
//   #d97706  → amber  (xterm 172) — structural chrome, brand
//   #ecece0  → cream  (xterm 255) — text labels
//   #60a5fa  → blue   (xterm 75)  — info values, URLs

const TEAL: u8 = 36;
const AMBER: u8 = 172;
const CREAM: u8 = 255;
const BLUE: u8 = 75;

const BOX_WIDTH: usize = 46;

pub fn banner() {
    let art = r#"
 ██╗  ██╗██╗      █████╗ ██╗   ██╗███████╗
 ██║ ██╔╝██║     ██╔══██╗██║   ██║██╔════╝
 █████╔╝ ██║     ███████║██║   ██║█████╗
 ██╔═██╗ ██║     ██╔══██║╚██╗ ██╔╝██╔══╝
 ██║  ██╗███████╗██║  ██║ ╚████╔╝ ███████╗
 ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝  ╚═══╝  ╚══════╝"#;

    println!("{}", style(art).color256(TEAL).bold());
    println!(" {}", style(tagline()).color256(CREAM).dim());
    println!();
}

fn tagline() -> &'static str {
    const LINES: &[&str] = &[
        "Your keys are safe. Your assumptions are not.",
        "Policy-first autonomy for agents that never sleep.",
        "Gasless by default. Reckless by choice.",
        "Where AI agents get their own wallets.",
        "One keypair per agent. Zero trust required.",
        "Vaults don't sleep. Neither do your agents.",
        "Your agent's balance is not your problem.",
        "Ship agents, not anxiety.",
        "If it's on-chain, it's audited. No exceptions.",
        "Solana at machine speed. Guardrails at human speed.",
    ];
    LINES[random::<u8>() as usize % LINES.len()]
}

pub fn flow_start(cmd: &str) {
    println!(
        "{}  {}",
        style("┌").color256(AMBER),
        style(format!("klave {cmd}")).color256(CREAM).bold()
    );
}

pub fn flow_blank() {
    println!("{}", style("│").color256(AMBER));
}

pub fn flow_step(text: &str) {
    println!(
        "{}  {}",
        style("◇").color256(TEAL),
        style(text).color256(CREAM)
    );
}

pub fn flow_line(text: &str) {
    println!("{}  {}", style("│").color256(AMBER), text);
}

pub fn flow_done(text: &str) {
    println!(
        "{}  {} {}",
        style("│").color256(AMBER),
        style("◆").color256(TEAL),
        style(text).color256(CREAM)
    );
}

pub fn flow_end(text: &str) {
    println!(
        "{}  {}",
        style("└").color256(AMBER),
        style(text).color256(CREAM)
    );
}

pub fn flow_box(title: &str, rows: &[(&str, &str)]) {
    let content_widths: Vec<usize> = rows
        .iter()
        .map(|(key, value)| key.len() + 2 + value.len())
        .collect();
    let max_content_width = content_widths.iter().copied().max().unwrap_or(0);
    let inner = max_content_width.max(BOX_WIDTH - 4);

    // Title line: ◇  title ───────────╮
    let dash_count = inner.saturating_sub(title.len()).saturating_sub(1).max(1);
    println!(
        "{}  {} {}{}",
        style("◇").color256(TEAL),
        style(title).color256(CREAM).bold(),
        style("─".repeat(dash_count)).color256(AMBER),
        style("╮").color256(AMBER),
    );

    // Content rows
    for (key, value) in rows {
        let content = format!(
            "{}: {}",
            style(key).color256(CREAM).dim(),
            style(value).color256(BLUE)
        );
        // Raw length for padding (without ANSI codes)
        let raw_len = key.len() + 2 + value.len();
        let pad = if inner > raw_len { inner - raw_len } else { 0 };
        println!(
            "{}  {}{}{}",
            style("│").color256(AMBER),
            content,
            " ".repeat(pad),
            style("│").color256(AMBER),
        );
    }

    // Close: ├───────────────────────╯
    println!(
        "{}{}{}",
        style("├").color256(AMBER),
        style("─".repeat(inner + 2)).color256(AMBER),
        style("╯").color256(AMBER),
    );
}

pub fn make_spinner(message: &str) -> indicatif::ProgressBar {
    let pb = indicatif::ProgressBar::new_spinner();
    let template = format!(
        "{}  {} {{msg}}",
        style("│").color256(AMBER),
        style("⠋").color256(TEAL)
    );
    pb.set_style(
        indicatif::ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template(&template)
            .expect("valid template"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn info(text: &str) -> String {
    style(text).color256(BLUE).to_string()
}

pub fn brand(text: &str) -> String {
    style(text).color256(AMBER).bold().to_string()
}

pub fn dim(text: &str) -> String {
    style(text).color256(CREAM).dim().to_string()
}
