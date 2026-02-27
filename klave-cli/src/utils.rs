use std::path::PathBuf;

pub fn project_root() -> PathBuf {
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

pub fn set_key(lines: &mut [String], prefix: &str, value: &str) {
    for line in lines.iter_mut() {
        if line.starts_with(prefix) {
            *line = format!("{prefix}{value}");
            return;
        }
    }
}
