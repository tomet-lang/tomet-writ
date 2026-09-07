pub fn read(p: &str) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}
