pub fn set_key(lines: &mut [String], prefix: &str, value: &str) {
    for line in lines.iter_mut() {
        if line.starts_with(prefix) {
            *line = format!("{prefix}{value}");
            return;
        }
    }
}
